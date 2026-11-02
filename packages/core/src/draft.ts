// Browser/Node ABI only. All game calculations are implemented in crates/game-core.
import { callRust } from "../../runtime/src/index.ts";
import type * as Contract from "../reference/draft.ts";
export type * from "../reference/draft.ts";
export const freshDraft: typeof Contract.freshDraft = (...args) => callRust("core.freshDraft", args);
export const grenadeRefusal: typeof Contract.grenadeRefusal = (...args) => callRust("core.grenadeRefusal", args);
export const guardsNeeded: typeof Contract.guardsNeeded = (...args) => callRust("core.guardsNeeded", args);
export const applyCommand: typeof Contract.applyCommand = (...args) => callRust("core.applyCommand", args);
export const actionOf: typeof Contract.actionOf = (...args) => callRust("core.actionOf", args);
export const missing: typeof Contract.missing = (...args) => callRust("core.missing", args);
export const toOrder: typeof Contract.toOrder = (...args) => callRust("core.toOrder", args);
export const draftChance: typeof Contract.draftChance = (...args) => callRust("core.draftChance", args);
