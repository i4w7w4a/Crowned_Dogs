// Browser/Node ABI only. All game calculations are implemented in crates/game-core.
import { callRust } from "../../runtime/src/index.ts";
import type * as Contract from "../reference/rng.ts";
export type * from "../reference/rng.ts";
export const seedRng: typeof Contract.seedRng = (...args) => callRust("core.seedRng", args);
export const nextFloat = ((...args: Parameters<typeof Contract.nextFloat>) => {
  const result = callRust<{value: unknown; state: number[]}>("core.nextFloat", args);
  args[0].splice(0, 4, ...result.state);
  return result.value;
}) as typeof Contract.nextFloat;
export const roll = ((...args: Parameters<typeof Contract.roll>) => {
  const result = callRust<{value: unknown; state: number[]}>("core.roll", args);
  args[0].splice(0, 4, ...result.state);
  return result.value;
}) as typeof Contract.roll;
export const nextInt = ((...args: Parameters<typeof Contract.nextInt>) => {
  const result = callRust<{value: unknown; state: number[]}>("core.nextInt", args);
  args[0].splice(0, 4, ...result.state);
  return result.value;
}) as typeof Contract.nextInt;
export const pickWeighted = ((...args: Parameters<typeof Contract.pickWeighted>) => {
  const result = callRust<{value: unknown; state: number[]}>("core.pickWeighted", args);
  args[0].splice(0, 4, ...result.state);
  return result.value;
}) as typeof Contract.pickWeighted;
