/** Sound scheduling is Rust policy; WebAudio only plays this returned script. */
import { callRust } from "../../runtime/src/index.ts";
import type { Side, TurnReport, WeaponId, Zone } from "@crowned-dogs/core";
import type { SoundId } from "./bank.ts";
export interface Cue { sound: SoundId; at: number; gainDb: number; pan: number; wetDb: number; cutHz?: number }
export interface TurnSoundContext {
  viewer: Side;
  armor?: [Partial<Record<Zone, number>>, Partial<Record<Zone, number>>];
  weapons?: [WeaponId, WeaponId];
  /** Stereo place of each side as the arena camera sees it; fixed defaults without it. */
  pan?: [number, number];
  /** "hall" rings the casings on the steel deck; open-air maps drop them on concrete. */
  room?: "hall" | "blocks";
}
export const TIMING = callRust<{ TIMING: {
  move: number; ourAttack: number; enemyAttack: number; stepGap: number; dashStepGap: number;
  knifeLands: number; grenadeBounce: number; grenadeBlast: number; fallAfterLastAttack: number;
} }>("client.data", []).TIMING;
export const fightSounds = (ours: WeaponId | readonly WeaponId[], theirs: WeaponId | readonly WeaponId[]): SoundId[] => callRust("audio.fightSounds", [ours, theirs]);
export const cuesForTurn = (report: TurnReport, ctx: TurnSoundContext): Cue[] => callRust("audio.cuesForTurn", [report, ctx]);
