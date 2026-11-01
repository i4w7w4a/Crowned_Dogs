import assert from "node:assert/strict";
import { test } from "node:test";
import { botOrder } from "../src/ai.ts";
import { ITEMS, MOD_SLOTS, modsFor, type WeaponItem } from "../src/items.ts";
import {
  STARTER, VETERAN_KITS, compileKit, describeItem, kitFighter, starterLoadout, validateLoadout, type Loadout,
} from "../src/loadout.ts";
import { createMatch, hitChance, resolveTurn, weaponSpec, type FighterSetup, type Match, type Order, type Side } from "../src/match.ts";
import { seedRng } from "../src/rng.ts";
import { DEFAULT_ARMOR, GRENADE, WEAPONS, WEAPON_IDS, type WeaponId } from "../src/rules.ts";

const weapons = ITEMS.filter((i): i is WeaponItem => i.kind === "weapon");

test("the catalogue is consistent", () => {
  const ids = ITEMS.map((i) => i.id);
  assert.equal(new Set(ids).size, ids.length, "item ids are unique");
  assert.deepEqual(weapons.map((w) => w.weapon).sort(), [...WEAPON_IDS].sort(), "one base weapon per class");
  for (const w of weapons) {
    assert.equal(Object.keys(w.stock).length, 5, `${w.id} has five mod slots`);
    for (const slot of MOD_SLOTS) {
      const options = modsFor(w.weapon, slot);
      assert.equal(options[0]!.id, w.stock[slot], `${w.id}: the factory ${slot} comes first`);
      assert.ok(options[0]!.isStock, `${w.id}: the factory ${slot} is marked as stock`);
      assert.ok(options.length >= 3, `${w.id}: at least two mods for ${slot}`);
    }
  }
});

test("every item that is not a factory part or paint changes the fight, and says how", () => {
  for (const item of ITEMS) {
    if (item.kind === "skin" || item.kind === "weapon") continue;
    if (item.kind === "mod" && item.isStock) {
      assert.deepEqual(item.effects, {}, `${item.id} is a factory part`);
      continue;
    }
    const starter = Object.values(STARTER).includes(item.id);
    if (starter) continue;
    assert.ok(item.kind === "grenade" || Object.keys(item.effects).length > 0, `${item.id} does something`);
    assert.ok(describeItem(item).length > 0, `${item.id} has words for the gear screen`);
  }
});

test("a starter kit is exactly the plain rules", () => {
  for (const w of WEAPON_IDS) {
    const kit = compileKit(starterLoadout(w));
    const base = WEAPONS[w];
    assert.deepEqual(kit.weapon.accuracy, [...base.accuracy]);
    assert.deepEqual(kit.weapon.falloff, [...base.falloff]);
    assert.equal(kit.weapon.power, base.power);
    assert.equal(kit.weapon.flatPen, base.flatPen);
    assert.equal(kit.weapon.crit, base.crit);
    assert.equal(kit.weapon.doubleShot, 0);
    assert.deepEqual(kit.armor, DEFAULT_ARMOR);
    assert.equal(kit.grenades, GRENADE.perFight);
    assert.equal(kit.grenade.base, GRENADE.base);
    assert.equal(kit.dodge, 0);
    assert.deepEqual(kit.camo, { near: 0, mid: 0, far: 0 });
  }
});

function fight(seed: string, fighters: [FighterSetup, FighterSetup]): Match {
  let m = createMatch({ seed, mapId: "dam", weather: "fog", time: "night", fighters });
  while (!m.result) {
    const orders = ([0, 1] as Side[]).map((s) => botOrder(m, s));
    m = resolveTurn(m, [orders[0]!, orders[1]!]).match;
  }
  return m;
}

test("a fight in starter kits replays turn for turn like a fight without items", () => {
  for (const [a, b] of [["rifle", "sniper"], ["shotgun", "pistol"]] as [WeaponId, WeaponId][]) {
    for (let i = 0; i < 5; i++) {
      const plain = fight(`same-${i}`, [{ id: "a", name: "a", weapon: a }, { id: "b", name: "b", weapon: b }]);
      const kitted = fight(`same-${i}`, [kitFighter("a", "a", starterLoadout(a)), kitFighter("b", "b", starterLoadout(b))]);
      assert.deepEqual(kitted.history, plain.history);
      assert.deepEqual(kitted.result, plain.result);
    }
  }
});

test("kits are checked before a fight", () => {
  const rifle = starterLoadout("rifle");
  assert.equal(validateLoadout(rifle), null);
  for (const kit of Object.values(VETERAN_KITS)) assert.equal(validateLoadout(kit), null);
  assert.deepEqual(validateLoadout({ ...rifle, weapon2: { item: "shotgun-taran" } }), { slot: "weapon2", error: "two_handed" });
  assert.equal(validateLoadout({ ...rifle, weapon2: { item: "pistol-pustelga" } }), null, "a pistol is a fine sidearm");
  assert.deepEqual(
    validateLoadout({ ...rifle, weapon1: { item: "rifle-varyag", mods: { optic: "optic-oryol" } } }),
    { slot: "weapon1", mod: "optic", error: "does_not_fit" },
  );
  assert.deepEqual(
    validateLoadout({ ...rifle, weapon1: { item: "rifle-varyag", mods: { muzzle: "optic-blik" } } }),
    { slot: "weapon1", mod: "muzzle", error: "wrong_slot" },
  );
  assert.deepEqual(validateLoadout({ ...rifle, head: "boots-bertsy" }), { slot: "head", error: "wrong_slot" });
  assert.deepEqual(validateLoadout({ ...rifle, knife: "nope" }), { slot: "knife", error: "unknown_item" });
  assert.throws(() => compileKit({ ...rifle, head: "boots-bertsy" }));
});

test("effects add up over the kit", () => {
  const kit = compileKit({
    ...starterLoadout("rifle"),
    weapon1: { item: "rifle-varyag", mods: { optic: "optic-sokol", rail: "rail-koster", mag: "mag-rifle-drum" } },
    head: "head-sfera",
    body: "body-plita4",
    boots: "boots-bertsy",
    suit: "suit-ten",
  });
  assert.deepEqual(kit.armor, { head: 25, body: 20, groin: 10, legs: 10 });
  assert.equal(kit.grenades, 3, "vest pouch and underbarrel launcher");
  assert.equal(kit.dodge, -0.04);
  assert.equal(kit.dashEvade, -0.04);
  assert.deepEqual(kit.camo, { near: 0, mid: 0.03, far: 0.04 });
  assert.equal(kit.heatProof, true);
  assert.equal(kit.weapon.doubleShot, 0.08);
  // Scope: worse up close, better far away; untouched where the rifle cannot reach.
  assert.equal(kit.weapon.accuracy[0], WEAPONS.rifle.accuracy[0]! - 0.05);
  assert.equal(kit.weapon.accuracy[11], WEAPONS.rifle.accuracy[11]! + 0.07);
  const shotgun = compileKit({ ...starterLoadout("shotgun"), weapon1: { item: "shotgun-taran", mods: { muzzle: "muzzle-choke" } } });
  assert.equal(shotgun.weapon.accuracy[9], 0, "a choke does not make a shotgun reach 10 steps");
});

function duel(a: Loadout, b: Loadout, over: Partial<Parameters<typeof createMatch>[0]> = {}): Match {
  return createMatch({ seed: "kit", mapId: "dam", weather: "clear", time: "morning", fighters: [kitFighter("a", "a", a), kitFighter("b", "b", b)], ...over });
}

const rifle = starterLoadout("rifle");
const hold = (zone: "head" | "body" = "body"): Order => ({ action: { type: "shoot", move: 0, zone }, defend: ["head", "body"] });

test("night vision lifts the night penalty and camouflage hides you far away", () => {
  const night = duel(rifle, rifle, { time: "night" });
  const nvg = duel({ ...rifle, head: "head-sych" }, rifle, { time: "night" });
  const morning = duel(rifle, rifle);
  assert.ok(hitChance(night, 0, "rifle", 10, 1, false) < hitChance(morning, 0, "rifle", 10, 1, false));
  assert.equal(hitChance(nvg, 0, "rifle", 10, 1, false), hitChance(morning, 0, "rifle", 10, 1, false));

  const hidden = duel(rifle, { ...rifle, suit: "suit-ten" });
  const far = hitChance(morning, 0, "rifle", 10, 1, false) - hitChance(hidden, 0, "rifle", 10, 1, false);
  assert.ok(Math.abs(far - 0.04) < 1e-9, String(far));
  assert.equal(hitChance(hidden, 0, "rifle", 4, 1, false), hitChance(morning, 0, "rifle", 4, 1, false), "camouflage does nothing up close");
});

test("a suppressor belongs to its gun", () => {
  const quiet: Loadout = {
    ...rifle,
    weapon1: { item: "rifle-varyag", mods: { muzzle: "muzzle-tishina" } },
    weapon2: { item: "pistol-pustelga" },
  };
  const m = duel(quiet, rifle, { time: "night" });
  assert.equal(weaponSpec(m, 0, "rifle").suppressor, true);
  assert.equal(weaponSpec(m, 0, "pistol").suppressor, false, "the sidearm still flashes");
  const after = resolveTurn(m, [hold(), hold()]).match;
  assert.equal(after.fighters[0].flash, false);
  assert.equal(after.fighters[1].flash, true);
});

test("a drum magazine sometimes lands a second hit", () => {
  const drum: Loadout = { ...rifle, weapon1: { item: "rifle-varyag", mods: { mag: "mag-rifle-drum" } } };
  let doubles = 0, hits = 0;
  for (let i = 0; i < 400; i++) {
    const m = createMatch({ seed: `drum-${i}`, mapId: "dam", weather: "clear", time: "morning", fighters: [kitFighter("a", "a", drum), kitFighter("b", "b", rifle)] });
    const a = resolveTurn(m, [hold("head"), hold()]).report.attacks[0]!;
    if (a.hit) hits += 1;
    if (a.double) doubles += 1;
  }
  assert.ok(doubles > 0 && doubles < hits * 0.2, `${doubles} doubles of ${hits} hits`);
});
