import assert from "node:assert/strict";
import { test } from "node:test";
import { GEAR_SLOTS, MOD_SLOTS, WEAPONS, compileKit, starterLoadout, weaponItem, type ModSlot } from "@crowned-dogs/core";
import {
  LOADOUT_STORAGE_KEY, effectiveWeaponModId, equipItem, equipWeaponMod, equippedItemId, isValidLoadout, itemsForSlot,
  loadLocalLoadout, saveLocalLoadout, weaponModsForSlot, type LoadoutStorage,
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

test("all five sockets expose the weapon's factory part and compatible choices", () => {
  for (const weapon of ["pistol", "shotgun", "rifle", "sniper"] as const) {
    const kit = starterLoadout(weapon);
    for (const slot of MOD_SLOTS) {
      const choices = weaponModsForSlot(kit, slot);
      assert.equal(choices[0]?.id, weaponItem(weapon).stock[slot]);
      assert.equal(effectiveWeaponModId(kit, slot), choices[0]?.id);
      assert.ok(choices.every((item) => item.slot === slot && item.fits.includes(weapon)));
    }
  }
});

test("weapon mods survive saving and change actual compiled combat properties", () => {
  const base = starterLoadout("rifle");
  const scoped = equipWeaponMod(base, "optic", "optic-sokol");
  const suppressed = equipWeaponMod(scoped, "muzzle", "muzzle-tishina");
  assert.equal(effectiveWeaponModId(suppressed, "optic"), "optic-sokol");
  assert.equal(effectiveWeaponModId(suppressed, "muzzle"), "muzzle-tishina");
  assert.equal(compileKit(base).weapon.suppressor, false);
  assert.equal(compileKit(suppressed).weapon.suppressor, true);
  assert.ok(compileKit(scoped).weapon.accuracy[10]! > compileKit(base).weapon.accuracy[10]!);
  assert.ok(compileKit(scoped).weapon.accuracy[2]! < compileKit(base).weapon.accuracy[2]!);
  const store = storage();
  assert.equal(saveLocalLoadout(suppressed, store), true);
  assert.deepEqual(loadLocalLoadout(store), suppressed);
  assert.deepEqual(compileKit(loadLocalLoadout(store)), compileKit(suppressed));
});

test("mod changes preserve other gear and independently copy both weapon configurations", () => {
  const base = equipItem(starterLoadout("rifle"), "weapon2", "pistol-pustelga");
  base.weapon1.mods = { optic: "optic-sokol" };
  base.weapon1.skin = "skin-factory";
  base.weapon2!.mods = { muzzle: "muzzle-pb9" };
  const before = structuredClone(base);
  const changed = equipWeaponMod(base, "muzzle", "muzzle-tishina");
  assert.deepEqual(base, before);
  assert.deepEqual(changed.weapon2, before.weapon2);
  assert.notEqual(changed.weapon2, base.weapon2);
  assert.notEqual(changed.weapon2!.mods, base.weapon2!.mods);
  assert.equal(changed.weapon1.skin, base.weapon1.skin);
  assert.equal(changed.weapon1.mods!.optic, "optic-sokol");
  for (const slot of GEAR_SLOTS) {
    if (slot !== "weapon1" && slot !== "weapon2") assert.equal(changed[slot], base[slot]);
  }
  changed.weapon1.mods!.optic = "optic-gran";
  changed.weapon2!.mods!.muzzle = "pistol-muzzle-stock";
  assert.deepEqual(base, before);
});

test("restoring factory parts clears only the chosen override and restores baseline effects", () => {
  const base = starterLoadout("rifle");
  const modified = equipWeaponMod(equipWeaponMod(base, "optic", "optic-sokol"), "muzzle", "muzzle-tishina");
  const factoryId = weaponItem("rifle").stock.muzzle;
  const reset = equipWeaponMod(modified, "muzzle", null);
  assert.deepEqual(reset, equipWeaponMod(modified, "muzzle", factoryId));
  assert.equal(effectiveWeaponModId(reset, "muzzle"), factoryId);
  assert.equal(compileKit(reset).weapon.suppressor, false);
  assert.equal(reset.weapon1.mods!.optic, "optic-sokol");
  const stock = equipWeaponMod(reset, "optic", null);
  assert.equal(stock.weapon1.mods, undefined);
  assert.deepEqual(compileKit(stock), compileKit(base));
  assert.equal(modified.weapon1.mods!.muzzle, "muzzle-tishina");
});

test("wrong sockets, incompatible parts and malformed mod saves are rejected", () => {
  const rifle = starterLoadout("rifle");
  assert.throws(() => equipWeaponMod(rifle, "muzzle", "optic-sokol"));
  assert.throws(() => equipWeaponMod(rifle, "muzzle", "muzzle-pb9"));
  assert.throws(() => equipWeaponMod(rifle, "muzzle", "pistol-muzzle-stock"));
  assert.throws(() => equipWeaponMod(rifle, "optic", "missing-mod"));
  assert.throws(() => equipWeaponMod(rifle, "unknown" as ModSlot, null));
  const invalid = { ...rifle, weapon1: { ...rifle.weapon1, mods: { muzzle: "optic-sokol" } } };
  assert.equal(isValidLoadout(invalid), false);
  assert.throws(() => equipWeaponMod(invalid, "optic", null));
  assert.deepEqual(loadLocalLoadout(storage(JSON.stringify({ version: 1, loadout: invalid }))), rifle);
});
