import assert from "node:assert/strict";
import { test } from "node:test";
import { createOnlineServer } from "../src/online/http.ts";
import { clock, botToken } from "./online-fixtures.ts";
import { gameFixture } from "./online-game-fixtures.ts";
test("verified players have independent quotas and only exact trusted peers can supply client IP", async (t) => {
    const f = await gameFixture();
    t.after(() => f.close());
    const origin = "https://game.example";
    async function serve(trustedProxyIps: string[] = [], max = 2) {
        const server = createOnlineServer({ store: f.store, game: f.game, botToken, origin, now: () => clock, trustedProxyIps, rateLimit: { max, authMax: 2, windowMs: 60000, playerMax: 2 } });
        await new Promise<void>(r => server.listen(0, "127.0.0.1", r));
        t.after(() => new Promise<void>(r => { server.closeAllConnections(); server.close(() => r()); }));
        const addr = server.address();
        assert.ok(addr && typeof addr !== "string");
        return `http://127.0.0.1:${addr.port}/api/online`;
    }
    const credentials = [];
    for (const p of f.accounts.slice(0, 2))
        credentials.push("cd_online_session=" + (await f.store.login(p.telegram, clock)).token);
    const base = await serve();
    const me = (cookie: string) => fetch(base + "/activity", { headers: { cookie } });
    assert.equal((await me(credentials[0]!)).status, 200);
    assert.equal((await me(credentials[0]!)).status, 200);
    assert.equal((await me(credentials[0]!)).status, 429);
    assert.equal((await me(credentials[1]!)).status, 200);
    assert.equal((await me(credentials[1]!)).status, 200);
    const auth = (url: string, ip: string) => fetch(url + "/auth/telegram", { method: "POST", headers: { origin, "content-type": "application/json", "x-crowned-client-ip": ip, "x-forwarded-for": ip }, body: '{"initData":"invalid"}' });
    const untrusted = await serve([], 20);
    assert.equal((await auth(untrusted, "203.0.113.1")).status, 401);
    assert.equal((await auth(untrusted, "203.0.113.2")).status, 401);
    assert.equal((await auth(untrusted, "203.0.113.3")).status, 429);
    const trusted = await serve(["127.0.0.1"], 20);
    assert.equal((await auth(trusted, "203.0.113.1")).status, 401);
    assert.equal((await auth(trusted, "203.0.113.1")).status, 401);
    assert.equal((await auth(trusted, "203.0.113.1")).status, 429);
    assert.equal((await auth(trusted, "203.0.113.2")).status, 401);
    assert.throws(() => createOnlineServer({ store: f.store, botToken, origin, trustedProxyIps: ["127.0.0.0/8"] }));
});
