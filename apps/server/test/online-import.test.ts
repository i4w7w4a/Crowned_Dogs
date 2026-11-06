import assert from "node:assert/strict";
import { test } from "node:test";
import { clock, embeddedDatabase } from "./online-fixtures.ts";
import { migrate, OnlineStore } from "../src/online/storage.ts";
import { importOperators, ImportRejected } from "../src/online/import-operators.ts";

export const legacy = [
  { telegramId: "501", tag: "K-001-007", name: "Первый", signedAt: clock - 1000 },
  { telegramId: "502", tag: "K-001-009", name: "Второй", signedAt: clock - 500 },
];

test("legacy import previews without writes, preserves identities and advances registration", async t => {
  const fixture = await embeddedDatabase();
  t.after(() => fixture.close());
  const db = fixture.database;
  await migrate(db);
  assert.deepEqual(await importOperators(db, legacy), { mode: "dry-run", total: 2, created: 2, existing: 0 });
  assert.equal((await db.query("SELECT count(*)::int n FROM online_operators")).rows[0]?.n, 0);
  assert.equal((await db.query("SELECT last_tag FROM online_tag_counters")).rows[0]?.last_tag, null);
  assert.deepEqual(await importOperators(db, legacy, { apply: true }), { mode: "apply", total: 2, created: 2, existing: 0 });
  const rows = (await db.query("SELECT * FROM online_operators ORDER BY telegram_id")).rows;
  assert.deepEqual(rows.map(r => [String(r.telegram_id), r.dog_tag, r.profile.level, r.profile.xp, r.appearance.tag]),
    [["501", "K-001-007", 1, 0, "K-001-007"], ["502", "K-001-009", 1, 0, "K-001-009"]]);
  assert.equal(rows[0]?.profile.free, 3);
  assert.equal(rows[0]?.appearance.sex, "male");
  assert.ok(rows[0]?.loadout.weapon1.item);
  assert.equal(new Date(rows[0]?.created_at).getTime(), clock - 1000);
  assert.equal((await db.query("SELECT count(*)::int n FROM online_sessions")).rows[0]?.n, 0);
  assert.equal((await db.query("SELECT last_tag FROM online_tag_counters")).rows[0]?.last_tag, "K-001-009");
  const store = new OnlineStore(db);
  const session = await store.login({ id: "501", firstName: "Изменено", lastName: null, username: null }, clock);
  await db.query("UPDATE online_operators SET profile=jsonb_set(profile,'{xp}','27'), appearance=jsonb_set(appearance,'{sex}','\"female\"'), version=3 WHERE telegram_id=501");
  const before = (await db.query("SELECT * FROM online_operators ORDER BY telegram_id")).rows;
  assert.deepEqual(await importOperators(db, legacy, { apply: true }), { mode: "apply", total: 2, created: 0, existing: 2 });
  assert.deepEqual((await db.query("SELECT * FROM online_operators ORDER BY telegram_id")).rows, before);
  assert.equal((await store.authenticate(session.token, clock + 1))?.profile.xp, 27);
  const next = await store.login({ id: "503", firstName: "Новый", lastName: null, username: null }, clock);
  assert.equal(next.account.tag, "K-001-010");
});

test("legacy validation aggregates categories without exposing personal data", async t => {
  const fixture = await embeddedDatabase();
  t.after(() => fixture.close());
  await migrate(fixture.database);
  for (const invalid of [null, {}, [null], [{ ...legacy[0], telegramId: 501 }],
    [{ ...legacy[0], telegramId: "0501" }], [{ ...legacy[0], telegramId: "9007199254740992" }],
    [{ ...legacy[0], tag: "K-000-001" }], [{ ...legacy[0], tag: "K-001-000" }],
    [{ ...legacy[0], signedAt: -1 }], [{ ...legacy[0], signedAt: 8640000000000001 }],
    [{ ...legacy[0], name: "\u0000secret" }], [{ ...legacy[0], profile: { level: 4 } }],
    [legacy[0], { ...legacy[1], telegramId: "501" }], [legacy[0], { ...legacy[1], tag: "K-001-007" }]]) {
    await assert.rejects(importOperators(fixture.database, invalid, { apply: true }), error => {
      assert.ok(error instanceof ImportRejected);
      assert.ok(Object.keys(error.categories).length);
      assert.doesNotMatch(JSON.stringify(error), /501|Первый|secret|K-001/);
      return true;
    });
  }
  assert.equal((await fixture.database.query("SELECT count(*)::int n FROM online_operators")).rows[0]?.n, 0);
});

test("conflicts and late SQL failure leave the entire import and allocator unchanged", async t => {
  const fixture = await embeddedDatabase();
  t.after(() => fixture.close());
  const db = fixture.database;
  await migrate(db);
  await importOperators(db, [legacy[0]], { apply: true });
  for (const conflict of [{ ...legacy[0], tag: "K-001-008" }, { ...legacy[1], telegramId: "599", tag: "K-001-007" }]) {
    await assert.rejects(importOperators(db, [legacy[1], conflict], { apply: true }), ImportRejected);
    assert.equal((await db.query("SELECT count(*)::int n FROM online_operators")).rows[0]?.n, 1);
  }
  await db.query("ALTER TABLE online_operators ADD CONSTRAINT reject_second CHECK (telegram_id<>504) NOT VALID");
  await assert.rejects(importOperators(db, [legacy[1], { ...legacy[1], telegramId: "504", tag: "K-001-011" }], { apply: true }));
  assert.equal((await db.query("SELECT count(*)::int n FROM online_operators")).rows[0]?.n, 1);
  assert.equal((await db.query("SELECT last_tag FROM online_tag_counters")).rows[0]?.last_tag, "K-001-007");
});
