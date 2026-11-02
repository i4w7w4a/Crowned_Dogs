import { callRust } from "../../runtime/src/index.ts";
import type { Shape } from "./draw.ts";
export type WeaponArtId = "pistol" | "shotgun" | "rifle" | "sniper";
export type Slot = "optic" | "muzzle" | "rail" | "mag" | "stock";
type Units = "rifle" | "pistol";
export interface WeaponArt { units: Units; body: Shape; anchors: Record<Slot, [number, number]> }
export interface PartArt { units: Units; mount: Shape; icon?: Shape }
const catalog = callRust<{ weapons: Record<WeaponArtId, WeaponArt>; parts: Record<string, PartArt>; wear: Record<string, Shape> }>("icons.catalog", []);
export const WEAPON_ART = catalog.weapons;
export const PART_ART = catalog.parts;
export const WEAR_ART = catalog.wear;
/** Entries preserve the caller's painter order at the Rust boundary. */
export const composeWeapon = (weapon: WeaponArtId, parts: Record<Slot, string>): Shape => callRust("icons.composeWeapon", [weapon, Object.entries(parts)]);
export const itemArt = (id: string, stockParts?: Record<WeaponArtId, Record<Slot, string>>): Shape | null =>
  callRust("icons.itemArt", [id, stockParts ? Object.entries(stockParts).map(([weapon, parts]) => [weapon, Object.entries(parts)]) : null]);
