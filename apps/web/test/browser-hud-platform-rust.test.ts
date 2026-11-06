import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import ts from "typescript";
import {
  RustFightLog, RustRadio, liquid_glass, store_get, store_set, load_local_loadout,
  save_local_loadout, guess_quality, start_quality, keep_quality, callRust, audioSamples,
} from "../../../packages/runtime/src/index.ts";

let fixtureId = 0;
async function original(file: string, bindings: Record<string, unknown> = {}): Promise<Record<string, any>> {
  const key = `__hudPlatformFixture${++fixtureId}`;
  Object.defineProperty(globalThis, key, { configurable: true, value: bindings });
  const source = readFileSync(new URL(`../../../crates/game-wasm/reference/browser90/${file}`, import.meta.url), "utf8");
  const compiled = ts.transpileModule(source, {
    compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 },
  }).outputText.replace(/^import .*?;\s*$/gm, "");
  const code = `const { ${Object.keys(bindings).join(",")} } = globalThis.${key};\n${compiled}`;
  try { return await import(`data:text/javascript;base64,${Buffer.from(code).toString("base64")}`); }
  finally { Reflect.deleteProperty(globalThis, key); }
}
async function globals<T>(values: Record<string, unknown>, run: () => T | Promise<T>): Promise<T> {
  const previous = Object.keys(values).map(key => [key, Object.getOwnPropertyDescriptor(globalThis, key)] as const);
  for (const [key, value] of Object.entries(values)) Object.defineProperty(globalThis, key, { configurable: true, writable: true, value });
  try { return await run(); }
  finally { for (const [key, descriptor] of previous) {
    if (descriptor) Object.defineProperty(globalThis, key, descriptor); else Reflect.deleteProperty(globalThis, key);
  } }
}

class Element {
  tag: string;
  children: Element[] = [];
  attributes: Record<string, string> = {};
  styles: Record<string, string> = {};
  classes = new Set<string>();
  trace: string[] = [];
  style = { setProperty: (key: string, value: string) => { this.styles[key] = value; }, position: "" };
  classList = {
    add: (name: string) => { this.trace.push(`add:${name}`); this.classes.add(name); },
    remove: (name: string) => { this.trace.push(`remove:${name}`); this.classes.delete(name); },
  };
  className = "";
  hidden = true;
  scrollTop = 0;
  scrollHeight = 137;
  width = 48;
  offsetHeight = 24;
  radius = "6px";
  isConnected = true;
  html = "";
  text = "";
  constructor(tag = "div") { this.tag = tag; }
  get offsetWidth() { this.trace.push("layout"); return this.width; }
  get innerHTML() { return this.html; }
  set innerHTML(value: string) { this.html = value; this.text = ""; this.children = []; }
  get textContent() { return this.text; }
  set textContent(value: string) { this.text = value; this.html = ""; this.children = []; }
  setAttribute(key: string, value: unknown) { this.attributes[key] = String(value); }
  appendChild(child: Element) { this.children.push(child); return child; }
  append(...children: Element[]) { this.children.push(...children); }
  querySelectorAll() { return this.children; }
  snapshot(): unknown {
    return { tag: this.tag, className: this.className, hidden: this.hidden, scrollTop: this.scrollTop,
      html: this.html, text: this.text, attributes: this.attributes, styles: this.styles, position: this.style.position,
      classes: [...this.classes], trace: this.trace, children: this.children.map(child => child.snapshot()) };
  }
}

test("Rust fight log matches frozen DOM behavior, reset and the fresh animation restart", async () => globals({
  document: { createElement: (tag: string) => new Element(tag) },
}, async () => {
  const { FightLog } = await original("apps/web/src/hud/log.ts", { callRust });
  const report = { turn: 3, distanceBefore: 7, distanceAfter: 6, limit: 12,
    actions: [{ type: "dash" }, { type: "shoot", zone: "body", move: 0 }], defend: [[], []], moves: [1, 0],
    attacks: [null, { weapon: "rifle", zone: "body", chance: 70, roll: 20, hit: true, blocked: false, crit: false, damage: 15 }], hp: [85, 100], afk: [false, false] };
  function exercise(create: (els: Record<string, Element>) => any) {
    const els = { list: new Element("ul"), ticker: new Element(), button: new Element("button"), sheet: new Element() };
    const log = create(els);
    const snapshots: unknown[] = [];
    const snap = () => snapshots.push({ open: log.open, els: Object.fromEntries(Object.entries(els).map(([key, el]) => [key, el.snapshot()])) });
    log.reset("Начало боя"); snap();
    assert.match(els.list.innerHTML, /class="empty"/);
    log.setOpen(true); snap();
    log.add(report, ["Тень", "Искра"], ["pistol", "rifle"]); snap();
    assert.equal(els.list.children.length, 1);
    assert.deepEqual(els.button.trace, ["remove:fresh", "layout", "add:fresh"]);
    log.add({ ...report, turn: 4 }, ["Тень", "Искра"], ["pistol", "rifle"]); snap();
    assert.equal(els.list.children.length, 2);
    log.note("Нельзя стрелять"); log.setOpen(false); snap();
    log.reset("Другой бой"); snap();
    log.add(report, ["Тень", "Искра"], ["pistol", "rifle"]); snap();
    assert.equal(els.list.children.length, 1, "reset must clear the previous entries");
    log.free?.();
    return snapshots;
  }
  assert.deepEqual(exercise(els => new RustFightLog(els)), exercise(els => new FightLog(els)));
}));

class ResizeObserverFake {
  static instances: ResizeObserverFake[] = [];
  observed = new Set<Element>();
  observations = 0;
  callback: (entries: { target: Element }[]) => void;
  constructor(callback: (entries: { target: Element }[]) => void) { this.callback = callback; ResizeObserverFake.instances.push(this); }
  observe(target: Element) { this.observed.add(target); this.observations++; }
  unobserve(target: Element) { this.observed.delete(target); }
  disconnect() { this.observed.clear(); }
  resize(targets = [...this.observed]) { this.callback(targets.map(target => ({ target }))); }
}
function glassFixture() {
  const root = new Element(), body = new Element("body");
  const pixels: { width: number; height: number; bytes: Uint8ClampedArray }[] = [];
  for (const [width, height, radius] of [[24, 12, 4], [37, 19, 6.5], [13, 13, 20], [90, 32, 0], [37, 19, 6.6], [0, 0, 0]]) {
    const el = new Element(); el.width = width!; el.offsetHeight = height!; el.radius = `${radius}px`; root.appendChild(el);
  }
  const document = {
    body,
    createElementNS: (_ns: string, tag: string) => new Element(tag),
    createElement(tag: string) {
      if (tag !== "canvas") return new Element(tag);
      const canvas = { width: 0, height: 0,
        getContext: () => ({
          createImageData: (width: number, height: number) => ({ data: new Uint8ClampedArray(width * height * 4) }),
          putImageData: (image: { data: Uint8ClampedArray }) => {
            pixels.push({ width: canvas.width, height: canvas.height, bytes: image.data.slice() });
          },
        }),
        toDataURL: () => `data:image/png;base64,${Buffer.from(pixels.at(-1)!.bytes).toString("base64")}`,
      };
      return canvas;
    },
  };
  return { root, body, pixels, document };
}
test("Rust liquid glass retains frozen pixels, rounded-size cache and Chromium-only observation", async () => {
  const fixture = glassFixture();
  ResizeObserverFake.instances = [];
  await globals({ document: fixture.document, navigator: { userAgentData: { brands: [{ brand: "Firefox" }] } }, ResizeObserver: ResizeObserverFake,
    getComputedStyle: (el: Element) => ({ borderTopLeftRadius: el.radius }) }, async () => {
    liquid_glass(fixture.root);
    assert.equal(ResizeObserverFake.instances.length, 0);
    Object.defineProperty(globalThis, "navigator", { configurable: true, value: { userAgentData: { brands: [{ brand: "Chromium" }] } } });
    const { liquidGlass } = await original("apps/web/src/hud/liquid.ts");
    liquidGlass(fixture.root); liquidGlass(fixture.root);
    const oldObserver = ResizeObserverFake.instances.at(-1)!;
    oldObserver.resize();
    const expected = { body: fixture.body.snapshot(), pixels: fixture.pixels, styles: fixture.root.children.map(el => el.styles) };
    assert.equal(fixture.pixels.length, 4, "same rounded dimensions must reuse a filter");
    assert.equal(oldObserver.observations, 6);

    const current = glassFixture();
    Object.defineProperty(globalThis, "document", { configurable: true, value: current.document });
    liquid_glass(current.root); liquid_glass(current.root);
    const observer = ResizeObserverFake.instances.at(-1)!;
    assert.notEqual(observer, oldObserver);
    observer.resize();
    assert.deepEqual({ body: current.body.snapshot(), pixels: current.pixels, styles: current.root.children.map(el => el.styles) }, expected);
    assert.equal(observer.observations, 6, "calling again must not reobserve existing elements");
    current.root.children[0]!.isConnected = false;
    observer.resize([current.root.children[0]!]);
    assert.equal(observer.observed.has(current.root.children[0]!), false);
  });
});

class AudioParameter {
  value = 1;
  events: number[][] = [];
  setTargetAtTime(...args: number[]) { this.events.push(args); }
}
class AudioNodeFake {
  id: number; kind: string;
  frequency = new AudioParameter(); Q = new AudioParameter(); gain = new AudioParameter(); pan = new AudioParameter();
  type = ""; oversample = ""; loop = false;
  curve: Float32Array | null = null;
  buffer: AudioBufferFake | null = null;
  targets: number[] = []; starts = 0; stops = 0; disconnects = 0;
  constructor(id: number, kind: string) { this.id = id; this.kind = kind; }
  connect(node: AudioNodeFake) { this.targets.push(node.id); return node; }
  disconnect() { this.disconnects++; }
  start() { this.starts++; }
  stop() { this.stops++; }
  snapshot() {
    return { id: this.id, kind: this.kind, type: this.type, oversample: this.oversample, loop: this.loop,
      frequency: this.frequency, Q: this.Q, gain: this.gain, pan: this.pan,
      curve: this.curve, buffer: this.buffer, targets: this.targets, starts: this.starts };
  }
}
class AudioBufferFake {
  channels: Float32Array[];
  sampleRate: number;
  constructor(channels: number, length: number, rate: number) {
    this.channels = Array.from({ length: channels }, () => new Float32Array(length)); this.sampleRate = rate;
  }
  copyToChannel(samples: Float32Array, channel: number) { this.channels[channel]!.set(samples); }
}
class AudioContextFake {
  sampleRate = 8000;
  currentTime = 12.5;
  nodes: AudioNodeFake[] = [];
  node(kind: string) { const node = new AudioNodeFake(this.nodes.length, kind); this.nodes.push(node); return node; }
  createBiquadFilter() { return this.node("biquad"); }
  createWaveShaper() { return this.node("shaper"); }
  createGain() { return this.node("gain"); }
  createStereoPanner() { return this.node("panner"); }
  createConvolver() { return this.node("convolver"); }
  createBufferSource() { return this.node("source"); }
  createBuffer(channels: number, length: number, rate: number) { return new AudioBufferFake(channels, length, rate); }
}
test("Rust radio matches frozen node graph, DSP samples and automation using fake WebAudio", async () => {
  const { createRadio } = await original("packages/audio/src/radio.ts", { callRust, audioSamples });
  const random = Math.random;
  const exercise = (create: (ctx: AudioContextFake) => any) => {
    let index = 0;
    Math.random = () => [0.12345, 0.76543][index++]!;
    const ctx = new AudioContextFake(), radio = create(ctx);
    assert.equal(index, 2, "room impulse and hiss each consume one seed");
    radio.place(-0.6, 0); radio.setHiss(true);
    ctx.currentTime = 15; radio.place(0.7, 0.8); radio.setHiss(false);
    const result = { input: radio.input.id, output: radio.output.id, nodes: ctx.nodes.map(node => node.snapshot()) };
    if (radio.dispose) {
      radio.dispose();
      const hiss = ctx.nodes.find(node => node.kind === "source")!;
      assert.equal(hiss.stops, 1);
      assert.equal(hiss.disconnects, 1);
      assert.equal(radio.input.disconnects, 1);
      assert.equal(radio.output.disconnects, 1);
      radio.free();
    }
    return result;
  };
  try { assert.deepEqual(exercise(ctx => new RustRadio(ctx)), exercise(ctx => createRadio(ctx))); }
  finally { Math.random = random; }
});

test("Rust storage and loadouts tolerate denied localStorage getters and failed operations", async () => globals({ localStorage: null }, async () => {
  const expected = callRust("client.load", [null]);
  Object.defineProperty(globalThis, "localStorage", { configurable: true, get() { throw new Error("private mode"); } });
  assert.equal(store_get("test"), null);
  assert.doesNotThrow(() => store_set("test", "value"));
  assert.deepEqual(load_local_loadout(undefined), expected);
  assert.equal(save_local_loadout(expected, undefined), false);
  const denied = { getItem() { throw new Error("denied read"); }, setItem() { throw new Error("quota"); } };
  assert.deepEqual(load_local_loadout(denied), expected);
  assert.equal(save_local_loadout(expected, denied), false);
  const saved = new Map<string, string>();
  const storage = { getItem: (key: string) => saved.get(key) ?? null, setItem: (key: string, value: string) => { saved.set(key, value); } };
  assert.equal(save_local_loadout(expected, storage), true);
  assert.deepEqual(load_local_loadout(storage), expected);
  assert.equal(save_local_loadout({ invalid: true }, storage), false);
}));

test("Rust quality selection retains GPU, mobile, preference and URL behavior", async () => globals({
  navigator: {}, window: globalThis, location: { search: "" }, matchMedia: () => ({ matches: false }), localStorage: null,
}, async () => {
  const values = new Map<string, string>();
  const store = { get: (key: string) => values.get(key) ?? null, set: (key: string, value: string) => { values.set(key, value); } };
  Object.defineProperty(globalThis, "localStorage", { configurable: true, value: { getItem: store.get, setItem: store.set } });
  const reference = await original("apps/web/src/arena/quality.ts", { callRust, store });
  for (const gpu of ["NVIDIA RTX 4070", "Intel UHD Graphics", "SwiftShader", "Adreno (TM) 530", "Apple GPU"]) {
    for (const mobile of [false, true]) for (const coarse of [false, true]) for (const low of [false, true]) {
      Object.defineProperty(globalThis, "navigator", { configurable: true, value: { userAgent: mobile ? "Android mobile" : "Desktop", deviceMemory: low ? 2 : 16, hardwareConcurrency: low ? 4 : 12 } });
      Object.defineProperty(globalThis, "matchMedia", { configurable: true, value: () => ({ matches: coarse }) });
      for (const debug of [false, true]) {
        const gl = { RENDERER: 1, getExtension: () => debug ? { UNMASKED_RENDERER_WEBGL: 2 } : null, getParameter: (key: number) => { assert.equal(key, debug ? 2 : 1); return gpu; } };
        assert.equal(guess_quality(gl), reference.guessQuality(gl));
      }
    }
  }
  assert.equal(guess_quality(null), "low");
  assert.deepEqual(start_quality(null), reference.startQuality(null));
  keep_quality("high");
  assert.equal(values.get("cd-quality"), "high");
  assert.deepEqual(start_quality(null), { id: "high", picked: true });
  Object.defineProperty(globalThis, "location", { configurable: true, value: { search: "?q=medium" } });
  assert.deepEqual(start_quality(null), reference.startQuality(null));
  assert.deepEqual(start_quality(null), { id: "medium", picked: true });
}));

test("Backend selection uses the quality detected on a new device", async () => globals({
  location: { search: "" },
  navigator: { userAgent: "Android mobile", deviceMemory: 8, hardwareConcurrency: 8 },
  matchMedia: () => ({ matches: true }),
}, async () => {
  // Two WebGL measurements are already stored. A measured device must try WebGPU next even
  // when its low preset came from auto-detection rather than ?q= or cd-quality.
  const record = JSON.stringify({ v: 1, gl: [
    { fps: 24, p95: 50, scale: 0.8 }, { fps: 22, p95: 55, scale: 0.8 },
  ], gpu: [], tries: 0, broken: false });
  Object.defineProperty(globalThis, "localStorage", { configurable: true, value: {
    getItem: (key: string) => key === "cd-backend-low" ? record : null,
    setItem: () => { throw new Error("selection must not write during this check"); },
  } });
  const gl = {
    RENDERER: 1,
    getExtension: () => null,
    getParameter: (key: number) => { assert.equal(key, 1); return "Mali-G52"; },
  };
  Object.defineProperty(globalThis, "document", { configurable: true, value: {
    createElement: () => ({ getContext: () => gl }),
  } });
  const { arena_backend } = await import("../../../packages/runtime/src/index.ts");
  assert.equal(arena_backend(true), "webgpu");
}));
