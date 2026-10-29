/** Browser deck adapter; the complete music state machine lives in Rust. */
import { RustMusicDirector } from "../../runtime/src/index.ts";
import { RADIO_PLAYLIST, type SoundId, type Station, type TrackId } from "./bank.ts";
import type { Random } from "./pick.ts";
export type Channel = "fight" | "radio";
export type Scene = "silent" | "menu" | "fight";
export interface MusicDeck {
  play(channel: Channel, track: TrackId): void;
  pause(channel: Channel): void;
  resume(channel: Channel): void;
  stop(channel: Channel): void;
  cue(sound: SoundId): void;
}
interface State { scene: Scene; menuOpen: boolean; fightTrack?: TrackId; radioTrack?: Station; fightPaused: boolean }
type Operation = { method: "play"; args: [Channel, TrackId] } | { method: "cue"; args: [SoundId] } | { method: "pause" | "resume" | "stop"; args: [Channel] };
export class MusicDirector {
  private runtime: RustMusicDirector;
  private host: { deck: MusicDeck; random: Random };
  constructor(deck: MusicDeck, random: Random = Math.random) {
    this.host = { deck, random };
    this.runtime = new RustMusicDirector(this.host, RADIO_PLAYLIST);
  }
  get state(): State { return this.runtime.state; }
  enterMenu(): void { this.runtime.send("enterMenu", null); }
  enterFight(): void { this.runtime.send("enterFight", null); }
  openMenu(): void { this.runtime.send("openMenu", null); }
  closeMenu(): void { this.runtime.send("closeMenu", null); }
  silence(): void { this.runtime.send("silence", null); }
  ended(channel: Channel): void { this.runtime.send("ended", channel); }
  nextStation(): void { this.runtime.send("nextStation", null); }
  tune(station: Station): void { this.runtime.send("tune", station); }
  playlist(list: readonly Station[]): void { this.runtime.playlist(list); }
}
