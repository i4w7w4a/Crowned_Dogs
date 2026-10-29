// Browser/Node ABI only. All game calculations are implemented in crates/game-core.
import { callRust } from "../../runtime/src/index.ts";
import type * as Contract from "../reference/match.ts";
export type * from "../reference/match.ts";
export const other: typeof Contract.other = (...args) => callRust("core.other", args);
export const createMatch: typeof Contract.createMatch = (...args) => callRust("core.createMatch", args);
export const weaponSpec: typeof Contract.weaponSpec = (...args) => {
  // Rust chooses the part; the bridge preserves the caller's object identity.
  const field = callRust<"weapon" | "sidearm" | "knife" | null>("core.weaponSpecSource", args);
  return field ? args[0].config.fighters[args[1]].kit![field]! : callRust("core.weaponSpec", args);
};
export const swapWeapon: typeof Contract.swapWeapon = (...args) => callRust("core.swapWeapon", args);
export const grenadeSpec: typeof Contract.grenadeSpec = (...args) => callRust("core.grenadeSpec", args);
export const grenadeHit: typeof Contract.grenadeHit = (...args) => callRust("core.grenadeHit", args);
export const guardShare: typeof Contract.guardShare = (...args) => callRust("core.guardShare", args);
export const turnLimit: typeof Contract.turnLimit = (...args) => callRust("core.turnLimit", args);
export const turnSeconds: typeof Contract.turnSeconds = (...args) => callRust("core.turnSeconds", args);
export const distanceLimit: typeof Contract.distanceLimit = (...args) => callRust("core.distanceLimit", args);
export const moveOf: typeof Contract.moveOf = (...args) => callRust("core.moveOf", args);
export const defendCount: typeof Contract.defendCount = (...args) => callRust("core.defendCount", args);
export const plannedDistance: typeof Contract.plannedDistance = (...args) => callRust("core.plannedDistance", args);
export const canFlee: typeof Contract.canFlee = (...args) => callRust("core.canFlee", args);
export const canThrowGrenade: typeof Contract.canThrowGrenade = (...args) => callRust("core.canThrowGrenade", args);
export const canUseKnife: typeof Contract.canUseKnife = (...args) => callRust("core.canUseKnife", args);
export const canSwap: typeof Contract.canSwap = (...args) => callRust("core.canSwap", args);
export const validateOrder: typeof Contract.validateOrder = (...args) => callRust("core.validateOrder", args);
export const conditionAccuracy: typeof Contract.conditionAccuracy = (...args) => callRust("core.conditionAccuracy", args);
export const hitChance: typeof Contract.hitChance = (...args) => callRust("core.hitChance", args);
export const critChance: typeof Contract.critChance = (...args) => callRust("core.critChance", args);
export const expectedHit: typeof Contract.expectedHit = (...args) => callRust("core.expectedHit", args);
export const resolveTurn: typeof Contract.resolveTurn = (...args) => callRust("core.resolveTurn", args);
