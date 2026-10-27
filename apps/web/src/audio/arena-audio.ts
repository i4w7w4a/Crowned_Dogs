// The fight screen's sound: the audio package's engine and music rules, plus what
// only this screen knows (which loops fit the painted frame, the on/off switch,
// the station picked in the menu). Browsers start sound only after a tap, so the
// audio graph is built on the first one; until then calls only remember the state.

import {
  GameAudio, RADIO_PLAYLIST, cuesForTurn, fightSounds, type PlayOptions, type SoundId, type Station,
} from "@crowned-dogs/audio";
import type { Match, TurnReport, WeaponId } from "@crowned-dogs/core";
import { store } from "../store.ts";

const UI: SoundId[] = ["ui.tap", "ui.guard", "ui.step", "ui.confirm", "ui.tick", "ui.tick.urgent", "ui.turn", "ui.victory", "ui.defeat"];
/** A location bed per map, and a layer for the weather on top. */
const BED: Record<string, SoundId> = { saltHall: "amb.hall", blocks: "amb.blocks" };
const WEATHER_LAYER: Partial<Record<Match["config"]["weather"], SoundId>> = { dust: "amb.dust", storm: "amb.storm", fog: "amb.fog" };
/** The hall goes quieter behind the menu: about −8 dB (the amb slider is squared). */
const AMB_IN_MENU = 0.62;

/** Every gun a fighter can hold in this match: the main one and a sidearm. */
function guns(m: Match, side: 0 | 1): WeaponId[] {
  const kit = m.config.fighters[side].kit;
  return kit?.sidearm ? [kit.weapon.id as WeaponId, kit.sidearm.id as WeaponId] : [m.fighters[side].weapon];
}

export class ArenaAudio {
  on = store.get("cd-music") !== "off";
  private picked: Station = (RADIO_PLAYLIST as readonly string[]).includes(store.get("cd-station") ?? "")
    ? store.get("cd-station") as Station : RADIO_PLAYLIST[0];
  private audio: GameAudio | null = null;
  private ambience: SoundId[] = [];
  private preloads: SoundId[] = [...UI];
  /** fight: the fight track; menu: the radio (end screen); paused: the menu sheet over a fight. */
  private mode: "fight" | "menu" | "paused" = "fight";

  /** Call from the first tap. */
  unlock(): void {
    if (this.audio) { void this.audio.unlock(); return; }
    try {
      this.audio = new GameAudio({ baseUrl: "./" });
    } catch {
      return; // no Web Audio: a silent fight
    }
    const a = this.audio;
    void a.unlock();
    a.setVolume("master", this.on ? 1 : 0);
    a.music.tune(this.picked);
    a.setAmbience(this.ambience);
    void a.preload(this.preloads).catch(() => {});
    if (this.mode === "menu") a.music.enterMenu();
    else {
      a.music.enterFight();
      if (this.mode === "paused") a.music.openMenu();
    }
    this.ambLevel();
  }

  /** The painted frame's place and weather. */
  setPlace(m: Match): void {
    const bed = BED[m.config.mapId] ?? "amb.hall", layer = WEATHER_LAYER[m.config.weather];
    this.ambience = layer ? [bed, layer] : [bed];
    this.audio?.setAmbience(this.ambience);
  }

  /** A new fight: a fresh fight track, and the sounds of both fighters' guns. */
  newFight(m: Match): void {
    this.preloads = [...UI, ...fightSounds(guns(m, 0), guns(m, 1))];
    this.setPlace(m);
    this.mode = "fight";
    if (!this.audio) return;
    void this.audio.preload(this.preloads).catch(() => {});
    this.audio.music.enterFight();
    this.ambLevel();
  }

  /** The menu sheet: pauses the fight track and turns the radio on; closing it brings the fight back. */
  sheet(open: boolean): void {
    if (this.mode === "menu") return;
    this.mode = open ? "paused" : "fight";
    if (open) this.audio?.music.openMenu();
    else this.audio?.music.closeMenu();
    this.ambLevel();
  }

  /** The fight is over: the radio plays under the end screen. */
  finish(won: boolean | null): void {
    if (won !== null) this.play(won ? "ui.victory" : "ui.defeat", { delay: 0.9 });
    this.mode = "menu";
    this.audio?.music.enterMenu();
    this.ambLevel();
  }

  /** A played turn as the rules report it, after the turn: `m` holds the guns now in hand. */
  turn(r: TurnReport, m: Match): void {
    if (!this.audio) return;
    const [a, b] = m.fighters;
    this.audio.playCues(cuesForTurn(r, { viewer: 0, armor: [a.armor, b.armor], weapons: [a.weapon, b.weapon] }), 0);
  }

  play(id: SoundId, o?: PlayOptions): void {
    this.audio?.play(id, o);
  }

  toggle(): void {
    this.on = !this.on;
    store.set("cd-music", this.on ? "on" : "off");
    this.unlock();
    this.audio?.setVolume("master", this.on ? 1 : 0);
  }

  /** The station on the radio now (it plays on to the next track by itself). */
  get station(): Station {
    return (this.audio?.music.state.radioTrack as Station | undefined) ?? this.picked;
  }

  tune(s: Station): void {
    this.picked = s;
    store.set("cd-station", s);
    if (!this.on) this.toggle();
    this.unlock();
    this.audio?.music.tune(s);
  }

  private ambLevel(): void {
    this.audio?.setVolume("amb", this.mode === "fight" ? 1 : AMB_IN_MENU);
  }
}
