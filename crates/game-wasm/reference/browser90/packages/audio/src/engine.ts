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
export class GameAudio {
  private readonly controller: BrowserGameAudio;
  constructor(opts: GameAudioOptions) {
    this.controller = new BrowserGameAudio(opts, { FILES, MIX, SOUNDS, TRACKS, MusicDirector, createRadio });
  }
  get ctx(): AudioContext { return this.controller.ctx as AudioContext; }
  get music(): MusicDirector { return this.controller.music as MusicDirector; }
  unlock(): Promise<void> { return this.controller.unlock(); }
  setVolume(bus: Bus | "master", value: number): void { this.controller.setVolume(bus, value); }
  url(file: string): string { return this.controller.url(file); }
  preload(ids: SoundId[]): Promise<void> { return this.controller.preload(ids); }
  play(id: SoundId, options: PlayOptions = {}): void { this.controller.play(id, options); }
  playCues(cues: Cue[], startIn = 0.05): void { this.controller.playCues(cues, startIn); }
  setAmbience(ids: SoundId[], fade = 2): void { this.controller.setAmbience(ids, fade); }
  setFightMusic(value: number): void { this.controller.setFightMusic(value); }
  placeRadio(pan: number, far: number): void { this.controller.placeRadio(pan, far); }
  musicBus(): AudioNode { return this.controller.musicBus() as AudioNode; }
  gainNode(db: number, to: AudioNode): GainNode { return this.controller.gainNode(db, to) as GainNode; }
  decode(file: string): Promise<AudioBuffer> { return this.controller.decode(file); }
}
