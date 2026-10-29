// Browser/Node ABI only. All game calculations are implemented in crates/game-core.
import { callRust } from "../../runtime/src/index.ts";
import type * as Contract from "../reference/ai.ts";
export type * from "../reference/ai.ts";
const constants = callRust<Record<string, unknown>>("core.constants", []);
export const DEFAULT_BOT = constants.DEFAULT_BOT as typeof Contract.DEFAULT_BOT;
export const botRng: typeof Contract.botRng = (...args) => callRust("core.botRng", args);
export const botOrder: typeof Contract.botOrder = (...args) => {
  if (!args[2]) return callRust("core.botOrder", args);
  const result = callRust<{value: ReturnType<typeof Contract.botOrder>; state: number[]}>("core.botOrder", args);
  args[2].splice(0, 4, ...result.state);
  return result.value;
};
