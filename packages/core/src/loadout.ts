// Browser/Node ABI only. All game calculations are implemented in crates/game-core.
import { callRust } from "../../runtime/src/index.ts";
import type * as Contract from "../reference/loadout.ts";
export type * from "../reference/loadout.ts";
const constants = callRust<Record<string, unknown>>("core.constants", []);
export const STARTER = constants.STARTER as typeof Contract.STARTER;
export const VETERAN_KITS = constants.VETERAN_KITS as typeof Contract.VETERAN_KITS;
export const kitFighter: typeof Contract.kitFighter = (...args) => callRust("core.kitFighter", args);
export const starterLoadout: typeof Contract.starterLoadout = (...args) => callRust("core.starterLoadout", args);
export const validateLoadout: typeof Contract.validateLoadout = (...args) => callRust("core.validateLoadout", args);
export const weaponParts: typeof Contract.weaponParts = (...args) => callRust("core.weaponParts", args);
export const applyWeaponEffects: typeof Contract.applyWeaponEffects = (...args) => callRust("core.applyWeaponEffects", args);
export const compileKit: typeof Contract.compileKit = (...args) => callRust("core.compileKit", args);
export const effectLines: typeof Contract.effectLines = (...args) => callRust("core.effectLines", args);
export const describeEffects: typeof Contract.describeEffects = (...args) => callRust("core.describeEffects", args);
export const describeGrenade: typeof Contract.describeGrenade = (...args) => callRust("core.describeGrenade", args);
export const describeItem: typeof Contract.describeItem = (...args) => callRust("core.describeItem", args);
