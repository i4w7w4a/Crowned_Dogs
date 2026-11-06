import assert from "node:assert/strict";
import { test } from "node:test";
import { embeddedDatabase } from "./online-fixtures.ts";
import { snapshotData } from "../src/online/data-backup.ts";

test("backup fingerprint detects changed values even when row counts and migrations match", async t => {
  const fixture = await embeddedDatabase();
  t.after(() => fixture.close());
  const db = fixture.database;
  await db.query("CREATE TABLE online_schema_migrations(version integer PRIMARY KEY); INSERT INTO online_schema_migrations VALUES (1),(2),(3); CREATE TABLE probe(id integer, value text); INSERT INTO probe VALUES (1,'private-a'),(2,'private-b')");
  const first = await db.transaction(snapshotData);
  assert.deepEqual((first as any).versions, [1, 2, 3]);
  assert.equal((first as any).tables.find((x: any) => x.name === "probe").rows, 2);
  assert.doesNotMatch(JSON.stringify(first), /private-a|private-b/);
  await db.query("UPDATE probe SET value='changed' WHERE id=1");
  assert.notDeepEqual(await db.transaction(snapshotData), first);
  await db.query("DELETE FROM probe; INSERT INTO probe VALUES (2,'private-b'),(1,'private-a')");
  assert.deepEqual(await db.transaction(snapshotData), first);
  await db.query("CREATE TABLE empty_table(value text)");
  const withEmpty = await db.transaction(snapshotData);
  await db.query("ALTER TABLE empty_table RENAME COLUMN value TO other");
  assert.notDeepEqual(await db.transaction(snapshotData), withEmpty);
});
