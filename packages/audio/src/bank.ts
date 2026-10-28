// Every sound the game can play, by id. Files live in packages/audio/assets and
// are built by tools/audio/build_sfx.py (effects) or copied from Art's tracks
// (music). The client serves that folder and passes its URL to GameAudio.

export type Bus = "sfx" | "ui" | "amb" | "music";

export interface SoundDef {
  /** Variants; one is picked at random, never the same twice in a row. */
  files: string[];
  bus: Bus;
  /** Level of this sound on its bus, in dB. */
  gainDb?: number;
  /** Random pitch spread, ± semitones. */
  pitchJitter?: number;
  /** Random level spread, ± dB. */
  gainJitter?: number;
  /** Dip the music by this many dB while the sound lands. */
  duckDb?: number;
}

const variants = (stem: string, n: number) => Array.from({ length: n }, (_, i) => `${stem}-${i + 1}.mp3`);
const one = (stem: string) => [`${stem}.mp3`];

const near = (stem: string, n: number): SoundDef => ({ files: variants(stem, n), bus: "sfx", pitchJitter: 0.4, gainJitter: 1 });
const far = (stem: string, n: number): SoundDef => ({ files: variants(stem, n), bus: "sfx", pitchJitter: 0.4, gainJitter: 1.5 });

const DEFS = {
  // our shot is heard from behind the shoulder, the enemy's across the hall
  "shot.pistol.near": near("sfx/shot-pistol-near", 3),
  "shot.pistol.far": far("sfx/shot-pistol-far", 3),
  "shot.shotgun.near": near("sfx/shot-shotgun-near", 3),
  "shot.shotgun.far": far("sfx/shot-shotgun-far", 3),
  "shot.rifle.near": near("sfx/shot-rifle-near", 4),
  "shot.rifle.far": far("sfx/shot-rifle-far", 3),
  "shot.sniper.near": near("sfx/shot-sniper-near", 3),
  "shot.sniper.far": far("sfx/shot-sniper-far", 3),
  "casing": { files: variants("sfx/casing", 3), bus: "sfx", gainDb: -4, pitchJitter: 1.5, gainJitter: 2 },

  "knife.swing": { files: variants("sfx/knife-swing", 2), bus: "sfx", pitchJitter: 1 },
  "knife.hit": { files: variants("sfx/knife-hit", 3), bus: "sfx", pitchJitter: 0.8 },
  "knife.blocked": { files: variants("sfx/knife-blocked", 2), bus: "sfx", pitchJitter: 0.8 },

  // a hit on a Persona: plates on the body and legs, the bare suit elsewhere
  "hit.armor": { files: variants("sfx/hit-armor", 3), bus: "sfx", pitchJitter: 1, gainJitter: 1.5 },
  "hit.suit": { files: variants("sfx/hit-suit", 3), bus: "sfx", pitchJitter: 1, gainJitter: 1.5 },
  "hit.head": { files: variants("sfx/hit-head", 2), bus: "sfx", gainDb: -2, pitchJitter: 1 },
  "hit.crit": { files: variants("sfx/hit-crit", 2), bus: "sfx", gainDb: -2, pitchJitter: 1 },
  "shield.block": { files: variants("sfx/shield-block", 3), bus: "sfx", gainDb: -1, pitchJitter: 0.7 },
  "shield.break": { files: one("sfx/shield-break"), bus: "sfx", pitchJitter: 0.5 },
  "miss.near": { files: variants("sfx/miss-near", 3), bus: "sfx", pitchJitter: 1.5, gainJitter: 2 },
  "miss.far": { files: variants("sfx/miss-far", 2), bus: "sfx", pitchJitter: 1.5, gainJitter: 2 },

  "grenade.pin": { files: one("sfx/grenade-pin"), bus: "sfx" },
  "grenade.throw": { files: one("sfx/grenade-throw"), bus: "sfx", pitchJitter: 1 },
  "grenade.bounce": { files: one("sfx/grenade-bounce"), bus: "sfx", pitchJitter: 1.5 },
  "grenade.blast": { files: one("sfx/grenade-blast"), bus: "sfx", pitchJitter: 0.6, duckDb: 6 },

  "step": { files: variants("sfx/step", 4), bus: "sfx", pitchJitter: 1, gainJitter: 2 },
  "dash.whoosh": { files: one("sfx/dash-whoosh"), bus: "sfx", pitchJitter: 1 },
  "gear.rustle": { files: variants("sfx/gear-rustle", 2), bus: "sfx", gainDb: -3, pitchJitter: 1.5, gainJitter: 2 },
  "body.fall": { files: one("sfx/body-fall"), bus: "sfx", pitchJitter: 0.8, duckDb: 4 },
  "persona.powerdown": { files: one("sfx/persona-powerdown"), bus: "sfx", gainDb: -2 },
  "persona.powerup": { files: one("sfx/persona-powerup"), bus: "sfx", gainDb: -2 },
  "sync.lost": { files: one("sfx/sync-lost"), bus: "sfx", duckDb: 8 },

  "ui.tap": { files: one("sfx/ui-tap"), bus: "ui", pitchJitter: 0.5 },
  "ui.guard": { files: one("sfx/ui-guard"), bus: "ui", pitchJitter: 0.5 },
  "ui.step": { files: one("sfx/ui-step"), bus: "ui", pitchJitter: 0.5 },
  "ui.confirm": { files: one("sfx/ui-confirm"), bus: "ui" },
  "ui.tick": { files: one("sfx/ui-tick"), bus: "ui" },
  "ui.tick.urgent": { files: one("sfx/ui-tick-urgent"), bus: "ui" },
  "ui.open": { files: one("sfx/ui-open"), bus: "ui" },
  "ui.close": { files: one("sfx/ui-close"), bus: "ui" },
  "ui.turn": { files: one("sfx/ui-turn"), bus: "ui" },
  "ui.error": { files: one("sfx/ui-error"), bus: "ui" },
  "ui.match.found": { files: one("sfx/ui-match-found"), bus: "ui" },
  "ui.victory": { files: one("sfx/ui-victory"), bus: "ui" },
  "ui.defeat": { files: one("sfx/ui-defeat"), bus: "ui" },
  "loot.water": { files: one("sfx/loot-water"), bus: "ui" },
  "loot.scrap": { files: one("sfx/loot-scrap"), bus: "ui" },

  "equip.pistol": { files: one("sfx/equip-pistol"), bus: "ui" },
  "equip.shotgun": { files: one("sfx/equip-shotgun"), bus: "ui" },
  "equip.rifle": { files: one("sfx/equip-rifle"), bus: "ui" },
  "equip.sniper": { files: one("sfx/equip-sniper"), bus: "ui" },
  "equip.knife": { files: one("sfx/equip-knife"), bus: "ui" },
  "equip.grenade": { files: one("sfx/equip-grenade"), bus: "ui" },
  "equip.armor": { files: one("sfx/equip-armor"), bus: "ui" },
  "equip.helmet": { files: one("sfx/equip-helmet"), bus: "ui" },
  "equip.suit": { files: one("sfx/equip-suit"), bus: "ui" },
  "equip.boots": { files: one("sfx/equip-boots"), bus: "ui" },
  "equip.chevron": { files: one("sfx/equip-chevron"), bus: "ui" },

  "radio.on": { files: one("sfx/radio-on"), bus: "ui", gainDb: -2 },
  "radio.off": { files: one("sfx/radio-off"), bus: "ui", gainDb: -2 },
  "radio.tune": { files: one("sfx/radio-tune"), bus: "ui", gainDb: -11 },

  // world events: lightning strike, a helicopter passing, shelling far away
  "thunder": { files: variants("sfx/thunder", 2), bus: "amb", pitchJitter: 1 },
  "helicopter.flyby": { files: one("sfx/helicopter-flyby"), bus: "amb" },
  "artillery.far": { files: one("sfx/artillery-far"), bus: "amb", pitchJitter: 1.5, gainJitter: 2 },

  // loops: one location bed plus an optional weather layer
  "amb.hall": { files: one("amb/amb-hall"), bus: "amb" },
  "amb.blocks": { files: one("amb/amb-blocks"), bus: "amb" },
  "amb.storm": { files: one("amb/amb-storm"), bus: "amb" },
  "amb.dust": { files: one("amb/amb-dust"), bus: "amb" },
  "amb.fog": { files: one("amb/amb-fog"), bus: "amb", gainDb: 2 },
  "amb.base": { files: one("amb/amb-base"), bus: "amb" },
} satisfies Record<string, SoundDef>;

export type SoundId = keyof typeof DEFS;

export const SOUNDS: Record<SoundId, SoundDef> = DEFS;

// ---------- music ----------

export interface TrackDef {
  file: string;
  title: string;
}

export const TRACKS = {
  "the-limbo-3": { file: "music/the-limbo-3.mp3", title: "The Limbo" },
  "404-room": { file: "music/404-room.mp3", title: "404 Room" },
  "in-the-middle-of-nowhere": { file: "music/in-the-middle-of-nowhere.mp3", title: "In The Middle of Nowhere" },
  "crazy-2": { file: "music/crazy-2.mp3", title: "Crazy" },
  "fxck-wag-6": { file: "music/fxck-wag-6.mp3", title: "FXCK $WAG" },
  "fog-3": { file: "music/fog-3.mp3", title: "Fog" },
  "try-to-fake": { file: "music/try-to-fake.mp3", title: "Try To Fake" },
} satisfies Record<string, TrackDef>;

export type TrackId = keyof typeof TRACKS;

/** Art, 2026-10-27: these play in a fight, picked at random. */
export const FIGHT_PLAYLIST: TrackId[] = ["the-limbo-3", "404-room", "in-the-middle-of-nowhere"];
/** The rest plays from the radio in the operator's room: menu, gear, results. */
export const RADIO_PLAYLIST = ["crazy-2", "fxck-wag-6", "fog-3", "try-to-fake"] as const satisfies readonly TrackId[];

/** A track the radio can be tuned to. */
export type Station = (typeof RADIO_PLAYLIST)[number];
/** The radio on the base screen and in the fitting room: different songs in each. */
export const BASE_PLAYLIST: readonly Station[] = ["crazy-2", "fog-3"];
export const FITTING_PLAYLIST: readonly Station[] = ["fxck-wag-6", "try-to-fake"];

/**
 * Bus levels in dB. Effects files are mastered around −10…−25 dBFS of punch and
 * music files at −16 LUFS, so the fight music sits under the shots and the radio
 * well under that (Art: the menu music was too loud, then the fight music too).
 */
export const MIX = {
  master: 0,
  sfx: 0,
  ui: -3,
  amb: -2,
  music: 0,
  fightMusic: -15.5, // Art: 1.5× quieter than the old -12
  radioMusic: -17,
  /** Effects of the enemy fade by this much per step of distance... */
  distanceDbPerStep: 0.6,
  /** ...but never by more than this. */
  distanceDbMax: 6,
} as const;
