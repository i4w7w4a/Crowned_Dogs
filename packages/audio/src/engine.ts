/// <reference lib="dom" />
/** Typed Web Audio boundary. Graphs, decoding, scheduling and deck lifecycle run in Rust. */
import { BrowserGameAudio } from "../../runtime/src/index.ts";
import { FILES } from "./files.generated.ts";
import { MIX, SOUNDS, TRACKS, type Bus, type SoundId } from "./bank.ts";
import type { Cue } from "./cues.ts";
import { MusicDirector } from "./music.ts";
import type { Random } from "./pick.ts";
import { createRadio } from "./radio.ts";
export interface GameAudioOptions { baseUrl: string; random?: Random }
export interface PlayOptions { delay?: number; gainDb?: number; pan?: number }
export class GameAudio extends BrowserGameAudio {
  declare readonly ctx: AudioContext;
  declare readonly music: MusicDirector;
  constructor(opts: GameAudioOptions) { super(opts, { FILES, MIX, SOUNDS, TRACKS, MusicDirector, createRadio }); }
  override play(id: SoundId, options: PlayOptions = {}): void { super.play(id, options); }
  override playCues(cues: Cue[], startIn = 0.05): void { super.playCues(cues, startIn); }
  override setAmbience(ids: SoundId[], fade = 2): void { super.setAmbience(ids, fade); }
}
