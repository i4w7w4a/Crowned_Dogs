/** Audio policy is shared Rust; this module supplies browser persistence and typed calls. */
import { BASE_PLAYLIST, FITTING_PLAYLIST, GameAudio, RADIO_PLAYLIST, cuesForTurn, fightSounds, type PlayOptions, type SoundId, type Station } from "@crowned-dogs/audio";
import type { Match, TurnReport } from "@crowned-dogs/core";
import { BrowserArenaAudio } from "../../../../packages/runtime/src/index.ts";
import { store } from "../store.ts";
export type VolumeKind = "fx" | "music" | "fight";
export class ArenaAudio {
  private readonly controller = new BrowserArenaAudio({
    GameAudio, BASE_PLAYLIST, FITTING_PLAYLIST, RADIO_PLAYLIST, cuesForTurn, fightSounds, store,
    silent: new URLSearchParams(location.search).get("muted") === "1",
  });
  get on(): boolean { return this.controller.on; }
  get volume(): Readonly<Record<VolumeKind, number>> { return this.controller.volume as Readonly<Record<VolumeKind, number>>; }
  get station(): Station { return this.controller.station as Station; }
  unlock(): void { this.controller.unlock(); }
  setPlace(match: Match): void { this.controller.setPlace(match); }
  screen(view: "base" | "fitting" | "weapon"): void { this.controller.screen(view); }
  newFight(match: Match): void { this.controller.newFight(match); }
  sheet(open: boolean): void { this.controller.sheet(open); }
  finish(won: boolean | null): void { this.controller.finish(won ?? undefined); }
  turn(report: TurnReport, match: Match): void { this.controller.turn(report, match); }
  play(id: SoundId, options?: PlayOptions): void { this.controller.play(id, options); }
  toggle(): void { this.controller.toggle(); }
  setVolume(kind: VolumeKind, value: number): void { this.controller.setVolume(kind, value); }
  tune(station: Station): void { this.controller.tune(station); }
}
export const gameAudio = new ArenaAudio();
