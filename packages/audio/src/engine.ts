/// <reference lib="dom" />
// The browser side: Web Audio buses, effect variation, ambience loops, the
// radio and the music rules. Everything plays through one AudioContext:
//
//   effects ─┬─ sfx ──┐
//            ├─ ui ───┤
//            └─ amb ──┼── master ── speakers
//   fight track ──────┤ (music bus, dips under big hits)
//   radio track ─ radio ┘

import { callRust } from "../../runtime/src/index.ts";
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

const dbToGain = (db: number): number => callRust("audio.dsp.dbToGain", [db]);
/** Half a second of silence, a WAV made in place. */
const SILENCE = (() => {
  const n = 4000, b = new Uint8Array(44 + n), v = new DataView(b.buffer);
  const w = (o: number, t: string) => { for (let i = 0; i < t.length; i++) b[o + i] = t.charCodeAt(i); };
  w(0, "RIFF"); v.setUint32(4, 36 + n, true); w(8, "WAVEfmt "); v.setUint32(16, 16, true); v.setUint16(20, 1, true);
  v.setUint16(22, 1, true); v.setUint32(24, 8000, true); v.setUint32(28, 8000, true); v.setUint16(32, 1, true); v.setUint16(34, 8, true);
  w(36, "data"); v.setUint32(40, n, true); b.fill(128, 44);
  let s = ""; for (const x of b) s += String.fromCharCode(x);
  return "data:audio/wav;base64," + btoa(s);
})();
const MUSIC_FADE = 0.4;

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
    this.deck = new BufferDeck(this);
    this.music = new MusicDirector(this.deck, this.random);
  }

  /** Browsers start audio muted: call this from the first tap. */
  async unlock(): Promise<void> {
    // iOS mutes Web Audio with the ring/silent switch unless the page plays media: ask for
    // "playback" (iOS 17+) and, for older ones, keep a silent <audio> loop going.
    const nav = navigator as Navigator & { audioSession?: { type: string } };
    if (nav.audioSession) try { nav.audioSession.type = "playback"; } catch { /* not allowed */ }
    if (!this.keepAlive) {
      const el = this.keepAlive = new Audio(SILENCE);
      el.loop = true;
      el.setAttribute("playsinline", "");
    }
    void this.keepAlive.play().catch(() => {});
    if (this.ctx.state !== "running") await this.ctx.resume().catch(() => {});
  }

  private keepAlive: HTMLAudioElement | null = null;

  /** A settings slider, 0..1. Squared so the whole travel is audible. */
  setVolume(bus: Bus | "master", value: number) {
    const base = bus === "master" ? MIX.master : MIX[bus];
    this.buses[bus].gain.setTargetAtTime(callRust("audio.dsp.volume", [base, value]), this.ctx.currentTime, 0.03);
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
      const levels = callRust<{ playbackRate: number; gainDb: number }>("audio.dsp.play", [id, FILES[file]?.trimDb ?? 0, o, jitter(def.pitchJitter, this.random), jitter(def.gainJitter, this.random)]);
      src.playbackRate.value = levels.playbackRate;
      const g = this.gainNode(levels.gainDb, this.buses[def.bus]);
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

  private deck: BufferDeck;
  /** Where the radio stands in the room: see RadioChain.place. */
  setFightMusic(v: number): void {
    this.deck.fightLevel(v);
  }

  placeRadio(pan: number, far: number): void {
    this.deck.radio.place(pan, far);
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

  /** Download and decode a long file without keeping it, for the music. */
  async decode(file: string): Promise<AudioBuffer> {
    const r = await fetch(this.url(file));
    if (!r.ok) throw new Error(`${file}: HTTP ${r.status}`);
    return this.ctx.decodeAudioData(await r.arrayBuffer());
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
    const schedule = callRust<{ end: number; fadeInEnd: number; fadeOutStart: number; stopAt: number; next: number; delay: number }>("audio.dsp.loop", [t, buf.duration, this.ctx.currentTime]);
    const { end } = schedule;
    g.gain.setValueAtTime(first ? 1 : 0, t);
    if (!first) g.gain.linearRampToValueAtTime(1, schedule.fadeInEnd);
    g.gain.setValueAtTime(1, schedule.fadeOutStart);
    g.gain.linearRampToValueAtTime(0, end);
    src.start(t);
    src.stop(schedule.stopAt);
    const next = schedule.next;
    this.timer = setTimeout(() => this.pass(buf, next, false), schedule.delay);
  }
}

/**
 * Plays the long music files as decoded buffers in the Web Audio graph. Not <audio> elements:
 * iOS (Safari and the Telegram browser) ignores the music volume when an element feeds the
 * graph. Only the track on each channel stays decoded; a stopped one is let go.
 */
class BufferDeck implements MusicDeck {
  private audio: GameAudio;
  readonly radio: RadioChain;
  private ch: Record<Channel, {
    out: GainNode; fade: GainNode; lvl: GainNode; base: number; src?: AudioBufferSourceNode; buf?: AudioBuffer; track?: TrackId;
    /** Where the track is: playing since `at` from `offset`, or held at `offset`. */
    at: number; offset: number; playing: boolean; serial: number; pending?: ReturnType<typeof setTimeout>;
  }>;

  constructor(audio: GameAudio) {
    this.audio = audio;
    const ctx = audio.ctx;
    const radio = createRadio(ctx);
    radio.output.connect(audio.musicBus());
    this.radio = radio;
    const make = (level: number, to: AudioNode) => {
      const fade = ctx.createGain();
      fade.gain.value = 0;
      const lvl = audio.gainNode(level, to);
      fade.connect(lvl);
      return { out: fade, fade, lvl, base: level, at: 0, offset: 0, playing: false, serial: 0 };
    };
    this.ch = { fight: make(MIX.fightMusic, audio.musicBus()), radio: make(MIX.radioMusic, radio.input) };
  }

  play(channel: Channel, track: TrackId) {
    const c = this.ch[channel];
    clearTimeout(c.pending);
    this.halt(channel);
    const serial = ++c.serial;
    c.track = track; c.offset = 0; c.playing = true; c.buf = undefined;
    this.fadeTo(channel, 1);
    if (channel === "radio") this.radio.setHiss(true);
    void this.audio.decode(TRACKS[track].file).then((buf) => {
      if (c.serial !== serial || !c.playing) { if (c.serial === serial) c.buf = buf; return; }
      c.buf = buf;
      this.start(channel);
    }).catch(() => {});
  }

  pause(channel: Channel) {
    const c = this.ch[channel];
    this.fadeTo(channel, 0);
    clearTimeout(c.pending);
    if (c.playing) { c.offset = this.position(channel); c.playing = false; }
    c.pending = setTimeout(() => this.halt(channel), MUSIC_FADE * 1000 + 50);
    if (channel === "radio") this.radio.setHiss(false);
  }

  resume(channel: Channel) {
    const c = this.ch[channel];
    clearTimeout(c.pending);
    if (!c.track) return;
    if (!c.playing) {
      c.playing = true;
      this.halt(channel);
      if (c.buf) this.start(channel);
      else this.play(channel, c.track);
    }
    this.fadeTo(channel, 1);
    if (channel === "radio") this.radio.setHiss(true);
  }

  stop(channel: Channel) {
    const c = this.ch[channel];
    this.fadeTo(channel, 0);
    clearTimeout(c.pending);
    c.playing = false; c.serial++;
    c.pending = setTimeout(() => { this.halt(channel); c.buf = undefined; c.track = undefined; c.offset = 0; }, MUSIC_FADE * 1000 + 50);
    if (channel === "radio") this.radio.setHiss(false);
  }

  /** The fight track's own slider, 0..1, on top of the music bus. */
  fightLevel(v: number) {
    const c = this.ch.fight;
    c.lvl.gain.setTargetAtTime(callRust("audio.dsp.volume", [c.base, v]), this.audio.ctx.currentTime, 0.05);
  }

  cue(sound: SoundId) {
    this.audio.play(sound);
  }

  private start(channel: Channel) {
    const c = this.ch[channel], ctx = this.audio.ctx;
    if (!c.buf) return;
    const src = ctx.createBufferSource();
    src.buffer = c.buf;
    src.connect(c.out);
    const offset = c.offset < c.buf.duration ? c.offset : 0;
    src.start(0, offset);
    c.at = ctx.currentTime - offset;
    c.src = src;
    src.onended = () => { if (c.src === src && c.playing) { c.src = undefined; this.audio.music.ended(channel); } };
  }

  private halt(channel: Channel) {
    const c = this.ch[channel];
    const src = c.src;
    c.src = undefined;
    if (src) { src.onended = null; try { src.stop(); } catch { /* not started */ } src.disconnect(); }
  }

  private position(channel: Channel): number {
    const c = this.ch[channel];
    return c.src ? this.audio.ctx.currentTime - c.at : c.offset;
  }

  private fadeTo(channel: Channel, v: number) {
    const g = this.ch[channel].fade.gain;
    const now = this.audio.ctx.currentTime;
    g.cancelScheduledValues(now);
    g.setValueAtTime(g.value, now);
    g.linearRampToValueAtTime(v, now + MUSIC_FADE);
  }
}
