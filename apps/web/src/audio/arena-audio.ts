/** Audio policy is shared Rust; this module supplies browser persistence and typed calls. */
import { BASE_PLAYLIST, FITTING_PLAYLIST, GameAudio, RADIO_PLAYLIST, cuesForTurn, fightSounds, type PlayOptions, type SoundId, type Station } from "@crowned-dogs/audio";
import { BrowserArenaAudio } from "../../../../packages/runtime/src/index.ts";
import { store } from "../store.ts";
export type VolumeKind = "fx" | "music" | "fight";
export class ArenaAudio extends BrowserArenaAudio {
  declare readonly volume: Readonly<Record<VolumeKind, number>>;
  declare readonly station: Station;
  constructor() { super({ GameAudio, BASE_PLAYLIST, FITTING_PLAYLIST, RADIO_PLAYLIST, cuesForTurn, fightSounds, store,
    silent: new URLSearchParams(location.search).get("muted") === "1" }); }
  override finish(won: boolean | null): void { super.finish(won ?? undefined); }
}
export const gameAudio = new ArenaAudio();
