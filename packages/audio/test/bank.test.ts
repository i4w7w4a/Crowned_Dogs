import assert from "node:assert/strict";
import { existsSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { test } from "node:test";
import { FIGHT_PLAYLIST, RADIO_PLAYLIST, SOUNDS, TRACKS } from "../src/bank.ts";
import { FILES } from "../src/files.generated.ts";

const ASSETS = join(import.meta.dirname, "..", "assets");

test("every sound points at files that were built", () => {
  for (const [id, def] of Object.entries(SOUNDS)) {
    assert.ok(def.files.length > 0, id);
    for (const f of def.files) {
      assert.ok(existsSync(join(ASSETS, f)), `${id}: ${f} is missing`);
      assert.ok(FILES[f], `${id}: ${f} is not in files.generated.ts`);
    }
  }
});

test("no built file is left without a sound", () => {
  const used = new Set(Object.values(SOUNDS).flatMap((d) => d.files));
  for (const dir of ["sfx", "amb", "voice"]) {
    for (const name of readdirSync(join(ASSETS, dir))) {
      assert.ok(used.has(`${dir}/${name}`), `${dir}/${name} is not used`);
    }
  }
  for (const f of Object.keys(FILES)) assert.ok(used.has(f), `${f} is listed but not used`);
});

test("ambience files are loops, effects are not", () => {
  for (const [f, info] of Object.entries(FILES)) {
    assert.equal(info.loop === true, f.startsWith("amb/"), f);
    assert.ok(Math.abs(info.trimDb) <= 4, f);
  }
});

test("every track is on disk and in exactly one playlist", () => {
  for (const [id, t] of Object.entries(TRACKS)) assert.ok(existsSync(join(ASSETS, t.file)), `${id}: ${t.file}`);
  const all = [...FIGHT_PLAYLIST, ...RADIO_PLAYLIST];
  assert.deepEqual([...all].sort(), Object.keys(TRACKS).sort());
  assert.deepEqual([...FIGHT_PLAYLIST].sort(), ["404-room", "in-the-middle-of-nowhere", "the-limbo-3"]);
});
