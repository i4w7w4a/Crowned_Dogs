import assert from "node:assert/strict";
import { randomUUID } from "node:crypto";
import { execFile } from "node:child_process";
import { promisify } from "node:util";
import { mkdtemp, readFile, writeFile, realpath, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve, sep } from "node:path";
import { test } from "node:test";
import { Pool } from "pg";
import { PgDatabase } from "../src/online/database.ts";
import { migrate, OnlineStore } from "../src/online/storage.ts";
import { importOperators, ImportRejected } from "../src/online/import-operators.ts";
import { createBackup, verifyRestore } from "../src/online/data-backup.ts";
import { dataTarget } from "../src/online/data-target.ts";
import { nativePoolConfigs } from "./online-native-config.ts";
import { clock } from "./online-fixtures.ts";

const connectionString = process.env.ONLINE_TEST_DATABASE_URL;
const run = promisify(execFile);
const sample = [
  { telegramId: "701", tag: "K-001-040", name: "Synthetic One", signedAt: clock },
  { telegramId: "702", tag: "K-001-041", name: "Synthetic Two", signedAt: clock },
];

test("native import holds the registration allocator and rolls back late SQL failure", { skip: !connectionString, timeout: 60_000 }, async t => {
  const schema = "online_test_" + randomUUID().replaceAll("-", "");
  const config = nativePoolConfigs(connectionString!, schema);
  const admin = new Pool(config.admin), pool = new Pool(config.application);
  t.after(async () => { await pool.end(); try { await admin.query(`DROP SCHEMA ${schema} CASCADE`); } finally { await admin.end(); } });
  await admin.query(`CREATE SCHEMA ${schema}`);
  const db = new PgDatabase(pool);
  await migrate(db);
  await importOperators(db, sample);
  assert.equal((await db.query("SELECT count(*)::int n FROM online_operators")).rows[0]?.n, 0);
  // Hold the import open on a real connection after inserts, then start registration on another.
  let release!: () => void, inserted!: () => void;
  const held = new Promise<void>(resolve => { release = resolve; });
  const ready = new Promise<void>(resolve => { inserted = resolve; });
  const importing = importOperators({ query: db.query.bind(db), transaction: fn => db.transaction(async tx => {
    const result = await fn(tx); inserted(); await held; return result;
  }) }, sample, { apply: true });
  let registering: ReturnType<OnlineStore["login"]> | undefined;
  try {
    await ready;
    registering = new OnlineStore(db).login({ id: "703", firstName: "Third", username: null, lastName: null }, clock);
    let waiting = false;
    for (let i = 0; i < 100 && !waiting; i++) {
      waiting = (await admin.query(`SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE
        datname=current_database() AND wait_event_type='Lock' AND query LIKE '%online_tag_counters%') AS waiting`)).rows[0]!.waiting;
      if (!waiting) await new Promise(resolve => setTimeout(resolve, 10));
    }
    assert.equal(waiting, true, "registration must wait for the importing transaction's allocator lock");
    release();
    await importing;
    assert.equal((await registering).account.tag, "K-001-042");
  } finally { release(); await importing; await registering; }
  await db.query("UPDATE online_operators SET profile=jsonb_set(profile,'{xp}','44') WHERE telegram_id=701");
  await Promise.all([importOperators(db, sample, { apply: true }), importOperators(db, sample, { apply: true })]);
  assert.equal((await db.query("SELECT profile FROM online_operators WHERE telegram_id=701")).rows[0]?.profile.xp, 44);
  await assert.rejects(importOperators(db, [{ ...sample[0], tag: "K-001-055" }], { apply: true }), ImportRejected);
  await db.query("ALTER TABLE online_operators ADD CONSTRAINT reject_later CHECK(telegram_id<>705) NOT VALID");
  await assert.rejects(importOperators(db, [{ ...sample[0], telegramId: "704", tag: "K-001-043" },
    { ...sample[1], telegramId: "705", tag: "K-001-044" }], { apply: true }));
  assert.equal((await db.query("SELECT count(*)::int n FROM online_operators")).rows[0]?.n, 3);
  assert.equal((await db.query("SELECT last_tag FROM online_tag_counters")).rows[0]?.last_tag, "K-001-042");
});

test("native PG18 backup restores accounts, sessions, migration metadata and future tables into an empty database", {
  skip: !connectionString || !process.env.PG_BIN, timeout: 120_000,
}, async t => {
  const suffix = randomUUID().replaceAll("-", "");
  const config = nativePoolConfigs(connectionString!, "online_test_" + suffix);
  const admin = new Pool(config.admin);
  const sourceName = "crowned_dogs_test_backup_" + suffix, targetName = "crowned_dogs_test_restore_" + suffix;
  const base = dataTarget(connectionString!, new URL(connectionString!).pathname.slice(1));
  const source = { ...base, database: sourceName }, target = { ...base, database: targetName };
  const src = new Pool({ ...source, max: 1 }), dst = new Pool({ ...target, max: 1 });
  const parent = await realpath(tmpdir());
  const directory = await mkdtemp(join(parent, "crowned-dogs-data-test-"));
  t.after(async () => {
    await src.end(); await dst.end();
    try {
      // Names are generated here, never read from environment; databases contain only this fixture.
      await admin.query(`DROP DATABASE IF EXISTS ${sourceName}`);
      await admin.query(`DROP DATABASE IF EXISTS ${targetName}`);
    } finally { await admin.end(); }
    const actual = await realpath(directory);
    if (actual !== resolve(directory) || !actual.startsWith(parent + sep) || !actual.slice(parent.length + 1).startsWith("crowned-dogs-data-test-")) throw new Error("Unsafe test cleanup");
    await rm(actual, { recursive: true });
  });
  await admin.query(`CREATE DATABASE ${sourceName} TEMPLATE template0`);
  await admin.query(`CREATE DATABASE ${targetName} TEMPLATE template0`);
  const db = new PgDatabase(src);
  await migrate(db);
  const migratedVersions = (await db.query("SELECT version FROM online_schema_migrations ORDER BY version")).rows.map(row => row.version);
  const migratedTables = (await db.query("SELECT count(*)::int n FROM pg_tables WHERE schemaname='public'")).rows[0]!.n;
  const input = join(directory, "synthetic-operators.json");
  const sourceUrl = new URL(connectionString!); sourceUrl.pathname = "/" + sourceName;
  const env = { ...process.env, IMPORT_TEST_URL: sourceUrl.href };
  await writeFile(input, JSON.stringify(sample));
  const original = await readFile(input);
  const args = ["apps/server/src/online/import-cli.ts", "--file", input, "--database-env", "IMPORT_TEST_URL", "--expect-database", sourceName];
  assert.equal(JSON.parse((await run(process.execPath, args, { env })).stdout).mode, "dry-run");
  assert.equal((await db.query("SELECT count(*)::int n FROM online_operators")).rows[0]?.n, 0);
  assert.equal(JSON.parse((await run(process.execPath, [...args, "--apply"], { env })).stdout).created, 2);
  assert.deepEqual(await readFile(input), original);
  const session = await new OnlineStore(db).login({ id: "701", firstName: "Test", username: null, lastName: null }, clock);
  await db.query("CREATE TABLE future_state(id integer GENERATED ALWAYS AS IDENTITY PRIMARY KEY, payload jsonb); INSERT INTO future_state(payload) VALUES ('{\"private\":\"must survive\"}'); INSERT INTO online_schema_migrations(version) VALUES(77)");
  const backup = join(directory, "backup");
  const options = { source, directory: backup, codeVersion: "f".repeat(40), pgBin: process.env.PG_BIN };
  const backupArgs = ["apps/server/src/online/backup-cli.ts", "backup", "--directory", backup,
    "--database-env", "IMPORT_TEST_URL", "--expect-database", sourceName, "--code-version", options.codeVersion];
  const { ok, operation, ...report } = JSON.parse((await run(process.execPath, backupArgs, { env })).stdout);
  assert.equal(ok, true);
  assert.equal(operation, "backup");
  assert.deepEqual(report.versions, [...migratedVersions, 77]);
  assert.equal(report.tables, migratedTables + 1);
  const archivePath = join(backup, "archive.dump"), archive = await readFile(archivePath);
  await writeFile(archivePath, Buffer.concat([archive, Buffer.from("corrupt")]));
  await assert.rejects(verifyRestore({ target, directory: backup, pgBin: process.env.PG_BIN }), /integrity/);
  assert.equal((await dst.query("SELECT count(*)::int n FROM information_schema.tables WHERE table_schema='public'")).rows[0]?.n, 0);
  await writeFile(archivePath, archive);
  // Post-backup writes do not change the restored snapshot.
  await db.query("UPDATE online_operators SET profile=jsonb_set(profile,'{xp}','999') WHERE telegram_id=701");
  const targetUrl = new URL(connectionString!); targetUrl.pathname = "/" + targetName;
  const restoredResult = JSON.parse((await run(process.execPath, ["apps/server/src/online/backup-cli.ts", "verify-restore",
    "--directory", backup, "--database-env", "RESTORE_TEST_URL", "--expect-database", targetName],
  { env: { ...env, RESTORE_TEST_URL: targetUrl.href } })).stdout);
  const { ok: restoreOk, operation: restoreOperation, ...restored } = restoredResult;
  assert.equal(restoreOk, true);
  assert.equal(restoreOperation, "verify-restore");
  assert.deepEqual(restored, report);
  assert.equal((await new OnlineStore(new PgDatabase(dst)).authenticate(session.token, clock + 1))?.profile.xp, 0);
  assert.equal((await dst.query("INSERT INTO future_state(payload) VALUES ('{}') RETURNING id")).rows[0]?.id, 2);
  await assert.rejects(verifyRestore({ target, directory: backup, pgBin: process.env.PG_BIN }), /empty/);
  assert.equal((await dst.query("SELECT count(*)::int n FROM future_state")).rows[0]?.n, 2);
  await assert.rejects(verifyRestore({ target: source, directory: backup, pgBin: process.env.PG_BIN }), /restore target/);
  await assert.rejects(createBackup(options), /exist/i);
});
