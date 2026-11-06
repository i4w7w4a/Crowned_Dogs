import assert from "node:assert/strict";
import { execFile, spawn } from "node:child_process";
import { mkdtemp, readFile, realpath, rm, writeFile } from "node:fs/promises";
import { createServer } from "node:http";
import type { AddressInfo } from "node:net";
import { tmpdir } from "node:os";
import { join, resolve, sep } from "node:path";
import { test } from "node:test";
import { promisify } from "node:util";

const run = promisify(execFile);
const binary = process.env.CADDY_BIN;
for (const fixture of [
  { name: "trusted gateway", peer: "127.0.0.1", expected: "192.0.2.25" },
  { name: "untrusted direct peer", peer: "192.0.2.10", expected: "127.0.0.1" },
]) test(`project Caddy replaces spoofed client headers for ${fixture.name} without stripping API path`, {
  skip: !binary, timeout: 20_000,
}, async t => {
  const upstream = createServer((request, response) => {
    response.setHeader("Content-Type", "application/json");
    response.end(JSON.stringify({ path: request.url, client: request.headers["x-crowned-client-ip"] }));
  });
  await new Promise<void>(resolve => upstream.listen(0, "127.0.0.1", resolve));
  t.after(() => new Promise<void>(resolve => upstream.close(() => resolve())));
  const probe = createServer();
  await new Promise<void>(resolve => probe.listen(0, "127.0.0.1", resolve));
  const port = (probe.address() as AddressInfo).port;
  await new Promise<void>(resolve => probe.close(() => resolve()));
  const parent = await realpath(tmpdir()), directory = await mkdtemp(join(parent, "crowned-dogs-proxy-test-"));
  t.after(async () => {
    const actual = await realpath(directory);
    if (actual !== resolve(directory) || !actual.startsWith(parent + sep)
      || !actual.slice(parent.length + 1).startsWith("crowned-dogs-proxy-test-")) throw new Error("Unsafe test cleanup");
    await rm(actual, { recursive: true });
  });
  const env = { ...process.env, PROJECT_WEB_PORT: String(port), SHARED_GATEWAY_PEER: fixture.peer,
    ONLINE_UPSTREAM: `127.0.0.1:${(upstream.address() as AddressInfo).port}` };
  const configFile = join(directory, "Caddyfile");
  await writeFile(configFile, await readFile("infra/online/project.Caddyfile"));
  const config = JSON.parse((await run(binary!, ["adapt", "--config", configFile, "--adapter", "caddyfile"], { env })).stdout);
  // Keep the real adapted routing/header logic. Isolate only test listeners and the admin endpoint.
  config.admin = { disabled: true };
  for (const server of Object.values(config.apps.http.servers) as { listen: string[] }[]) server.listen = [`127.0.0.1:${port}`];
  const jsonFile = join(directory, "caddy.json");
  await writeFile(jsonFile, JSON.stringify(config));
  const child = spawn(binary!, ["run", "--config", jsonFile], { env, windowsHide: true, stdio: "ignore" });
  const exited = new Promise(resolve => child.once("exit", resolve));
  t.after(async () => { if (child.exitCode === null && child.signalCode === null) child.kill(); await exited; });
  const base = `http://127.0.0.1:${port}`;
  let ready = false;
  for (let i = 0; i < 100 && !ready; i++) {
    if (child.exitCode !== null) throw new Error("Test Caddy failed to start");
    try { ready = (await fetch(base + "/r2/api/online/health")).ok; } catch { /* startup */ }
    if (!ready) await new Promise(resolve => setTimeout(resolve, 25));
  }
  assert.equal(ready, true);
  const response = await fetch(base + "/r2/api/online/me?smoke=1", { headers: {
    "X-Forwarded-For": "198.51.100.99, 192.0.2.25", "X-Crowned-Client-IP": "203.0.113.8",
  } });
  assert.equal(response.status, 200);
  assert.deepEqual(await response.json(), { path: "/r2/api/online/me?smoke=1", client: fixture.expected });
  assert.equal((await fetch(base + "/not-an-api-route")).status, 404);
});
