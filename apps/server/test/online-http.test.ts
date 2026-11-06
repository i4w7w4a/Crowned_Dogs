import assert from "node:assert/strict";
import { request } from "node:http";
import { test } from "node:test";
import { botToken, clock, embeddedDatabase, initData } from "./online-fixtures.ts";
import { createOnlineServer } from "../src/online/http.ts";
import { OnlineStore, migrate } from "../src/online/storage.ts";

test("HTTP serves only authenticated server profiles and protects browser mutations", async t => {
  const fixture = await embeddedDatabase();
  t.after(() => fixture.close());
  await migrate(fixture.database);
  let now = clock;
  const store = new OnlineStore(fixture.database);
  const origin = "https://game.example";
  const server = createOnlineServer({ store, botToken, origin, production: true, now: () => now });
  await new Promise<void>(resolve => server.listen(0, "127.0.0.1", resolve));
  t.after(() => new Promise<void>(resolve => { server.closeAllConnections(); server.close(() => resolve()); }));
  const address = server.address();
  assert.ok(address && typeof address !== "string");
  const url = `http://127.0.0.1:${address.port}/api/online`;
  const post = (route: string, body: unknown, extra: Record<string, string> = {}) => fetch(url + route, {
    method: "POST", headers: { origin, "content-type": "application/json", ...extra }, body: JSON.stringify(body),
  });
  assert.equal(server.requestTimeout, 10_000);
  assert.equal(server.headersTimeout, 10_000);
  assert.equal((await fetch(url + "/me")).status, 401);
  const healthy = await fetch(url + "/health");
  assert.deepEqual(await healthy.json(), { ok: true });
  assert.equal((await post("/auth/telegram", { initData: initData() }, { origin: "https://evil.example" })).status, 403);
  assert.equal((await fetch(url + "/auth/telegram", { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify({ initData: initData() }) })).status, 403);
  assert.equal((await post("/auth/telegram", { initData: initData(), level: 4, xp: 999, tag: "K-999-999" })).status, 400);
  assert.equal((await post("/auth/telegram", { initDataUnsafe: { user: { id: 123 } } })).status, 400);
  assert.equal((await post("/auth/telegram", {})).status, 400);
  assert.equal((await post("/auth/telegram", { initData: "" })).status, 401);
  assert.equal((await post("/auth/telegram", { initData: initData({}, "wrong") })).status, 401);
  for (const raw of ["{", "null", "[]", '"text"']) {
    assert.equal((await fetch(url + "/auth/telegram", { method: "POST", headers: { origin, "content-type": "application/json" }, body: raw })).status, 400);
  }
  assert.equal((await post("/auth/telegram", { initData: "x".repeat(16_384) })).status, 413);
  const chunkedStatus = await new Promise<number>((resolve, reject) => {
    const upload = request(url + "/auth/telegram", { method: "POST", headers: { origin, "content-type": "application/json" } }, result => {
      result.resume();
      resolve(result.statusCode!);
    });
    upload.on("error", reject);
    upload.write('{"initData":"');
    upload.write("x".repeat(17_000));
    upload.end('"}');
  });
  assert.equal(chunkedStatus, 413);
  assert.equal((await post("/auth/telegram", { initData: initData() }, { "content-type": "text/plain" })).status, 415);
  const login = await post("/auth/telegram", { initData: initData() });
  assert.equal(login.status, 200);
  const cookie = login.headers.get("set-cookie")!;
  assert.match(cookie, /HttpOnly/);
  assert.match(cookie, /SameSite=Lax/);
  assert.match(cookie, /Secure/);
  assert.match(cookie, /Path=\/api\/online(?:;|$)/);
  assert.match(cookie, /Max-Age=28800/);
  assert.equal(login.headers.get("access-control-allow-origin"), null);
  assert.equal(login.headers.get("cache-control"), "no-store");
  const payload = await login.json() as any;
  assert.equal(payload.maxLevel, 4);
  assert.equal(payload.account.profile.level, 1);
  assert.equal(payload.account.profile.xp, 0);
  assert.equal(payload.token, undefined);
  assert.equal(payload.account.token_hash, undefined);
  const credentials = { cookie: cookie.split(";")[0]! };
  const me = await fetch(url + "/me", { headers: credentials });
  assert.deepEqual(await me.json(), payload);
  const repeat = await post("/auth/telegram", { initData: initData({ user: JSON.stringify({ id: 123, first_name: "Другое имя" }) }) });
  assert.equal(((await repeat.json()) as any).account.id, payload.account.id);
  assert.equal(((await (await fetch(url + "/me", { headers: credentials })).json()) as any).account.telegram.firstName, "Другое имя");
  assert.equal((await post("/auth/logout", {}, { ...credentials, origin: "https://evil.example" })).status, 403);
  assert.equal((await post("/auth/logout", { xp: 500 }, credentials)).status, 400);
  const logout = await post("/auth/logout", {}, credentials);
  assert.equal(logout.status, 204);
  assert.match(logout.headers.get("set-cookie")!, /Max-Age=0/);
  assert.equal((await fetch(url + "/me", { headers: credentials })).status, 401);
  now += 8 * 60 * 60 * 1000;
  assert.equal((await fetch(url + "/me", { headers: { cookie: repeat.headers.get("set-cookie")!.split(";")[0]! } })).status, 401);
  assert.equal((await fetch(url + "/unknown")).status, 404);
  assert.equal((await fetch(url + "/me", { method: "PUT" })).status, 405);
  await fixture.database.query("DROP TABLE online_schema_migrations");
  const unavailable = await fetch(url + "/health");
  assert.equal(unavailable.status, 503);
  assert.deepEqual(await unavailable.json(), { error: "unavailable" });
});

test("HTTP mount controls routing and cookie path, rejects unsafe config and limits requests", async t => {
  const fixture = await embeddedDatabase();
  t.after(() => fixture.close());
  await migrate(fixture.database);
  const opts = { store: new OnlineStore(fixture.database), botToken, origin: "https://game.example", now: () => clock };
  for (const basePath of ["", "/", "api/online", "//api", "/api/", "/api/../x", "/api?x", "/api;Secure", "/api%2fx", "/api\\x"]) {
    assert.throws(() => createOnlineServer({ ...opts, basePath }), /base path/i);
  }
  assert.throws(() => createOnlineServer({ ...opts, origin: "*" }), /origin/i);
  assert.throws(() => createOnlineServer({ ...opts, origin: "https://game.example/path" }), /origin/i);
  assert.throws(() => createOnlineServer({ ...opts, botToken: "" }), /token/i);
  const server = createOnlineServer({ ...opts, basePath: "/r2/api/online", rateLimit: { max: 2, windowMs: 60_000, authMax: 2 } });
  await new Promise<void>(resolve => server.listen(0, "127.0.0.1", resolve));
  t.after(() => new Promise<void>(resolve => { server.closeAllConnections(); server.close(() => resolve()); }));
  const address = server.address();
  assert.ok(address && typeof address !== "string");
  const url = `http://127.0.0.1:${address.port}`;
  const login = await fetch(url + "/r2/api/online/auth/telegram", { method: "POST", headers: { origin: opts.origin, "content-type": "application/json" }, body: JSON.stringify({ initData: initData() }) });
  assert.equal(login.status, 200);
  assert.match(login.headers.get("set-cookie")!, /Path=\/r2\/api\/online;/);
  assert.doesNotMatch(login.headers.get("set-cookie")!, /Secure/);
  assert.equal((await fetch(url + "/api/online/health")).status, 404);
  assert.equal((await fetch(url + "/r2/api/online/health")).status, 429);
});
