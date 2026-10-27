// Which song plays when. Art's rules (2026-10-27):
// - a fight plays The Limbo, 404 Room or In The Middle of Nowhere, picked at random;
// - menus and the gear screen play the other tracks from an in-world radio, quietly;
// - opening the menu during a fight pauses the fight track; closing it resumes.

import { FIGHT_PLAYLIST, RADIO_PLAYLIST, type SoundId, type TrackId } from "./bank.ts";
import { Shuffle, type Random } from "./pick.ts";

export type Channel = "fight" | "radio";

/** What actually makes the sound; the browser engine implements it, tests fake it. */
export interface MusicDeck {
  /** Start a track from the beginning. */
  play(channel: Channel, track: TrackId): void;
  /** Fade out and hold the position. */
  pause(channel: Channel): void;
  /** Fade back in from the held position. */
  resume(channel: Channel): void;
  stop(channel: Channel): void;
  cue(sound: SoundId): void;
}

export type Scene = "silent" | "menu" | "fight";

export class MusicDirector {
  private deck: MusicDeck;
  private fightList: Shuffle<TrackId>;
  private radioList: Shuffle<TrackId>;
  private scene: Scene = "silent";
  private menuOpen = false;
  private radioOn = false;
  private fightPaused = false;

  constructor(deck: MusicDeck, random: Random = Math.random) {
    this.deck = deck;
    this.fightList = new Shuffle(FIGHT_PLAYLIST, random);
    this.radioList = new Shuffle(RADIO_PLAYLIST, random);
  }

  get state() {
    return {
      scene: this.scene,
      menuOpen: this.menuOpen,
      fightTrack: this.fightList.now,
      radioTrack: this.radioOn ? this.radioList.now : undefined,
      fightPaused: this.fightPaused,
    };
  }

  /** Main menu, gear, results: the radio plays, no fight music. */
  enterMenu() {
    if (this.scene === "fight") this.deck.stop("fight");
    this.scene = "menu";
    this.menuOpen = false;
    this.fightPaused = false;
    this.radioUp();
  }

  /** A new fight: a fresh random fight track from the start. */
  enterFight() {
    this.radioDown();
    this.scene = "fight";
    this.menuOpen = false;
    this.fightPaused = false;
    this.deck.play("fight", this.fightList.next());
  }

  /** The menu sheet over a fight. */
  openMenu() {
    if (this.scene !== "fight" || this.menuOpen) return;
    this.menuOpen = true;
    this.fightPaused = true;
    this.deck.pause("fight");
    this.radioUp();
  }

  closeMenu() {
    if (this.scene !== "fight" || !this.menuOpen) return;
    this.menuOpen = false;
    this.radioDown();
    this.fightPaused = false;
    this.deck.resume("fight");
  }

  silence() {
    this.deck.stop("fight");
    this.radioDown();
    this.scene = "silent";
    this.menuOpen = false;
    this.fightPaused = false;
  }

  /** The deck reports that a track played to its end. */
  ended(channel: Channel) {
    if (channel === "fight" && this.scene === "fight" && !this.fightPaused) {
      this.deck.play("fight", this.fightList.next());
    } else if (channel === "radio" && this.radioOn) {
      this.deck.cue("radio.tune");
      this.deck.play("radio", this.radioList.next());
    }
  }

  /** The next station, as if someone turned the knob. */
  nextStation() {
    if (!this.radioOn) return;
    this.deck.cue("radio.tune");
    this.deck.play("radio", this.radioList.next());
  }

  private radioUp() {
    if (this.radioOn) return;
    this.radioOn = true;
    this.deck.cue("radio.on");
    // pick up the station where it was, like a real radio left on
    if (this.radioList.now === undefined) this.deck.play("radio", this.radioList.next());
    else this.deck.resume("radio");
  }

  private radioDown() {
    if (!this.radioOn) return;
    this.radioOn = false;
    this.deck.cue("radio.off");
    this.deck.pause("radio");
  }
}
