// Browser/Node ABI only. All game calculations are implemented in crates/game-core.
import { callRust } from "../../runtime/src/index.ts";
import type * as Contract from "../reference/items.ts";
export type * from "../reference/items.ts";
const constants = callRust<Record<string, unknown>>("core.constants", []);
export const TIER_NAMES = constants.TIER_NAMES as typeof Contract.TIER_NAMES;
export const GEAR_SLOTS = constants.GEAR_SLOTS as typeof Contract.GEAR_SLOTS;
export const GEAR_SLOT_NAMES = constants.GEAR_SLOT_NAMES as typeof Contract.GEAR_SLOT_NAMES;
export const MOD_SLOTS = constants.MOD_SLOTS as typeof Contract.MOD_SLOTS;
export const MOD_SLOT_NAMES = constants.MOD_SLOT_NAMES as typeof Contract.MOD_SLOT_NAMES;
export const SMOKE_PENALTY = constants.SMOKE_PENALTY as typeof Contract.SMOKE_PENALTY;
export const ITEMS = constants.ITEMS as typeof Contract.ITEMS;
export const ITEM_BY_ID: typeof Contract.ITEM_BY_ID = new Map(ITEMS.map(item => [item.id, item]));
export const KIND_FOR_SLOT = constants.KIND_FOR_SLOT as typeof Contract.KIND_FOR_SLOT;
export const getItem: typeof Contract.getItem = (...args) => callRust("core.getItem", args);
export const modsFor: typeof Contract.modsFor = (...args) => callRust("core.modsFor", args);
export const weaponItem: typeof Contract.weaponItem = (...args) => callRust("core.weaponItem", args);
