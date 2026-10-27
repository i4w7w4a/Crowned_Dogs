// Music and sound effects.
// Fight: one of three of Art's tracks, picked at random, the next one starts when a track ends.
// Menu and end screen: the fight track pauses on the spot, and the other four play quietly from an
// in-world radio (band-limited, a little gritty, with hiss). Effects and the hall ambience come from
// the Sonniss GDC 2024 bundle: fine inside the game, but never to be published on their own.
// Everything goes through Web Audio gains, so fades work on phones (iOS ignores audio.volume).

export interface Track {
  id: string;
  src: string;
  name?: string;
  len?: string;
}

export const FIGHT_TRACKS: Track[] = [
  { id: "limbo", src: "music/the-limbo-3.mp3" },
  { id: "room", src: "music/404-room.mp3" },
  { id: "nowhere", src: "music/in-the-middle-of-nowhere.mp3" },
];

export const STATIONS: Track[] = [
  { id: "crazy", name: "CRAZY_2", len: "3:18", src: "music/crazy-2.mp3" },
  { id: "wag", name: "FXCK WAG_6", len: "2:41", src: "music/fxck-wag-6.mp3" },
  { id: "fake", name: "Try To Fake", len: "1:32", src: "music/try-to-fake.mp3" },
  { id: "fog", name: "Fog_3", len: "1:14", src: "music/fog-3.mp3" },
];

/** Effect id -> how many takes there are and how loud it plays. */
export const EFFECTS = {
  "shot-ours": { n: 3, gain: 1 },
  "shot-enemy": { n: 2, gain: 1 },
  "hit-armor": { n: 3, gain: 1 },
  "shield-block": { n: 2, gain: 1 },
  "miss": { n: 2, gain: 1 },
  "knife": { n: 1, gain: 0.73 },
  "grenade-throw": { n: 1, gain: 1 },
  "explosion": { n: 1, gain: 1 },
  "bodyfall": { n: 1, gain: 0.65 },
  "ui-click": { n: 1, gain: 0.88 },
  "ui-confirm": { n: 1, gain: 0.41 },
  "ui-tick": { n: 1, gain: 0.68 },
} as const;
export type Effect = keyof typeof EFFECTS;

const effectFile = (id: Effect, i: number) => `sfx/${id.startsWith("ui-") ? id : "sfx-" + id}-${i}.mp3`;
const AMBIENCE = "sfx/amb-hall-loop-1.mp3";

const LEVEL = { fight: 0.6, station: 0.15, hiss: 0.03, burst: 0.2, sfx: 1, amb: 0.87, ambMenu: 0.3 };
/** How hard the radio's small speaker distorts; the trim after it keeps the level where it was. */
const DRIVE = 4;

export type Scene = "fight" | "menu";

interface Player { el: HTMLAudioElement; gain: GainNode }

const store = {
  get(k: string): string | null { try { return localStorage.getItem(k); } catch { return null; } },
  set(k: string, v: string): void { try { localStorage.setItem(k, v); } catch { /* private mode */ } },
};

function softClip(k: number): Float32Array<ArrayBuffer> {
  const n = 1024, c = new Float32Array(n);
  for (let i = 0; i < n; i++) { const x = i * 2 / n - 1; c[i] = (1 + k) * x / (1 + k * Math.abs(x)); }
  return c;
}

export class Sound {
  on = store.get("cd-music") !== "off";
  mode: Scene = "fight";
  fight = FIGHT_TRACKS[Math.floor(Math.random() * FIGHT_TRACKS.length)]!.id;
  station = STATIONS.some((t) => t.id === store.get("cd-station")) ? store.get("cd-station")! : STATIONS[0]!.id;

  private ctx: AudioContext | null = null;
  private master!: GainNode;
  private radioIn!: GainNode;
  private radioOut!: GainNode;
  private hiss!: GainNode;
  private sfx!: GainNode;
  private amb!: GainNode;
  private players: Record<string, Player> = {};
  private buffers: Record<string, AudioBuffer> = {};
  private takes: Partial<Record<Effect, number>> = {};

  private base: string;

  /** @param base folder with music/ and sfx/, ending in a slash */
  constructor(base: string) {
    this.base = base;
  }

  /** Browsers only allow sound after a tap: call this from the first pointerdown or key press. */
  unlock(): void {
    if (this.ctx) return;
    const ctx = this.ctx = new AudioContext();
    this.master = ctx.createGain(); this.master.connect(ctx.destination);
    // the radio: a small speaker
    const hp = ctx.createBiquadFilter(); hp.type = "highpass"; hp.frequency.value = 380; hp.Q.value = 0.7;
    const mid = ctx.createBiquadFilter(); mid.type = "peaking"; mid.frequency.value = 1600; mid.gain.value = 5; mid.Q.value = 1;
    const lp = ctx.createBiquadFilter(); lp.type = "lowpass"; lp.frequency.value = 3400; lp.Q.value = 0.9;
    const grit = ctx.createWaveShaper(); grit.curve = softClip(DRIVE); grit.oversample = "2x";
    const trim = ctx.createGain(); trim.gain.value = 1 / (1 + DRIVE);
    this.radioIn = ctx.createGain();
    this.radioOut = ctx.createGain(); this.radioOut.gain.value = 0;
    this.radioIn.connect(hp).connect(mid).connect(lp).connect(grit).connect(trim).connect(this.radioOut).connect(this.master);
    // hiss and crackle of the speaker
    const len = ctx.sampleRate * 2, buf = ctx.createBuffer(1, len, ctx.sampleRate), d = buf.getChannelData(0);
    for (let i = 0; i < len; i++) d[i] = (Math.random() * 2 - 1) * 0.35 + (Math.random() < 0.0004 ? (Math.random() * 2 - 1) * 3 : 0);
    const noise = ctx.createBufferSource(); noise.buffer = buf; noise.loop = true;
    const band = ctx.createBiquadFilter(); band.type = "bandpass"; band.frequency.value = 2200; band.Q.value = 0.6;
    this.hiss = ctx.createGain(); this.hiss.gain.value = LEVEL.hiss;
    noise.connect(band).connect(this.hiss).connect(this.radioOut);
    noise.start();
    // effects and the hall
    this.sfx = ctx.createGain(); this.sfx.gain.value = LEVEL.sfx; this.sfx.connect(this.master);
    this.amb = ctx.createGain(); this.amb.gain.value = 0; this.amb.connect(this.master);
    for (const [id, e] of Object.entries(EFFECTS) as [Effect, { n: number }][]) {
      for (let i = 1; i <= e.n; i++) void this.load(id + i, effectFile(id, i));
    }
    // a buffer loops without the gap an <audio loop> leaves at the seam
    void this.load("amb", AMBIENCE).then(() => {
      const b = this.buffers.amb;
      if (!b) return;
      const src = ctx.createBufferSource(); src.buffer = b; src.loop = true;
      src.connect(this.amb); src.start();
      this.applyAmbience();
    });
    this.apply();
  }

  /** "fight": the fight track plays. "menu": it pauses on the spot and the radio plays. */
  scene(mode: Scene): void {
    this.mode = mode;
    this.apply();
  }

  /** A new fight starts with a new random track. */
  newFight(): void {
    if (this.ctx) this.nextFight(); else this.fight = this.pickFight();
  }

  /** Switches the radio to another track. */
  tune(id: string): void {
    const prev = this.station;
    this.station = id; store.set("cd-station", id);
    if (!this.ctx) return;
    const old = this.players[prev];
    if (prev !== id && old) this.fadeOut(old, 0.3, () => this.station === prev);
    if (this.mode === "menu" && this.on) {
      const p = this.stationPlayer(id);
      p.el.currentTime = 0;
      this.fadeIn(p, LEVEL.station, 0.3);
      this.burst(0.3);
    }
  }

  toggle(): void {
    this.on = !this.on; store.set("cd-music", this.on ? "on" : "off");
    this.unlock(); this.apply();
  }

  /** Plays an effect now or after `delay` ms. Takes of one sound play in turn, with a slight pitch change. */
  play(id: Effect, delay = 0): void {
    const ctx = this.ctx;
    if (!ctx) return;
    const e = EFFECTS[id], i = this.takes[id] = (this.takes[id] ?? 0) % e.n + 1, buf = this.buffers[id + i];
    if (!buf) return;
    const src = ctx.createBufferSource(); src.buffer = buf;
    src.playbackRate.value = 1 + (Math.random() - 0.5) * 0.06;
    const g = ctx.createGain(); g.gain.value = e.gain;
    src.connect(g).connect(this.sfx);
    src.start(ctx.currentTime + delay / 1000);
  }

  /** One attack: the shot (or knife, or grenade blast) and what it hit. `ours`: we attack. */
  attack(weapon: "gun" | "knife" | "grenade", hit: boolean, blocked: boolean, ours: boolean): void {
    if (weapon === "grenade") { this.play("explosion"); return; }
    this.play(weapon === "knife" ? "knife" : ours ? "shot-ours" : "shot-enemy");
    if (!hit) { if (weapon !== "knife") this.play("miss", 160); return; }
    this.play(blocked ? "shield-block" : "hit-armor", weapon === "knife" ? 40 : 70);
  }

  private async load(key: string, src: string): Promise<void> {
    try {
      const res = await fetch(this.base + src);
      if (res.ok) this.buffers[key] = await this.ctx!.decodeAudioData(await res.arrayBuffer());
    } catch { /* that sound stays silent */ }
  }

  private player(t: Track, out: AudioNode, onEnd: () => void): Player {
    let p = this.players[t.id];
    if (!p) {
      const el = new Audio(this.base + t.src);
      el.preload = "auto";
      const gain = this.ctx!.createGain(); gain.gain.value = 0;
      this.ctx!.createMediaElementSource(el).connect(gain).connect(out);
      el.addEventListener("ended", onEnd);
      p = this.players[t.id] = { el, gain };
    }
    return p;
  }

  private fightPlayer(id: string): Player {
    return this.player(FIGHT_TRACKS.find((t) => t.id === id)!, this.master, () => this.nextFight());
  }

  private stationPlayer(id: string): Player {
    const i = STATIONS.findIndex((t) => t.id === id);
    // the radio plays on
    return this.player(STATIONS[i]!, this.radioIn, () => this.tune(STATIONS[(i + 1) % STATIONS.length]!.id));
  }

  private ramp(param: AudioParam, to: number, sec: number): void {
    const now = this.ctx!.currentTime;
    param.cancelScheduledValues(now); param.setValueAtTime(param.value, now);
    param.linearRampToValueAtTime(to, now + sec);
  }

  private fadeOut(p: Player, sec: number, keep: () => boolean): void {
    this.ramp(p.gain.gain, 0, sec);
    setTimeout(() => { if (!keep()) p.el.pause(); }, sec * 1000 + 80);
  }

  private fadeIn(p: Player, level: number, sec: number): void {
    if (p.el.paused) p.el.play().catch(() => undefined);
    this.ramp(p.gain.gain, level, sec);
  }

  /** The click and short burst of noise when a radio comes on. */
  private burst(sec: number): void {
    const now = this.ctx!.currentTime, g = this.hiss.gain;
    g.cancelScheduledValues(now); g.setValueAtTime(LEVEL.burst, now); g.linearRampToValueAtTime(LEVEL.hiss, now + sec);
  }

  /** A random fight track, never the same one twice in a row. */
  private pickFight(): string {
    const pool = FIGHT_TRACKS.filter((t) => t.id !== this.fight);
    return pool[Math.floor(Math.random() * pool.length)]!.id;
  }

  private nextFight(): void {
    const prev = this.players[this.fight];
    this.fight = this.pickFight();
    if (prev) { prev.el.pause(); prev.el.currentTime = 0; prev.gain.gain.value = 0; }
    const p = this.fightPlayer(this.fight);
    p.el.currentTime = 0;
    if (this.on && this.mode === "fight") this.fadeIn(p, LEVEL.fight, 1.5);
  }

  private applyAmbience(): void {
    this.ramp(this.amb.gain, this.mode === "menu" ? LEVEL.ambMenu : LEVEL.amb, 0.6);
  }

  private apply(): void {
    if (!this.ctx) return;
    if (this.ctx.state === "suspended") void this.ctx.resume();
    this.ramp(this.master.gain, this.on ? 1 : 0, 0.3);
    this.applyAmbience();
    if (!this.on) {
      for (const p of Object.values(this.players)) this.fadeOut(p, 0.3, () => this.on);
      return;
    }
    const fight = this.fightPlayer(this.fight);
    if (this.mode === "fight") {
      this.ramp(this.radioOut.gain, 0, 0.3);
      for (const t of STATIONS) { const p = this.players[t.id]; if (p) this.fadeOut(p, 0.3, () => this.mode === "menu"); }
      this.fadeIn(fight, LEVEL.fight, 0.4);
    } else {
      this.fadeOut(fight, 0.4, () => this.mode === "fight");
      this.fadeIn(this.stationPlayer(this.station), LEVEL.station, 0.4);
      if (this.radioOut.gain.value < 0.5) this.burst(0.25);
      this.ramp(this.radioOut.gain, 1, 0.15);
    }
  }
}
