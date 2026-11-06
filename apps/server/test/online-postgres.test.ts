import assert from "node:assert/strict";
import { randomUUID } from "node:crypto";
import { test } from "node:test";
import { Pool } from "pg";
import { PgDatabase } from "../src/online/database.ts";
import { migrate, OnlineStore } from "../src/online/storage.ts";
import { clock } from "./online-fixtures.ts";

// Never falls back to ONLINE_DATABASE_URL or .env. Only an explicitly isolated local test DB.
const connectionString = process.env.ONLINE_TEST_DATABASE_URL;
test("native PostgreSQL pins transactions, releases after rollback and serializes concurrent registrations", { skip: !connectionString, timeout: 30_000 }, async t => {
  const url = new URL(connectionString!);
  assert.ok(["localhost", "127.0.0.1", "[::1]"].includes(url.hostname), "Native tests require a loopback database");
  assert.match(url.pathname, /^\/crowned_dogs_test(?:_[a-z0-9_]+)?$/i, "Native tests require a dedicated crowned_dogs_test database");
  const schema = "online_test_" + randomUUID().replaceAll("-", "");
  assert.match(schema, /^online_test_[a-f0-9]{32}$/);
  const admin = new Pool({ connectionString, max: 1 });
  const pool = new Pool({ connectionString, max: 4, options: `-c search_path=${schema}`, connectionTimeoutMillis: 5000 });
  t.after(async () => {
    await pool.end();
    // Identifier is generated above and validated; never derived from environment input.
    try { await admin.query(`DROP SCHEMA IF EXISTS ${schema} CASCADE`); }
    finally { await admin.end(); }
  });
  await admin.query(`CREATE SCHEMA ${schema}`);
  const db = new PgDatabase(pool);
  await migrate(db);
  await migrate(db);
  await db.query("CREATE TABLE transaction_probe (value integer)");
  await assert.rejects(db.transaction(async tx => {
    await tx.query("INSERT INTO transaction_probe VALUES ($1)", [7]);
    throw new Error("intentional rollback");
  }), /intentional rollback/);
  assert.equal((await db.query("SELECT count(*)::int AS count FROM transaction_probe")).rows[0]?.count, 0);
  await db.transaction(async tx => { await tx.query("INSERT INTO transaction_probe VALUES ($1)", [8]); });
  assert.equal((await db.query("SELECT value FROM transaction_probe")).rows[0]?.value, 8);
  const identity = { id: "100", firstName: "Test", lastName: null, username: null };
  const stores = [new OnlineStore(db), new OnlineStore(db)];
  const same = await Promise.all(Array.from({ length: 12 }, (_, i) => stores[i % 2]!.login(identity, clock)));
  assert.equal(new Set(same.map(x => x.account.id)).size, 1);
  assert.equal(new Set(same.map(x => x.account.tag)).size, 1);
  const distinct = await Promise.all(Array.from({ length: 12 }, (_, i) => stores[i % 2]!.login({ ...identity, id: String(200 + i) }, clock)));
  assert.equal(new Set(distinct.map(x => x.account.tag)).size, 12);
  assert.equal((await db.query("SELECT count(*)::int AS count FROM online_operators")).rows[0]?.count, 13);
  // Force an SQL error; every pooled connection must remain usable afterwards.
  await assert.rejects(db.transaction(tx => tx.query("SELECT * FROM nonexistent_table")));
  await Promise.all(Array.from({ length: 8 }, () => db.transaction(tx => tx.query("SELECT 1"))));
  assert.equal(pool.waitingCount, 0);
  assert.equal(pool.idleCount, pool.totalCount);
});
