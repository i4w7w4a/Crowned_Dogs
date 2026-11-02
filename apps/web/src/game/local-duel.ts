/** Typed facade over the Rust local-duel state machine. */
import { callRust, RustLocalDuel } from "../../../../packages/runtime/src/index.ts";
import type { Command, Draft, Loadout, Match, Side, TimeOfDay, TurnReport, Weather, WeaponId } from "@crowned-dogs/core";
export interface LocalFighter { name: string; weapon: WeaponId; kit?: Loadout; tag?: string }
export interface LocalDuelOptions {
  seed?: string; mapId?: string; weather?: Weather; time?: TimeOfDay; startDistance?: number;
  player: LocalFighter; bot: LocalFighter;
}
export const MOVE_MIN = -2;
export const MOVE_MAX = 3;
export const moveCommand = (move: number): Command => callRust("client.moveCommand", [move]);
export class LocalDuel extends RustLocalDuel {
  constructor(opts: LocalDuelOptions) { super(opts); }
  declare readonly me: Side;
  declare readonly bot: Side;
  declare readonly fighters: [LocalFighter, LocalFighter];
  declare readonly match: Match;
  declare readonly draft: Draft;
}
