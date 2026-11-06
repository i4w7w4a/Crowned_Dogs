import assert from "node:assert/strict";
import type { PoolConfig } from "pg";

/** Test-only configuration boundary, shared by pure regressions and the opt-in native test. */
export function nativePoolConfigs(connectionString: string, schema: string): { admin: PoolConfig; application: PoolConfig } {
  let url: URL;
  try { url = new URL(connectionString); }
  catch { throw new Error("Invalid native test database URL"); }
  if (!["postgres:", "postgresql:"].includes(url.protocol)) throw new Error("Invalid native test database protocol");
  // pg gives URL query keys precedence over both authority fields and explicit Pool options.
  // No query options are needed by this isolated local fixture; refuse all of them.
  if (url.search) throw new Error("Native tests reject URL query parameters");
  assert.ok(["localhost", "127.0.0.1", "[::1]"].includes(url.hostname), "Native tests require a loopback database");
  assert.match(url.pathname, /^\/crowned_dogs_test(?:_[a-z0-9_]+)?$/i, "Native tests require a dedicated crowned_dogs_test database");
  assert.match(schema, /^online_test_[a-f0-9]{32}$/);
  let credentials: Pick<PoolConfig, "user" | "password">;
  try { credentials = { user: decodeURIComponent(url.username), password: decodeURIComponent(url.password) }; }
  catch { throw new Error("Invalid native test database credentials encoding"); }
  // Pass validated fields directly: the original string must never be reparsed by pg.
  const target: PoolConfig = { host: url.hostname.replace(/^\[(.+)\]$/, "$1"),
    database: url.pathname.slice(1), port: Number(url.port || "5432"), ...credentials };
  return { admin: { ...target, max: 1 },
    application: { ...target, max: 4, options: `-c search_path=${schema}`, connectionTimeoutMillis: 5000 } };
}
