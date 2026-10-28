import assert from "node:assert/strict";
import { test } from "node:test";
import { GEAR_SLOTS, WEAPONS, compileKit, starterLoadout } from "@crowned-dogs/core";
import {
  LOADOUT_STORAGE_KEY, equipItem, equippedItemId, isValidLoadout, itemsForSlot,
  loadLocalLoadout, saveLocalLoadout, type LoadoutStorage,
} from "../src/base/loadout-state.ts";

function storage(initial: string | null = null): LoadoutStorage {
  const values = new Map<string, string>();
  if (initial !== null) values.set(LOADOUT_STORAGE_KEY, initial);
  return { getItem: (key) => values.get(key) ?? null, setItem: (key, value) => { values.set(key, value); } };
}

test("each real gear slot exposes compatible catalogue items and excludes gun mods", () => {
  assert.equal(GEAR_SLOTS.length, 8);
  for (const slot of GEAR_SLOTS) {
    const choices = itemsForSlot(slot);
    assert.ok(choices.length > 0, slot);
    for (const item of choices) {
      const next = equipItem(starterLoadout("rifle"), slot, item.id);
      assert.ok(isValidLoadout(next), `${slot}: ${item.id}`);
      assert.equal(equippedItemId(next, slot), item.id);
      assert.notEqual(item.kind, "mod");
    }
  }
});

test("offhand cannot accept two-handed weapons; only it can be emptied", () => {
  const base = starterLoadout("rifle");
  assert.ok(itemsForSlot("weapon2").every((item) => item.kind === "weapon" && WEAPONS[item.weapon].hands === 1));
  assert.throws(() => equipItem(base, "weapon2", "rifle-varyag"));
  assert.throws(() => equipItem(base, "head", null));
  assert.throws(() => equipItem(base, "head", "boots-bertsy"));
  assert.throws(() => equipItem(base, "body", "not-a-real-item"));
  const withSidearm = equipItem(base, "weapon2", "pistol-pustelga");
  assert.equal(equippedItemId(withSidearm, "weapon2"), "pistol-pustelga");
  assert.equal(equippedItemId(equipItem(withSidearm, "weapon2", null), "weapon2"), null);
  assert.equal(base.weapon2, null, "the input was not mutated");
});

test("reselecting weapons preserves parts independently; changing weapons drops incompatible parts", () => {
  const base = starterLoadout("rifle");
  base.weapon1.mods = { optic: "optic-sokol" };
  const same = equipItem(base, "weapon1", base.weapon1.item);
  assert.deepEqual(same.weapon1.mods, base.weapon1.mods);
  assert.notEqual(same.weapon1.mods, base.weapon1.mods);
  const changed = equipItem(base, "weapon1", "pistol-pustelga");
  assert.deepEqual(changed.weapon1, { item: "pistol-pustelga" });
  assert.deepEqual(base.weapon1.mods, { optic: "optic-sokol" });
});

test("equipped armour produces the core combat numbers, not cosmetic-only stats", () => {
  const base = starterLoadout("rifle");
  const changed = equipItem(base, "head", "head-sfera");
  assert.equal(compileKit(base).armor.head, 0);
  assert.equal(compileKit(changed).armor.head, 25);
});

test("valid loadout survives local storage and returns independent objects", () => {
  const store = storage();
  const kit = equipItem(starterLoadout("rifle"), "head", "head-k6");
  kit.weapon1.mods = { optic: "optic-sokol" };
  assert.equal(saveLocalLoadout(kit, store), true);
  assert.deepEqual(loadLocalLoadout(store), kit);
  const loaded = loadLocalLoadout(store);
  loaded.weapon1.mods!.optic = "optic-gran";
  assert.equal(loadLocalLoadout(store).weapon1.mods!.optic, "optic-sokol");
});

test("malformed JSON, missing nested fields and invalid item combinations safely reset to starter", () => {
  const base = starterLoadout("rifle");
  const invalid: unknown[] = [
    null, [], {}, { ...base, weapon1: null }, { ...base, weapon2: undefined },
    { ...base, head: "missing" }, { ...base, weapon2: { item: "rifle-varyag" } },
    { ...base, weapon1: { item: base.weapon1.item, mods: [] } },
    { ...base, weapon1: { item: base.weapon1.item, mods: { optic: null } } },
    { ...base, weapon1: { item: base.weapon1.item, mods: { unknown: "optic-sokol" } } },
    { ...base, weapon1: { item: "pistol-pustelga", mods: { optic: "optic-sokol" } } },
  ];
  for (const loadout of invalid) {
    assert.equal(isValidLoadout(loadout), false);
    assert.deepEqual(loadLocalLoadout(storage(JSON.stringify({ version: 1, loadout }))), base);
  }
  for (const raw of ["broken{", "null", "[]", JSON.stringify({ version: 2, loadout: base })]) {
    assert.deepEqual(loadLocalLoadout(storage(raw)), base);
  }
});

test("blocked or quota-exhausted storage does not break selection", () => {
  const blocked: LoadoutStorage = {
    getItem() { throw new Error("blocked"); },
    setItem() { throw new Error("quota"); },
  };
  assert.deepEqual(loadLocalLoadout(blocked), starterLoadout("rifle"));
  assert.deepEqual(loadLocalLoadout(null), starterLoadout("rifle"));
  assert.equal(saveLocalLoadout(starterLoadout("rifle"), blocked), false);
  assert.equal(saveLocalLoadout(starterLoadout("rifle"), null), false);
});
