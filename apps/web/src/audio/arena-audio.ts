// The fight screen's sound: the audio package's engine and music rules, plus what
// only this screen knows (which loops fit the painted frame, the on/off switch,
// the station picked in the menu). Browsers start sound only after a tap, so the
// audio graph is built on the first one; until then calls only remember the state.

import {
  BASE_PLAYLIST, FITTING_PLAYLIST, GameAudio, RADIO_PLAYLIST, cuesForTurn, fightSounds, type PlayOptions, type SoundId, type Station,
} from "@crowned-dogs/audio";
import type { Match, TurnReport, WeaponId } from "@crowned-dogs/core";
import { store } from "../store.ts";

const UI: SoundId[] = ["ui.tap", "ui.guard", "ui.step", "ui.confirm", "ui.tick", "ui.tick.urgent", "ui.turn", "ui.victory", "ui.defeat"];
/** A location bed per map, and a layer for the weather on top. */
const BED: Record<string, SoundId> = { saltHall: "amb.hall", blocks: "amb.blocks" };
const WEATHER_LAYER: Partial<Record<Match["config"]["weather"], SoundId>> = { dust: "amb.dust", storm: "amb.storm", fog: "amb.fog" };
/** The hall goes quieter behind the menu: about −8 dB (the amb slider is squared). */
const AMB_IN_MENU = 0.62;
/** Where the radio stands on each screen (Art): the base close on the left, the fitting room far on the right. */
type Spot = [pan: number, far: number];
const SPOT_BASE: Spot = [-0.55, 0], SPOT_FITTING: Spot = [0.6, 1], SPOT_FIGHT: Spot = [0, 0.4];
const SCREEN_AMB: Record<"base" | "fitting" | "weapon", SoundId[]> = { base: ["amb.base"], fitting: ["amb.hall"], weapon: ["amb.hall"] };

/** The two volume sliders in the menu: the fight's sounds (effects, interface, ambience) and the music. */
export type VolumeKind = "fx" | "music";
const VOLUME_KEY: Record<VolumeKind, string> = { fx: "cd-vol-fx", music: "cd-vol-music" };
const savedVolume = (k: VolumeKind): number => {
  const v = Number(store.get(VOLUME_KEY[k]) ?? 1);
  return Number.isFinite(v) ? Math.min(1, Math.max(0, v)) : 1;
};

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
  private preloads: SoundId[] = [...UI];
  /** fight: the fight track; menu: the radio (base, fitting room, end screen); paused: the menu sheet over a fight. */
  private mode: "fight" | "menu" | "paused" = "menu";
  /** The stations the radio picks from on this screen. */
  private list: readonly Station[] = BASE_PLAYLIST;
  private spot: Spot = SPOT_BASE;
  private ambience: SoundId[] = SCREEN_AMB.base;
  private levels: Record<VolumeKind, number> = { fx: savedVolume("fx"), music: savedVolume("music") };

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
    this.applyLevels();
    a.music.tune(this.picked);
    a.music.playlist(this.list);
    a.placeRadio(...this.spot);
    a.setAmbience(this.ambience);
    void a.preload(this.preloads).catch(() => {});
    if (this.mode === "menu") a.music.enterMenu();
    else {
      a.music.enterFight();
      if (this.mode === "paused") a.music.openMenu();
    }
  }

  /** The painted frame's place and weather. */
  setPlace(m: Match): void {
    const bed = BED[m.config.mapId] ?? "amb.hall", layer = WEATHER_LAYER[m.config.weather];
    this.ambience = layer ? [bed, layer] : [bed];
    this.audio?.setAmbience(this.ambience);
  }

  /** The base or the fitting room (the weapon screen too): no fight, the radio plays that screen's songs. */
  screen(view: "base" | "fitting" | "weapon"): void {
    this.mode = "menu";
    this.list = view === "base" ? BASE_PLAYLIST : FITTING_PLAYLIST;
    this.ambience = SCREEN_AMB[view];
    this.spot = view === "base" ? SPOT_BASE : SPOT_FITTING;
    if (!this.audio) return;
    this.audio.placeRadio(...this.spot);
    this.audio.setAmbience(this.ambience);
    this.ambLevel();
    this.audio.music.enterMenu();
    this.audio.music.playlist(this.list);
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
    this.radioList(RADIO_PLAYLIST);
    if (open) this.audio?.music.openMenu();
    else this.audio?.music.closeMenu();
    this.ambLevel();
  }

  /** The fight is over: the radio plays under the end screen. */
  finish(won: boolean | null): void {
    if (won !== null) this.play(won ? "ui.victory" : "ui.defeat", { delay: 0.9 });
    this.mode = "menu";
    this.radioList(RADIO_PLAYLIST);
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

  /** Slider positions, 0..1. */
  get volume(): Readonly<Record<VolumeKind, number>> {
    return this.levels;
  }

  /** Moves a slider; the level is kept for the next visit. */
  setVolume(kind: VolumeKind, value: number): void {
    const v = Math.min(1, Math.max(0, value));
    this.levels[kind] = v;
    store.set(VOLUME_KEY[kind], v.toFixed(2));
    this.applyLevels();
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

  private radioList(list: readonly Station[]): void {
    this.list = list;
    this.spot = SPOT_FIGHT;
    this.audio?.placeRadio(...this.spot);
    this.audio?.music.playlist(list);
  }

  private applyLevels(): void {
    const a = this.audio;
    if (!a) return;
    const fx = this.levels.fx;
    a.setVolume("sfx", fx);
    a.setVolume("ui", fx);
    a.setVolume("music", this.levels.music);
    this.ambLevel();
  }

  private ambLevel(): void {
    this.audio?.setVolume("amb", this.levels.fx * (this.mode === "fight" ? 1 : AMB_IN_MENU));
  }
}

/** One sound for the whole page: the base, the fitting room and the fight. */
export const gameAudio = new ArenaAudio();
