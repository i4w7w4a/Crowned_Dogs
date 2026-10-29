/** Rust owns Russian inflection and combat report wording; this file is its typed facade. */
import { callRust } from "../../../../packages/runtime/src/index.ts";
import type { Action, MatchResult, Side, TimeOfDay, TurnReport, Weather, WeaponId, Zone } from "@crowned-dogs/core";
interface WordData { ZONE_TO: Record<Zone, string>; GUN_ACC: Record<WeaponId, string>; GUN_WITH: Record<WeaponId, string>; TIME_WORD: Record<TimeOfDay, string>; WEATHER_WORD: Record<Weather, string> }
export const { ZONE_TO, GUN_ACC, GUN_WITH, TIME_WORD, WEATHER_WORD } = callRust<WordData>("client.data", []);
export type Outcome =
  | { kind: "none"; text: string }
  | { kind: "miss" }
  | { kind: "blocked"; damage: number }
  | { kind: "hit"; damage: number; crit: boolean };

/** One side's line of a turn in the log. */
export interface Line {
  icon: "shot" | "knife" | "grenade" | "swap" | "move";
  /** The move icon points back. */
  back: boolean;
  dash: boolean;
  act: string;
  outcome: Outcome;
  /** What the hit landed on, for the short ticker: a zone or "граната". */
  target: string;
}


export const plural = (n: number, one: string, few: string, many: string): string => callRust("words.plural", [n, one, few, many]);
export const steps = (n: number): string => callRust("words.steps", [n]);
export const turnsWord = (n: number): string => callRust("words.turnsWord", [n]);
export const moveWord = (action: Action): string => callRust("words.moveWord", [action]);
export const zoneName = (zone: Zone): string => callRust("words.zoneName", [zone]);
export const turnLine = (report: TurnReport, side: Side, inHand: WeaponId): Line => callRust("words.turnLine", [report, side, inHand]);
export const endWords = (result: MatchResult, me: Side, foe: string, turns: number): { title: string; text: string; again: string; lost: boolean } => callRust("words.endWords", [result, me, foe, turns]);
