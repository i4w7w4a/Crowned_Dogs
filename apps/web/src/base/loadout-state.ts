/** Browser persistence adapter. Validation and equipment policy execute in Rust. */
import { callRust, load_local_loadout, save_local_loadout } from "../../../../packages/runtime/src/index.ts";
import type { GearSlot, Item, Loadout, ModItem, ModSlot } from "@crowned-dogs/core";
export { GEAR_SLOTS, GEAR_SLOT_NAMES, MOD_SLOTS, MOD_SLOT_NAMES, describeItem, compileKit } from "@crowned-dogs/core";
export type { GearSlot, Item, Loadout, ModItem, ModSlot } from "@crowned-dogs/core";
export const LOADOUT_STORAGE_KEY = "crowned-dogs:loadout:v1";
export interface LoadoutStorage {
  getItem(key: string): string | null;
  setItem(key: string, value: string): void;
}
export function isValidLoadout(value: unknown): value is Loadout {
  return callRust<boolean>("client.isValidLoadout", [value]);
}
export function loadLocalLoadout(storage?: LoadoutStorage | null): Loadout { return load_local_loadout(storage); }
export function saveLocalLoadout(loadout: Loadout, storage?: LoadoutStorage | null): boolean { return save_local_loadout(loadout, storage); }
export const itemsForSlot = (slot: GearSlot): Item[] => callRust("client.itemsForSlot", [slot]);
export const equippedItemId = (loadout: Loadout, slot: GearSlot): string | null => callRust("client.equippedItemId", [loadout, slot]);
export const equipItem = (loadout: Loadout, slot: GearSlot, itemId: string | null): Loadout => callRust("client.equipItem", [loadout, slot, itemId]);
export const weaponModsForSlot = (loadout: Loadout, slot: ModSlot): ModItem[] => callRust("client.weaponModsForSlot", [loadout, slot]);
export const effectiveWeaponModId = (loadout: Loadout, slot: ModSlot): string => callRust("client.effectiveWeaponModId", [loadout, slot]);
export const equipWeaponMod = (loadout: Loadout, slot: ModSlot, itemId: string | null): Loadout => callRust("client.equipWeaponMod", [loadout, slot, itemId]);
