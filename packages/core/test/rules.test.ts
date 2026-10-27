import assert from "node:assert/strict";
import { test } from "node:test";
import { nextFloat, pickWeighted, seedRng } from "../src/rng.ts";
import { BASE_STATS, WEAPONS, armorFactor, baseDamage, grenadeDamage } from "../src/rules.ts";

test("the same seed gives the same dice, and the state survives JSON", () => {
  const a = seedRng("duel-42"), b = seedRng("duel-42");
  const first = [nextFloat(a), nextFloat(a), nextFloat(a)];
  assert.deepEqual(first, [nextFloat(b), nextFloat(b), nextFloat(b)]);
  const copy = JSON.parse(JSON.stringify(a));
  assert.equal(nextFloat(copy), nextFloat(a));
  assert.notDeepEqual(seedRng("duel-42"), seedRng("duel-43"));
});

test("numbers are in [0, 1) and weighted picks follow the weights", () => {
  const s = seedRng(1);
  const counts = { a: 0, b: 0 };
  for (let i = 0; i < 4000; i++) {
    const x = nextFloat(s);
    assert.ok(x >= 0 && x < 1);
    counts[pickWeighted(s, [["a", 3], ["b", 1]] as const)] += 1;
  }
  assert.ok(counts.a > counts.b * 2.4 && counts.a < counts.b * 3.6, JSON.stringify(counts));
});

test("base damage matches the concept doc", () => {
  // Weapon table in the doc: pistol 36, shotgun 42 (58 point-blank), rifle 37, sniper 65, knife 33, grenade 28.
  // The doc rounds, so allow less than one point either way.
  const near = (actual: number, expected: number) => assert.ok(Math.abs(actual - expected) < 1, `${actual} vs ${expected}`);
  const raw = (id: keyof typeof WEAPONS) => BASE_STATS.personal + WEAPONS[id].power * BASE_STATS.mastery;
  near(raw("pistol"), 36);
  near(raw("shotgun"), 42);
  near(baseDamage(WEAPONS.shotgun, 2), 58);
  near(baseDamage(WEAPONS.rifle, 6), 37);
  near(baseDamage(WEAPONS.sniper, 9), 65);
  near(baseDamage(WEAPONS.knife, 1), 33);
  assert.equal(grenadeDamage(20, 0), 28);
});

test("a grenade hurts less if the target steps and not at all if it runs", () => {
  assert.equal(grenadeDamage(20, 1), 14);
  assert.equal(grenadeDamage(20, -1), 14);
  assert.equal(grenadeDamage(20, 2), 0);
  assert.equal(grenadeDamage(20, 3), 0);
});

test("armour follows the 4i7 curve", () => {
  assert.equal(armorFactor(0, 0.04, 0, 0.04), 1);
  const vest = armorFactor(20, 0.04, 8, 0.04);
  assert.ok(vest > 0.8 && vest < 0.9, String(vest));
  // More penetration never makes armour stronger.
  assert.ok(armorFactor(20, 0.3, 8, 0.04) >= vest);
  assert.ok(armorFactor(20, 0.04, 12, 0.04) >= vest);
});

test("weapon ranges have the intended shape", () => {
  const acc = (id: keyof typeof WEAPONS, d: number) => WEAPONS[id].accuracy[d - 1]!;
  assert.equal(acc("shotgun", 9), 0, "a shotgun cannot reach far");
  assert.equal(acc("knife", 3), 0, "a knife needs 2 steps or less");
  assert.ok(acc("sniper", 10) > acc("sniper", 3), "a sniper prefers distance");
  assert.ok(acc("pistol", 3) > acc("pistol", 9), "a pistol prefers close range");
  assert.ok(acc("rifle", 6) >= acc("rifle", 2) && acc("rifle", 6) >= acc("rifle", 12), "a rifle prefers mid range");
});
