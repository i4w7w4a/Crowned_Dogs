/** Declarative audio catalogue and mix values are embedded in the Rust engine. */
import { callRust } from "../../runtime/src/index.ts";
export type Bus = "sfx" | "ui" | "amb" | "music";
export interface SoundDef { files: string[]; bus: Bus; gainDb?: number; pitchJitter?: number; gainJitter?: number; duckDb?: number; voice?: true }
export interface TrackDef { file: string; title: string }
export type SoundId = "shot.pistol.near" | "shot.pistol.far" | "shot.shotgun.near" | "shot.shotgun.far" | "shot.rifle.near" | "shot.rifle.far" | "shot.sniper.near" | "shot.sniper.far" | "casing" | "casing.steel" | "knife.swing" | "knife.hit" | "knife.blocked" | "hit.armor" | "hit.suit" | "hit.head" | "hit.crit" | "shield.block" | "shield.break" | "miss.near" | "miss.far" | "grenade.pin" | "grenade.throw" | "grenade.bounce" | "grenade.blast" | "step" | "dash.whoosh" | "gear.rustle" | "body.fall" | "persona.powerdown" | "persona.powerup" | "sync.lost" | "ui.tap" | "ui.guard" | "ui.step" | "ui.confirm" | "ui.tick" | "ui.tick.urgent" | "ui.open" | "ui.close" | "ui.turn" | "ui.error" | "ui.match.found" | "ui.victory" | "ui.defeat" | "loot.water" | "loot.scrap" | "equip.pistol" | "equip.shotgun" | "equip.rifle" | "equip.sniper" | "equip.knife" | "equip.grenade" | "equip.armor" | "equip.helmet" | "equip.suit" | "equip.boots" | "equip.chevron" | "radio.on" | "radio.off" | "radio.tune" | "thunder" | "helicopter.flyby" | "artillery.far" | "artillery.incoming" | "amb.hall" | "amb.blocks" | "amb.storm" | "amb.dust" | "amb.fog" | "amb.base" | "amb.workshop" | "voice.sedoy.1" | "voice.sedoy.2" | "voice.sedoy.3" | "voice.sedoy.4" | "voice.sedoy.5" | "voice.sedoy.6";
export type TrackId = "the-limbo-3" | "404-room" | "in-the-middle-of-nowhere" | "crazy-2" | "fxck-wag-6" | "fog-3" | "try-to-fake";
export type Station = "crazy-2" | "fxck-wag-6" | "fog-3" | "try-to-fake";
interface AudioData {
  SOUNDS: Record<SoundId, SoundDef>; TRACKS: Record<TrackId, TrackDef>;
  FIGHT_PLAYLIST: TrackId[]; RADIO_PLAYLIST: readonly Station[];
  BASE_PLAYLIST: readonly Station[]; FITTING_PLAYLIST: readonly Station[];
  MIX: { master: number; sfx: number; ui: number; amb: number; music: number; fightMusic: number; radioMusic: number; distanceDbPerStep: number; distanceDbMax: number };
}
const data = callRust<AudioData>("client.data", []);
export const { SOUNDS, TRACKS, FIGHT_PLAYLIST, RADIO_PLAYLIST, BASE_PLAYLIST, FITTING_PLAYLIST, MIX } = data;
