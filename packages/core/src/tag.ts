// Browser/Node ABI only. All game calculations are implemented in crates/game-core.
import { callRust } from "../../runtime/src/index.ts";
import type * as Contract from "../reference/tag.ts";
export type * from "../reference/tag.ts";
const constants = callRust<Record<string, unknown>>("core.constants", []);
export const TAG_COUNT = constants.TAG_COUNT as typeof Contract.TAG_COUNT;
export const formatTag: typeof Contract.formatTag = (...args) => {
  try { return callRust("core.formatTag", args); }
  catch (e) { throw new RangeError(e instanceof Error ? e.message : String(e)); }
};
export const parseTag: typeof Contract.parseTag = (...args) => callRust("core.parseTag", args);
