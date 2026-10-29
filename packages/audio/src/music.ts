/** Browser deck adapter; the complete music state machine lives in Rust. */
import { callRust } from "../../runtime/src/index.ts";
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
  private data: unknown = callRust("audio.music.new", []);
  private deck: MusicDeck;
  private random: Random;
  private radioPlaylist: readonly Station[] = RADIO_PLAYLIST;
  constructor(deck: MusicDeck, random: Random = Math.random) { this.deck = deck; this.random = random; }
  get state(): State { return callRust("audio.music.state", [this.data]); }
  private send(method: string, argument: unknown = null): void {
    const random = callRust<boolean>("audio.music.needsRandom", [this.data, method, argument]) ? this.random() : 0;
    const result = callRust<{ state: unknown; operations: Operation[] }>("audio.music.step", [this.data, method, argument, random]);
    this.data = result.state;
    for (const operation of result.operations) {
      if (operation.method === "play") this.deck.play(...operation.args);
      else if (operation.method === "cue") this.deck.cue(...operation.args);
      else this.deck[operation.method](...operation.args);
    }
  }
  enterMenu(): void { this.send("enterMenu"); }
  enterFight(): void { this.send("enterFight"); }
  openMenu(): void { this.send("openMenu"); }
  closeMenu(): void { this.send("closeMenu"); }
  silence(): void { this.send("silence"); }
  ended(channel: Channel): void { this.send("ended", channel); }
  nextStation(): void { this.send("nextStation"); }
  tune(station: Station): void { this.send("tune", station); }
  playlist(list: readonly Station[]): void {
    if (list === this.radioPlaylist) return;
    this.send("playlist", list); this.radioPlaylist = list;
  }
}
