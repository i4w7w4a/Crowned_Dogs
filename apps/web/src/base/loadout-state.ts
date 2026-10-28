/** Local fitting-room selections. Catalogue access is not server-owned inventory. */
import {
  GEAR_SLOTS, ITEMS, KIND_FOR_SLOT, MOD_SLOTS, WEAPONS, starterLoadout, validateLoadout,
  type EquippedWeapon, type GearSlot, type Item, type Loadout,
} from "@crowned-dogs/core";

export { GEAR_SLOTS, GEAR_SLOT_NAMES, describeItem, compileKit } from "@crowned-dogs/core";
export type { GearSlot, Item, Loadout } from "@crowned-dogs/core";

export const LOADOUT_STORAGE_KEY = "crowned-dogs:loadout:v1";

/** A small interface also usable by tests and embedders without browser storage. */
export interface LoadoutStorage {
  getItem(key: string): string | null;
  setItem(key: string, value: string): void;
}

function browserStorage(): LoadoutStorage | null {
  try { return globalThis.localStorage ?? null; } catch { return null; }
}

function record(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function weaponShape(value: unknown): value is EquippedWeapon {
  if (!record(value) || typeof value.item !== "string") return false;
  if (value.skin !== undefined && typeof value.skin !== "string") return false;
  if (value.mods !== undefined) {
    if (!record(value.mods)) return false;
    if (!Object.entries(value.mods).every(([slot, id]) =>
      MOD_SLOTS.some((known) => known === slot) && typeof id === "string")) return false;
  }
  return true;
}

/** Check untrusted saved JSON before the core validator accesses nested fields. */
export function isValidLoadout(value: unknown): value is Loadout {
  if (!record(value) || !weaponShape(value.weapon1)) return false;
  if (value.weapon2 !== null && !weaponShape(value.weapon2)) return false;
  for (const slot of GEAR_SLOTS) {
    if (slot !== "weapon1" && slot !== "weapon2" && typeof value[slot] !== "string") return false;
  }
  try { return validateLoadout(value as unknown as Loadout) === null; } catch { return false; }
}

function cloneWeapon(weapon: EquippedWeapon): EquippedWeapon {
  return {
    item: weapon.item,
    ...(weapon.mods === undefined ? {} : { mods: { ...weapon.mods } }),
    ...(weapon.skin === undefined ? {} : { skin: weapon.skin }),
  };
}

function cloneLoadout(loadout: Loadout): Loadout {
  return {
    weapon1: cloneWeapon(loadout.weapon1),
    weapon2: loadout.weapon2 === null ? null : cloneWeapon(loadout.weapon2),
    head: loadout.head, body: loadout.body, suit: loadout.suit, boots: loadout.boots,
    knife: loadout.knife, grenade: loadout.grenade,
  };
}

/** No storage, blocked storage or invalid saves all produce a usable starter kit. */
export function loadLocalLoadout(storage: LoadoutStorage | null = browserStorage()): Loadout {
  try {
    const raw = storage?.getItem(LOADOUT_STORAGE_KEY);
    if (raw) {
      const saved: unknown = JSON.parse(raw);
      if (record(saved) && saved.version === 1 && isValidLoadout(saved.loadout)) {
        return cloneLoadout(saved.loadout);
      }
    }
  } catch { /* Private browsing, unavailable storage and malformed JSON are recoverable. */ }
  return starterLoadout("rifle");
}

/** False means selections remain in memory; callers can explain that saving failed. */
export function saveLocalLoadout(loadout: Loadout, storage: LoadoutStorage | null = browserStorage()): boolean {
  if (!storage || !isValidLoadout(loadout)) return false;
  try {
    storage.setItem(LOADOUT_STORAGE_KEY, JSON.stringify({ version: 1, loadout: cloneLoadout(loadout) }));
    return true;
  } catch { return false; }
}

/** Full available catalogue, not an ownership claim. Offhand obeys the core hands rule. */
export function itemsForSlot(slot: GearSlot): Item[] {
  if (slot === "weapon1" || slot === "weapon2") {
    return ITEMS.filter((item) => item.kind === "weapon" &&
      (slot === "weapon1" || WEAPONS[item.weapon].hands === 1));
  }
  return ITEMS.filter((item) => item.kind === KIND_FOR_SLOT[slot]);
}

export function equippedItemId(loadout: Loadout, slot: GearSlot): string | null {
  if (slot === "weapon1") return loadout.weapon1.item;
  if (slot === "weapon2") return loadout.weapon2?.item ?? null;
  return loadout[slot];
}

/** Returns an independent validated kit. Only the optional second weapon can be removed. */
export function equipItem(loadout: Loadout, slot: GearSlot, itemId: string | null): Loadout {
  if (!isValidLoadout(loadout)) throw new Error("Некорректный комплект снаряжения");
  if (!GEAR_SLOTS.includes(slot)) throw new Error("Неизвестный слот снаряжения");
  if (itemId === null && slot !== "weapon2") throw new Error("Этот слот нельзя оставить пустым");
  if (itemId !== null && !itemsForSlot(slot).some((item) => item.id === itemId)) {
    throw new Error("Предмет не подходит для этого слота");
  }
  const next = cloneLoadout(loadout);
  if (slot === "weapon2" && itemId === null) next.weapon2 = null;
  else if (slot === "weapon1" || slot === "weapon2") {
    // Reselecting preserves parts; changing the weapon starts with its factory parts.
    if (equippedItemId(next, slot) !== itemId) next[slot] = { item: itemId! };
  } else next[slot] = itemId!;
  if (validateLoadout(next)) throw new Error("Предмет несовместим с комплектом");
  return next;
}
