import test from "node:test";
import assert from "node:assert/strict";
import { BrowserArenaAudio, BrowserGameAudio } from "../../../packages/runtime/src/index.ts";

test("silent QA audio never constructs the playback engine or changes saved levels", () => {
  const stored = new Map([["cd-vol-fx", "0.27"], ["cd-vol-music", "0.63"], ["cd-vol-fight", "0.41"]]);
  const writes: [string, string][] = [];
  let constructions = 0;
  class ForbiddenAudio { constructor() { constructions++; throw new Error("Audio must stay silent"); } }
  const audio = new BrowserArenaAudio({
    silent: true, GameAudio: ForbiddenAudio,
    store: { get: (key: string) => stored.get(key) ?? null, set: (key: string, value: string) => { writes.push([key, value]); stored.set(key, value); } },
    BASE_PLAYLIST: ["crazy-2"], FITTING_PLAYLIST: ["fog-3"], RADIO_PLAYLIST: ["crazy-2", "fog-3"], fightSounds: () => [],
  });
  audio.unlock(); audio.screen("fitting"); audio.unlock(); audio.toggle(); audio.tune("fog-3");
  audio.newFight({ config: { fighters: [{}, {}], weather: "fog", mapId: "saltHall" }, fighters: [{ weapon: "rifle" }, { weapon: "pistol" }] });
  audio.sheet(true); audio.sheet(false); audio.finish(true); audio.play("ui.tap", {});
  assert.equal(constructions, 0);
  assert.deepEqual(audio.volume, { fx: 0.27, music: 0.63, fight: 0.41 });
  assert.ok(writes.every(([key]) => !key.startsWith("cd-vol-")));
  assert.equal(audio.station, "fog-3");
});

class Parameter {
  value = 1;
  events: [string, ...number[]][] = [];
  setTargetAtTime(...values: number[]) { this.events.push(["target", ...values]); }
  cancelScheduledValues(...values: number[]) { this.events.push(["cancel", ...values]); }
  setValueAtTime(...values: number[]) { this.events.push(["set", ...values]); }
  linearRampToValueAtTime(...values: number[]) { this.events.push(["ramp", ...values]); }
}
class AudioNode {
  gain = new Parameter(); pan = new Parameter(); playbackRate = new Parameter();
  buffer: { duration: number } | null = null;
  onended: (() => void) | null = null;
  connections: AudioNode[] = [];
  starts: number[][] = []; stops: number[][] = [];
  disconnected = false;
  connect(to: AudioNode) { this.connections.push(to); return to; }
  disconnect() { this.disconnected = true; }
  start(...args: number[]) { this.starts.push(args); }
  stop(...args: number[]) { this.stops.push(args); }
}
class Context {
  static instances: Context[] = [];
  currentTime = 10;
  state = "suspended";
  destination = new AudioNode();
  gains: AudioNode[] = []; sources: AudioNode[] = [];
  decoded = 0; resumes = 0;
  pending: ((buffer: { duration: number }) => void)[] = [];
  defer = false;
  constructor() { Context.instances.push(this); }
  createGain() { const node = new AudioNode(); this.gains.push(node); return node; }
  createStereoPanner() { return new AudioNode(); }
  createBufferSource() { const node = new AudioNode(); this.sources.push(node); return node; }
  decodeAudioData() { this.decoded++; return this.defer ? new Promise(resolve => this.pending.push(resolve)) : Promise.resolve({ duration: 3 }); }
  resume() { this.resumes++; this.state = "running"; return Promise.resolve(); }
}
interface Deck { play(channel: string, track: string): void; pause(channel: string): void; resume(channel: string): void; stop(channel: string): void; cue(sound: string): void }
class Director {
  deck: Deck;
  ends: string[] = [];
  constructor(deck: Deck) { this.deck = deck; }
  ended(channel: string) { this.ends.push(channel); }
}
const settle = async () => { for (let i = 0; i < 12; i++) await Promise.resolve(); };
async function withAudio(run: (engine: BrowserGameAudio, ctx: Context, timers: Map<number, () => void>, fetches: string[], setFailure: (value: boolean) => void) => Promise<void>) {
  const keys = ["AudioContext", "Audio", "navigator", "fetch", "setTimeout", "clearTimeout"];
  const old = keys.map(key => Object.getOwnPropertyDescriptor(globalThis, key));
  const timers = new Map<number, () => void>(); let serial = 0; let fail = false;
  const fetches: string[] = [];
  const media: { plays: number; src: string; loop: boolean }[] = [];
  class Media {
    plays = 0; loop = false; src: string;
    constructor(src: string) { this.src = src; media.push(this); }
    setAttribute() {}
    play() { this.plays++; return Promise.resolve(); }
  }
  const navigator = { audioSession: { type: "auto" } };
  const replacements = {
    AudioContext: Context, Audio: Media, navigator,
    fetch: (file: string) => { fetches.push(file); return Promise.resolve({ ok: !fail, status: fail ? 503 : 200, arrayBuffer: () => Promise.resolve(new ArrayBuffer(1)) }); },
    setTimeout: (callback: () => void) => { const id = ++serial; timers.set(id, callback); return id; },
    clearTimeout: (id: number) => { timers.delete(id); },
  };
  for (const [key, value] of Object.entries(replacements)) Object.defineProperty(globalThis, key, { configurable: true, writable: true, value });
  const bank = {
    MIX: { master: -1, sfx: -2, ui: -3, amb: -4, music: -5, fightMusic: -6, radioMusic: -7 },
    SOUNDS: { "ui.tap": { files: ["tap.wav"], bus: "ui" }, "amb.hall": { files: ["hall.wav"], bus: "amb" } },
    FILES: {}, TRACKS: { first: { file: "first.wav" }, second: { file: "second.wav" } },
    MusicDirector: Director,
    createRadio: () => ({ input: new AudioNode(), output: new AudioNode(), place() {}, setHiss() {} }),
  };
  try {
    const engine = new BrowserGameAudio({ baseUrl: "/sound/", random: () => 0.5 }, bank);
    const ctx = Context.instances.at(-1)!;
    await engine.unlock(); await engine.unlock();
    assert.equal(ctx.resumes, 1);
    assert.equal(media.length, 1, "iOS keepalive must be reused");
    assert.equal(media[0]!.plays, 2);
    assert.equal(media[0]!.loop, true);
    assert.equal(navigator.audioSession.type, "playback");
    const wav = Buffer.from(media[0]!.src.split(",")[1]!, "base64");
    assert.equal(wav.subarray(0, 4).toString(), "RIFF");
    assert.equal(wav.readUInt32LE(40), 4000);
    assert.ok(wav.subarray(44).every(value => value === 128));
    await run(engine, ctx, timers, fetches, value => { fail = value; });
  } finally {
    keys.forEach((key, i) => { if (old[i]) Object.defineProperty(globalThis, key, old[i]!); else Reflect.deleteProperty(globalThis, key); });
  }
}

test("Rust audio caches decoded effects, retries failed downloads, and schedules late decode immediately", async () => withAudio(async (engine, ctx, _timers, fetches, setFailure) => {
  await Promise.all([engine.preload(["ui.tap"]), engine.preload(["ui.tap"])]);
  assert.deepEqual(fetches, ["/sound/tap.wav"]);
  assert.equal(ctx.decoded, 1);
  engine.play("ui.tap", { delay: 0.5 }); ctx.currentTime = 20;
  await settle();
  assert.equal(ctx.sources.at(-1)!.starts[0]![0], 20);
  setFailure(true); await assert.rejects(engine.preload(["amb.hall"])); await settle();
  setFailure(false); await engine.preload(["amb.hall"]);
  assert.equal(fetches.filter(file => file.endsWith("hall.wav")).length, 2);
}));

test("Rust music deck ignores stale decode, keeps pause position, and reports natural endings", async () => withAudio(async (engine, ctx, timers) => {
  const director = engine.music as Director;
  ctx.defer = true;
  director.deck.play("radio", "first"); director.deck.play("radio", "second"); await settle();
  assert.equal(ctx.pending.length, 2);
  ctx.pending[0]!({ duration: 30 }); await settle();
  assert.equal(ctx.sources.length, 0, "superseded decode must never play");
  ctx.pending[1]!({ duration: 30 }); await settle();
  assert.equal(ctx.sources.length, 1);
  ctx.currentTime = 17; director.deck.pause("radio");
  const prior = ctx.sources[0]!;
  for (const [id, callback] of [...timers]) { timers.delete(id); callback(); }
  assert.equal(prior.onended, null);
  assert.equal(prior.disconnected, true);
  director.deck.resume("radio");
  const resumed = ctx.sources.at(-1)!;
  assert.deepEqual(resumed.starts[0], [0, 7]);
  resumed.onended?.();
  assert.deepEqual(director.ends, ["radio"]);
  director.deck.stop("radio");
  for (const [id, callback] of [...timers]) { timers.delete(id); callback(); }
}));

test("Rust ambience cancellation prevents an in-flight buffer starting after leaving the room", async () => withAudio(async (engine, ctx, timers) => {
  ctx.defer = true; engine.setAmbience(["amb.hall"], 2); await settle();
  engine.setAmbience([], 0.1);
  ctx.pending[0]!({ duration: 4 }); await settle();
  assert.equal(ctx.sources.length, 0);
  for (const [id, callback] of [...timers]) { timers.delete(id); callback(); }
}));
