import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { test } from "node:test";
import { clock, embeddedDatabase } from "./online-fixtures.ts";
import { OnlineStore, migrate } from "../src/online/storage.ts";

test("SQL accounts preserve server progress, permanent tags and revocable hashed sessions", async t => {
  const fixture = await embeddedDatabase();
  t.after(() => fixture.close());
  const db = fixture.database;
  await migrate(db);
  await migrate(db);
  assert.equal((await db.query("SELECT count(*)::int AS count FROM online_schema_migrations")).rows[0]?.count, 1);
  const store = new OnlineStore(db);
  const identity = { id: "123", firstName: "Арт", lastName: null, username: "old" };
  const first = await store.login(identity, clock);
  assert.match(first.account.id, /^[a-f\d-]{36}$/);
  assert.equal(first.account.tag, "K-001-001");
  assert.equal(first.account.profile.level, 1);
  assert.equal(first.account.profile.free, 3);
  assert.equal(first.account.profile.xp, 0);
  assert.deepEqual(first.account.profile.stats, { power: 0, cooling: 0, instinct: 0, reflexes: 0 });
  assert.equal(first.account.profile.mastery.pistol, 74);
  assert.equal(first.account.appearance.sex, "male");
  assert.equal(first.account.appearance.tag, "K-001-001");
  assert.ok(first.account.loadout.weapon1.item);
  assert.match(first.token, /^[A-Za-z0-9_-]{43}$/);
  const stored = (await db.query("SELECT token_hash, expires_at FROM online_sessions")).rows[0]!;
  assert.equal(stored.token_hash, createHash("sha256").update(first.token).digest("hex"));
  assert.equal(new Date(stored.expires_at).getTime(), clock + 8 * 60 * 60 * 1000);
  // Simulate progress written by a future trusted Rust command, not a browser update.
  await db.query("UPDATE online_operators SET profile = jsonb_set(profile, '{xp}', '17') WHERE id = $1", [first.account.id]);
  const again = await store.login({ ...identity, firstName: "Новое", username: null }, clock + 1);
  assert.equal(again.account.id, first.account.id);
  assert.equal(again.account.tag, first.account.tag);
  assert.equal(again.account.profile.xp, 17);
  assert.equal(again.account.telegram.firstName, "Новое");
  assert.equal(again.account.telegram.username, null);
  assert.notEqual(again.token, first.token);
  const parallel = await Promise.all(Array.from({ length: 8 }, (_, i) => store.login({ ...identity, id: String(200 + i) }, clock)));
  assert.equal(new Set(parallel.map(x => x.account.tag)).size, 8);
  const same = await Promise.all(Array.from({ length: 8 }, () => store.login(identity, clock)));
  assert.equal(new Set(same.map(x => x.account.id)).size, 1);
  assert.equal((await db.query("SELECT count(*)::int AS count FROM online_operators")).rows[0]?.count, 9);
  assert.equal(await store.authenticate("absent", clock), null);
  assert.equal((await store.authenticate(first.token, clock))?.id, first.account.id);
  assert.equal(await store.authenticate(first.token, clock + 8 * 60 * 60 * 1000), null);
  await store.logout(first.token, clock + 2);
  assert.equal(await store.authenticate(first.token, clock + 3), null);
  await fixture.reopen();
  await migrate(db);
  assert.equal((await store.authenticate(again.token, clock + 3))?.profile.xp, 17);
  assert.equal((await store.login(identity, clock + 4)).account.id, first.account.id);

  await t.test("tag exhaustion rolls back account, counter and session atomically", async () => {
    // Nine tags were issued above; fill the remaining 990 with isolated trusted SQL fixtures.
    await db.query(`INSERT INTO online_operators
      (id, telegram_id, first_name, dog_tag, profile, appearance, loadout, created_at, updated_at)
      SELECT md5('capacity-' || n::text)::uuid, 10000+n, 'Capacity fixture',
        'K-001-' || lpad(n::text, 3, '0'), o.profile, o.appearance, o.loadout, o.created_at, o.updated_at
      FROM generate_series(10, 999) n CROSS JOIN
        (SELECT profile, appearance, loadout, created_at, updated_at FROM online_operators LIMIT 1) o`);
    assert.equal((await db.query("SELECT count(*)::int AS count FROM online_operators")).rows[0]?.count, 999);
    await db.query("UPDATE online_tag_counters SET last_tag = 'K-001-999' WHERE id = 1");
    const before = (await db.query("SELECT count(*)::int AS count FROM online_sessions")).rows[0]?.count;
    await assert.rejects(store.login({ ...identity, id: "999" }, clock), /tag.*exhaust/i);
    assert.equal((await db.query("SELECT count(*)::int AS count FROM online_operators WHERE telegram_id = '999'")).rows[0]?.count, 0);
    assert.equal((await db.query("SELECT count(*)::int AS count FROM online_sessions")).rows[0]?.count, before);
    assert.equal((await db.query("SELECT last_tag FROM online_tag_counters")).rows[0]?.last_tag, "K-001-999");
    // Existing accounts remain usable even after the month's last tag.
    assert.equal((await store.login(identity, clock)).account.id, first.account.id);
    await db.query("DELETE FROM online_operators WHERE telegram_id >= 10010");
  });
  await t.test("session insert failure rolls back new account and tag allocation", async () => {
    await db.query("UPDATE online_tag_counters SET last_tag = 'K-001-009' WHERE id = 1");
    await db.query("ALTER TABLE online_sessions ADD CONSTRAINT reject_sessions CHECK (false) NOT VALID");
    await assert.rejects(store.login({ ...identity, id: "1000" }, clock));
    assert.equal((await db.query("SELECT count(*)::int AS count FROM online_operators WHERE telegram_id = '1000'")).rows[0]?.count, 0);
    assert.equal((await db.query("SELECT last_tag FROM online_tag_counters")).rows[0]?.last_tag, "K-001-009");
    await assert.rejects(store.login({ ...identity, firstName: "Must roll back" }, clock));
    assert.equal((await db.query("SELECT first_name FROM online_operators WHERE telegram_id = $1", [identity.id])).rows[0]?.first_name, "Арт");
    await db.query("ALTER TABLE online_sessions DROP CONSTRAINT reject_sessions");
  });
});
