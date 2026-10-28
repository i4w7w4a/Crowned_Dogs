/// <reference lib="dom" />
// The browser side: Web Audio buses, effect variation, ambience loops, the
// radio and the music rules. Everything plays through one AudioContext:
//
//   effects ─┬─ sfx ──┐
//            ├─ ui ───┤
//            └─ amb ──┼── master ── speakers
//   fight track ──────┤ (music bus, dips under big hits)
//   radio track ─ radio ┘

import { FILES } from "./files.generated.ts";
import { MIX, SOUNDS, TRACKS, type Bus, type SoundId, type TrackId } from "./bank.ts";
import type { Cue } from "./cues.ts";
import { MusicDirector, type Channel, type MusicDeck } from "./music.ts";
import { jitter, VariantPicker, type Random } from "./pick.ts";
import { createRadio, type RadioChain } from "./radio.ts";

export interface GameAudioOptions {
  /** Where packages/audio/assets is served, e.g. "/audio/". */
  baseUrl: string;
  random?: Random;
}

export interface PlayOptions {
  /** Seconds from now. */
  delay?: number;
  gainDb?: number;
  pan?: number;
}

const dbToGain = (db: number) => 10 ** (db / 20);
const MUSIC_FADE = 0.4;
const AMB_OVERLAP = 1.5;

export class GameAudio {
  readonly ctx: AudioContext;
  readonly music: MusicDirector;
  private buses: Record<Bus | "master", GainNode>;
  private duckGain: GainNode;
  private buffers = new Map<string, Promise<AudioBuffer>>();
  private picker: VariantPicker;
  private random: Random;
  private loops = new Map<SoundId, AmbienceLoop>();
  private baseUrl: string;

  constructor(opts: GameAudioOptions) {
    this.baseUrl = opts.baseUrl;
    this.ctx = new AudioContext({ latencyHint: "interactive" });
    this.random = opts.random ?? Math.random;
    this.picker = new VariantPicker(this.random);
    const master = this.gainNode(MIX.master, this.ctx.destination);
    this.duckGain = this.gainNode(0, master);
    this.buses = {
      master,
      sfx: this.gainNode(MIX.sfx, master),
      ui: this.gainNode(MIX.ui, master),
      amb: this.gainNode(MIX.amb, master),
      music: this.gainNode(MIX.music, this.duckGain),
    };
    this.music = new MusicDirector(new ElementDeck(this), this.random);
  }

  /** Browsers start audio muted: call this from the first tap. */
  async unlock(): Promise<void> {
    if (this.ctx.state !== "running") await this.ctx.resume();
  }

  /** A settings slider, 0..1. Squared so the whole travel is audible. */
  setVolume(bus: Bus | "master", value: number) {
    const v = Math.min(1, Math.max(0, value));
    const base = bus === "master" ? MIX.master : MIX[bus];
    this.buses[bus].gain.setTargetAtTime(dbToGain(base) * v * v, this.ctx.currentTime, 0.03);
  }

  url(file: string): string {
    return this.baseUrl.replace(/\/?$/, "/") + file;
  }

  /** Download and decode ahead of time, e.g. the weapons of both fighters at fight start. */
  async preload(ids: SoundId[]): Promise<void> {
    await Promise.all(ids.flatMap((id) => SOUNDS[id].files.map((f) => this.buffer(f))));
  }

  play(id: SoundId, o: PlayOptions = {}): void {
    const def = SOUNDS[id];
    const file = this.picker.pick(id, def.files);
    const when = this.ctx.currentTime + (o.delay ?? 0);
    void this.buffer(file).then((buf) => {
      const src = this.ctx.createBufferSource();
      src.buffer = buf;
      src.playbackRate.value = 2 ** (jitter(def.pitchJitter, this.random) / 12);
      const level = (def.gainDb ?? 0) + (FILES[file]?.trimDb ?? 0) + jitter(def.gainJitter, this.random) + (o.gainDb ?? 0);
      const g = this.gainNode(level, this.buses[def.bus]);
      let head: AudioNode = g;
      if (o.pan) {
        const p = this.ctx.createStereoPanner();
        p.pan.value = o.pan;
        p.connect(g);
        head = p;
      }
      src.connect(head);
      // a late decode plays at once rather than never
      src.start(Math.max(when, this.ctx.currentTime));
      if (def.duckDb) this.duck(def.duckDb, Math.max(when, this.ctx.currentTime));
    });
  }

  /** Play a turn's sound script (see cuesForTurn). */
  playCues(cues: Cue[], startIn = 0.05): void {
    for (const c of cues) this.play(c.sound, { delay: startIn + c.at, gainDb: c.gainDb, pan: c.pan });
  }

  /** Crossfade to a new set of loops: a location bed plus an optional weather layer. */
  setAmbience(ids: SoundId[], fade = 2): void {
    for (const [id, loop] of this.loops) {
      if (!ids.includes(id)) {
        loop.stop(fade);
        this.loops.delete(id);
      }
    }
    for (const id of ids) {
      if (this.loops.has(id)) continue;
      const def = SOUNDS[id];
      const file = def.files[0]!;
      const loop = new AmbienceLoop(this.ctx, this.gainNode(def.gainDb ?? 0, this.buses[def.bus]));
      this.loops.set(id, loop);
      void this.buffer(file).then((buf) => loop.start(buf, fade));
    }
  }

  /** The music bus, for the radio and fight decks. */
  musicBus(): AudioNode {
    return this.buses.music;
  }

  gainNode(db: number, to: AudioNode): GainNode {
    const g = this.ctx.createGain();
    g.gain.value = dbToGain(db);
    g.connect(to);
    return g;
  }

  private buffer(file: string): Promise<AudioBuffer> {
    let p = this.buffers.get(file);
    if (!p) {
      p = fetch(this.url(file))
        .then((r) => {
          if (!r.ok) throw new Error(`${file}: HTTP ${r.status}`);
          return r.arrayBuffer();
        })
        .then((data) => this.ctx.decodeAudioData(data));
      p.catch(() => this.buffers.delete(file));
      this.buffers.set(file, p);
    }
    return p;
  }

  private duck(db: number, at: number) {
    const g = this.duckGain.gain;
    g.cancelScheduledValues(at);
    g.setTargetAtTime(dbToGain(-db), at, 0.02);
    g.setTargetAtTime(1, at + 0.25, 0.4);
  }
}

/** A loop that overlaps each pass with the next, so codec padding never leaves a gap. */
class AmbienceLoop {
  private timer: ReturnType<typeof setTimeout> | undefined;
  private stopped = false;
  private ctx: AudioContext;
  private level: GainNode;

  constructor(ctx: AudioContext, out: GainNode) {
    this.ctx = ctx;
    this.level = ctx.createGain();
    this.level.gain.value = 0;
    this.level.connect(out);
  }

  start(buf: AudioBuffer, fade: number) {
    if (this.stopped) return;
    const now = this.ctx.currentTime;
    this.level.gain.setTargetAtTime(1, now, fade / 3);
    this.pass(buf, now, true);
  }

  stop(fade: number) {
    this.stopped = true;
    clearTimeout(this.timer);
    const now = this.ctx.currentTime;
    this.level.gain.cancelScheduledValues(now);
    this.level.gain.setTargetAtTime(0, now, fade / 3);
    setTimeout(() => this.level.disconnect(), fade * 1500);
  }

  private pass(buf: AudioBuffer, t: number, first: boolean) {
    if (this.stopped) return;
    const src = this.ctx.createBufferSource();
    src.buffer = buf;
    const g = this.ctx.createGain();
    src.connect(g).connect(this.level);
    const end = t + buf.duration;
    g.gain.setValueAtTime(first ? 1 : 0, t);
    if (!first) g.gain.linearRampToValueAtTime(1, t + AMB_OVERLAP);
    g.gain.setValueAtTime(1, end - AMB_OVERLAP);
    g.gain.linearRampToValueAtTime(0, end);
    src.start(t);
    src.stop(end + 0.05);
    const next = end - AMB_OVERLAP;
    this.timer = setTimeout(() => this.pass(buf, next, false), Math.max(0, (next - this.ctx.currentTime - 1) * 1000));
  }
}

/** Streams the long music files through <audio> elements into the Web Audio graph. */
class ElementDeck implements MusicDeck {
  private audio: GameAudio;
  private radio: RadioChain;
  private ch: Record<Channel, { el: HTMLAudioElement; fade: GainNode; pending?: ReturnType<typeof setTimeout> }>;

  constructor(audio: GameAudio) {
    this.audio = audio;
    const ctx = audio.ctx;
    const radio = createRadio(ctx);
    radio.output.connect(audio.musicBus());
    this.radio = radio;
    const make = (channel: Channel, level: number, to: AudioNode) => {
      const el = new Audio();
      el.preload = "auto";
      el.crossOrigin = "anonymous";
      const fade = ctx.createGain();
      fade.gain.value = 0;
      ctx.createMediaElementSource(el).connect(fade).connect(audio.gainNode(level, to));
      el.addEventListener("ended", () => audio.music.ended(channel));
      return { el, fade };
    };
    this.ch = {
      fight: make("fight", MIX.fightMusic, audio.musicBus()),
      radio: make("radio", MIX.radioMusic, radio.input),
    };
  }

  play(channel: Channel, track: TrackId) {
    const c = this.ch[channel];
    clearTimeout(c.pending);
    c.el.src = this.audio.url(TRACKS[track].file);
    c.el.currentTime = 0;
    this.fadeTo(channel, 1);
    void c.el.play().catch(() => {});
    if (channel === "radio") this.radio.setHiss(true);
  }

  pause(channel: Channel) {
    const c = this.ch[channel];
    this.fadeTo(channel, 0);
    clearTimeout(c.pending);
    c.pending = setTimeout(() => c.el.pause(), MUSIC_FADE * 1000 + 50);
    if (channel === "radio") this.radio.setHiss(false);
  }

  resume(channel: Channel) {
    const c = this.ch[channel];
    clearTimeout(c.pending);
    if (!c.el.src) return;
    void c.el.play().catch(() => {});
    this.fadeTo(channel, 1);
    if (channel === "radio") this.radio.setHiss(true);
  }

  stop(channel: Channel) {
    const c = this.ch[channel];
    this.fadeTo(channel, 0);
    clearTimeout(c.pending);
    c.pending = setTimeout(() => {
      c.el.pause();
      c.el.removeAttribute("src");
      c.el.load();
    }, MUSIC_FADE * 1000 + 50);
    if (channel === "radio") this.radio.setHiss(false);
  }

  cue(sound: SoundId) {
    this.audio.play(sound);
  }

  private fadeTo(channel: Channel, v: number) {
    const g = this.ch[channel].fade.gain;
    const now = this.audio.ctx.currentTime;
    g.cancelScheduledValues(now);
    g.setValueAtTime(g.value, now);
    g.linearRampToValueAtTime(v, now + MUSIC_FADE);
  }
}
