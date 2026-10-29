// Behavioral parity for the Rust render graph. No WebGL/audio device is opened.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { pathToFileURL } from "node:url";
import { resolve } from "node:path";
import ts from "typescript";
import * as Three from "three";
import { RustArena, RustLens, RustTextureTiers, callRust, motionCompute } from "../packages/runtime/src/index.ts";

const root = resolve(import.meta.dirname, "..");
const shaders = await import(pathToFileURL(resolve(root, "crates/game-wasm/reference/browser/lens-shaders.ts")).href);
const quality = callRust("client.data", []).QUALITY;
let fixtureNumber = 0;
async function original(file, bindings) {
  const key = `__renderFixture${++fixtureNumber}`;
  globalThis[key] = bindings;
  const source = readFileSync(resolve(root, "crates/game-wasm/reference/browser", file), "utf8");
  const output = ts.transpileModule(source, { compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 } }).outputText
    .replace(/^import .*?;\s*$/gm, "");
  const module = await import(`data:text/javascript;base64,${Buffer.from(`const { ${Object.keys(bindings).join(",")} } = globalThis.${key};\n${output}`).toString("base64")}`);
  delete globalThis[key];
  return module;
}
const flush = () => new Promise(resolve => setImmediate(resolve));
function parity(actual, expected, path = "render") {
  if (typeof actual === "number" && typeof expected === "number") {
    assert.ok(Math.abs(actual - expected) <= 1e-12 * Math.max(1, Math.abs(expected)), `${path}: ${actual} != ${expected}`);
  } else if (actual && expected && typeof actual === "object" && typeof expected === "object") {
    assert.deepEqual(Object.keys(actual), Object.keys(expected), path);
    for (const key of Object.keys(actual)) parity(actual[key], expected[key], `${path}.${key}`);
  } else assert.equal(actual, expected, path);
}
function deferred() { let resolve, reject; const promise = new Promise((a, b) => { resolve = a; reject = b; }); return { promise, resolve, reject }; }
function fakeCanvas() {
  return { width: 1, height: 1, getContext() { return {
    createRadialGradient: () => ({ addColorStop() {} }), fillRect() {}, drawImage() {},
    getImageData: (_x, _y, w, h) => ({ data: Uint8ClampedArray.from({ length: w * h * 4 }, (_, i) => i % 4 === 3 ? 255 : 210 + i % 40) }),
  }; } };
}
const saved = { document: globalThis.document, location: globalThis.location, devicePixelRatio: globalThis.devicePixelRatio, fetch: globalThis.fetch, window: globalThis.window };
globalThis.document = { createElement: () => fakeCanvas(), addEventListener() {}, removeEventListener() {}, hidden: false };
globalThis.location = { search: "" }; globalThis.devicePixelRatio = 1.5; globalThis.window = globalThis;

function normalized(value) {
  if (value == null || typeof value !== "object") return value;
  if (value.isTexture) return { texture: value.userData.label ?? "texture", width: value.image?.width, height: value.image?.height };
  if (value.isColor) return value.toArray();
  if (value.isVector2 || value.isVector3 || value.isMatrix4) return value.toArray();
  if (Array.isArray(value)) return value.map(normalized);
  return Object.fromEntries(Object.entries(value).map(([k, v]) => [k, normalized(v)]));
}
function makeHost() {
  const loads = [], images = [], trace = [], passes = [], targets = [], renderers = [], viewmodels = [];
  class Timer { update(time = 125) { this.time = time; } connect() {} dispose() {} reset() { this.time = 0; } getDelta() { return 1 / 60; } getElapsed() { return (this.time ?? 125) / 1000; } }
  class TextureLoader {
    loadAsync(url) { const request = { url, ...deferred() }; loads.push(request); return request.promise; }
    load(url, done) { this.loadAsync(url).then(done); }
  }
  class ImageLoader { loadAsync(url) { const request = { url, ...deferred() }; images.push(request); return request.promise; } }
  class RenderTarget extends Three.WebGLRenderTarget {
    constructor(...args) { super(...args); this.index = targets.length; this.texture.userData.label = `target${this.index}`; targets.push(this); }
    setSize(w, h) { super.setSize(w, h); trace.push(["size", this.index, w, h]); }
    dispose() { trace.push(["dispose", this.index]); super.dispose(); }
  }
  class Renderer {
    constructor() { this.extensions = { has: () => true }; this.capabilities = { maxTextureSize: 8192, getMaxAnisotropy: () => 8 }; this.shadowMap = { autoUpdate: true, enabled: true }; this.ratio = 1; this.width = this.height = 1; renderers.push(this); }
    setClearColor(...args) { trace.push(["clearColor", ...args]); }
    setRenderTarget(t) { this.target = t; trace.push(["target", t?.index ?? null]); }
    clear() { trace.push(["clear"]); }
    render(scene, camera) { this.scene = scene; this.camera = camera; trace.push(["render", camera.layers.mask, this.shadowMap.autoUpdate]); }
    setPixelRatio(v) { this.ratio = v; }
    setSize(w, h) { this.width = w; this.height = h; }
    getDrawingBufferSize(out) { return out.set(Math.floor(this.width * this.ratio), Math.floor(this.height * this.ratio)); }
    getContext() { return this; }
    setAnimationLoop(fn) { this.loop = fn; }
    dispose() { this.disposals = (this.disposals ?? 0) + 1; }
  }
  class FullScreenQuad { dispose() {} render(renderer) { passes.push({ target: renderer.target?.index ?? null, shader: this.material.fragmentShader.replaceAll("\r\n", "\n"), defines: { ...this.material.defines }, uniforms: normalized(Object.fromEntries(Object.entries(this.material.uniforms).map(([k, v]) => [k, v.value]))) }); } }
  class PMREMGenerator { fromEquirectangular(t) { return { texture: t.clone() }; } dispose() {} }
  const THREE = { ...Three, Timer, TextureLoader, ImageLoader, WebGLRenderTarget: RenderTarget, WebGLRenderer: Renderer, PMREMGenerator };
  return { THREE, FullScreenQuad, loads, images, trace, passes, targets, renderers, viewmodels };
}
function texture(label, width = 512, height = 1024) { const t = new Three.Texture({ width, height }); t.userData.label = label; return t; }
const look = { smear: .045, soft: .0025, barrel: .07, vignette: .97, glow: .45, contrast: .3, near: 2.5, saturation: .55, tint: 0xf1f4f6, dof: .035, dofNear: .3, rays: .9, haze: .03, hazeColor: 0xdfe6ea };

async function exerciseLens(useRust) {
  const h = makeHost(), renderer = new h.THREE.WebGLRenderer();
  const Old = useRust ? null : (await original("lens.ts", { THREE: h.THREE, FullScreenQuad: h.FullScreenQuad, motionCompute, ...shaders })).Lens;
  const lens = useRust ? new RustLens(h, renderer) : new Old(renderer);
  lens.setSize(1080, 1920, 720, 1280, { x: .03, y: .1, w: .9, h: .8 });
  lens.setQuality(quality.medium); lens.setLook(look, 1.05); lens.setHorizon(.4);
  lens.setBackdrop("hall.jpg", "hall@2x.jpg", "depth.png");
  // Upgrade while the original image is in flight; the lower-resolution completion is stale.
  lens.setQuality(quality.ultra);
  for (const request of h.loads) request.resolve(texture(request.url));
  await flush();
  const scene = new Three.Scene(), camera = new Three.PerspectiveCamera(55, 9 / 16, .1, 80);
  camera.position.set(0, 1.65, 0); camera.updateMatrixWorld(); camera.layers.mask = 5;
  const light = new Three.DirectionalLight(); light.castShadow = true; light.shadow.map = { depthTexture: texture("shadow") };
  lens.render(scene, camera, 1, { enemyDepth: 7, light });
  assert.equal(camera.layers.mask, 5); assert.equal(renderer.shadowMap.autoUpdate, true);
  assert.equal(lens.dof, look.dof);
  const screen = Array.from(lens.toScreen(.3, .7));
  const direction = lens.sunDirection(camera, new Three.Vector3()).toArray();
  lens.setQuality(quality.low); lens.setBackdrop("next.jpg", undefined, undefined);
  for (const request of h.loads.filter(r => r.url === "next.jpg")) request.resolve(texture(request.url));
  await flush();
  lens.render(scene, camera, 1, { enemyDepth: 1, light });
  assert.equal(lens.dof, 0); assert.equal(lens.raysTexture, null);
  return { passes: h.passes, targets: h.targets.map(t => ({ w: t.width, h: t.height, samples: t.samples })), screen, direction };
}

async function testTextureTiers() {
  const h = makeHost();
  globalThis.fetch = async () => ({ ok: true, json: async () => ({ sizes: [2048, 4096], materials: { suit: "tex/suit" } }) });
  const albedo = texture("color", 1024, 1024), orm = texture("orm", 1024, 1024);
  const material = new Three.MeshStandardMaterial({ map: albedo, roughnessMap: orm, metalnessMap: orm }); material.name = "suit";
  const root = new Three.Group(); root.add(new Three.Mesh(new Three.BoxGeometry(), material));
  const tiers = new RustTextureTiers(h, [root, root], "models/persona.glb");
  const low = tiers.set(512, 2); await low;
  assert.equal(material.map.image.width, 512); assert.equal(material.roughnessMap, material.metalnessMap); assert.equal(tiers.current, 512);
  const older = tiers.set(2048, 4); await flush(); const current = tiers.set(4096, 8); await flush();
  assert.equal(h.images.filter(v => v.url.endsWith("2048.webp")).length, 2);
  assert.equal(h.images.filter(v => v.url.endsWith("4096.webp")).length, 2);
  for (const request of h.images.filter(v => v.url.endsWith("4096.webp"))) request.resolve({ width: 4096, height: 4096 });
  await current;
  for (const request of h.images.filter(v => v.url.endsWith("2048.webp"))) request.resolve({ width: 2048, height: 2048 });
  await older;
  assert.equal(tiers.current, 4096); assert.equal(material.map.image.width, 4096); assert.equal(material.map.anisotropy, 8);
  assert.equal(material.roughnessMap, material.metalnessMap);
  const failed = tiers.set(2048, 2); await flush();
  for (const request of h.images.slice(-2)) request.reject(new Error("missing optional tier"));
  await failed; assert.equal(tiers.current, 1024); assert.equal(material.map, albedo); assert.equal(material.map.anisotropy, 2);
}

function arenaHost() {
  const h = makeHost();
  const events = [];
  class Motion {
    constructor() { this.stances = {}; this.gait = null; this.attackKind = null; this.pistolReady = 1; this.planted = true; }
    arm(gun) { this.gun = gun; this.onHold?.(gun); }
    walk(kind, phase) { this.gait = kind; events.push(["walk", kind, phase]); }
    pace(rate) { events.push(["pace", rate]); }
    attack(kind, when) { this.attackKind = kind; this.pending = kind; events.push(["attack", kind, when]); }
    update() { if (this.pending) { this.onMark?.(this.pending); this.pending = null; } }
    settle() {} stand() { this.gait = null; } fall() { this.attackKind = null; }
    reset() { this.gait = null; this.attackKind = null; }
  }
  class Pose { save() {} restore() {} }
  class Feet { update() {} }
  class Collapse { begin() {} apply(value) { events.push(["fall", value]); } }
  class HoldOut { apply() {} }
  class Hand {
    constructor(model, _grip, _steel, opts) { this.mount = new Three.Group(); model.add(this.mount); this.opts = opts; }
    hold(kind) { events.push(["hold", kind]); }
    fit(mods) { events.push(["fit", mods]); }
  }
  class Flash {
    constructor() { this.sprite = new Three.Sprite(); this.life = 0; }
    fire(_hand, size) { this.life = size; }
    stop() { this.life = 0; }
    update(dt) { this.life = Math.max(0, this.life - dt); return this.life; }
  }
  class Viewmodel {
    constructor() { h.viewmodels.push(this); this.shots = 0; }
    shot() { this.shots++; }
    off() { this.active = false; }
    update(_dt, gun, active, stance) { this.active = gun; this.attack = active; this.stance = stance; }
  }
  class Dust { constructor() { this.points = new Three.Points(); } setCount(value) { this.count = value; } update() {} }
  class Lens {
    constructor(renderer) { this.renderer = renderer; this.raysTexture = null; this.dof = 0; }
    setQuality(q) { this.quality = q; } setBackdrop() {} setHorizon() {} setLook() {} setSize() {}
    toScreen(x, y) { return [x, y]; } sunDirection(_camera, out) { return out.set(0, .5, -1).normalize(); }
    render(scene, camera) { this.renderer.scene = scene; this.renderer.camera = camera; }
    dispose() {}
  }
  class TextureTiers { async set() {} }
  const scene = new Three.Group();
  scene.add(new Three.Mesh(new Three.BoxGeometry(.4, 1.7, .2), new Three.MeshStandardMaterial()));
  for (const [i, name] of ["head", "top", "neck", "spine", "spine1", "spine2", "hips", "rLeg", "rUpLeg"].entries()) {
    const bone = new Three.Bone(); bone.name = name; bone.position.set(i * .01, [1.6, 1.75, 1.5, 1, 1.1, 1.3, .85, .3, .7][i], 0); scene.add(bone);
  }
  const model = { scene, animations: [new Three.AnimationClip("walk", 1, []), new Three.AnimationClip("run", .5, [])] };
  Object.assign(h, {
    Motion, Pose, Feet, Collapse, HoldOut, Flash, Hand, Viewmodel, Dust, Lens, TextureTiers,
    clone: value => value.clone(true), dress() {}, KIT_A: {}, tune() {}, FALL_TIME: 1.4,
    armsOnly: () => () => {}, findBones: value => Object.fromEntries(value.children.filter(o => o.isBone).map(o => [o.name, o])),
    pitch: (bone, angle) => { if (bone) bone.rotation.x += angle; }, gaitOf: () => ({ speed: 1, phase: 0 }),
    gripOf: () => ({}), loadModel: async () => ({ ...model, scene: model.scene.clone(true) }), loadGuns: async () => ({}),
    quality: quality.medium, events,
  });
  return h;
}
const frame = {
  id: "hall", name: "Hall", src: "hall.jpg", w: 941, h: 1672, top: 0, mapId: "saltHall", time: "day", weather: "clear",
  cal: { fov: 55, camH: 2, feet: [.572, .608], height: .212, me: { x: -.15, y: .17, z: -.66 }, back: .45, fp: { camH: 1.65, near: .88 } },
  look: { exposure: 1.05, rim: 0xe8eef2, fill: 0xc7ccd0, fog: 0x8e9497, sky: 0xc3cacd, ground: 0x2b2d2e, puddle: 0, lens: look },
};
async function exerciseArena(useRust) {
  const h = arenaHost();
  const bindings = { ...h, callRust, motionCompute, SkeletonUtils: { clone: h.clone }, QUALITY: quality };
  const Old = useRust ? null : (await original("arena.ts", bindings)).Arena;
  const arena = useRust ? new RustArena(h, fakeCanvas(), { wide: .62, fpkey: 2.5 }) : new Old(fakeCanvas());
  arena.setFrame(frame); arena.resize(720, 1280, { x: 0, y: .02, w: 1, h: .95 });
  arena.resume(); assert.equal(h.renderers[0].loop, undefined);
  await arena.load("same.glb", "same.glb", 7);
  for (const request of h.loads) request.resolve(texture(request.url)); await flush();
  assert.equal(arena.ready, true); assert.equal(typeof h.renderers[0].loop, "function");
  let callbackFrames = 0;
  arena.onFrame = () => { arena.zoneOnScreen("head"); callbackFrames++; };
  arena.arm("me", "rifle", { sight: "a" }); arena.arm("enemy", "pistol", {});
  arena.setFirstPerson(true); arena.move(5, 1, 1); arena.attack("me", "rifle", .1); arena.attack("enemy", "pistol", .1);
  for (let i = 0; i < 8; i++) h.renderers[0].loop(1000 + i * 16.7);
  arena.hit("enemy", "head", "blocked"); arena.hit("me", "body", "hit");
  for (let i = 0; i < 3; i++) h.renderers[0].loop(1200 + i * 16.7);
  arena.fall("enemy"); h.renderers[0].loop(1300);
  const screen = ["head", "body", "groin", "legs"].map(zone => Array.from(arena.zoneOnScreen(zone)));
  const renderer = h.renderers[0];
  const snapshot = renderer.scene.children.map(o => ({
    type: o.type, position: o.position.toArray(), rotation: o.rotation.toArray(), scale: o.scale.toArray(), visible: o.visible,
    intensity: o.intensity, color: o.color?.toArray(),
  }));
  const projection = renderer.camera.projectionMatrix.toArray();
  arena.pause(); assert.equal(renderer.loop, null); assert.equal(callbackFrames, 12);
  arena.setQuality(quality.low); await arena.texturesDone; arena.setFirstPerson(false); arena.reset(7); arena.resume(); renderer.loop(1500); arena.pause();
  return { screen, snapshot, projection, events: h.events, vm: h.viewmodels.map(v => ({ shots: v.shots, active: v.active, stance: v.stance })) };
}

// New projected-hall behavior is tested separately from the frozen painted-hall
// oracle. Keep the original fixture and parity assertions unchanged above.
function projectedArenaHost(gate = Promise.resolve()) {
  const h = arenaHost(), stages = [], lenses = [];
  class ProjectedStage {
    constructor() { this.ready = false; this.travel = 0; this.directionX = 0; this.moves = []; this.loads = []; this.disposals = 0; stages.push(this); }
    load(url) {
      this.loads.push(url);
      return gate.then(() => { if (!this.disposals) this.ready = true; });
    }
    setTravel(metres, directionX = 0) { this.travel = metres; this.directionX = directionX; this.moves.push([metres, directionX]); }
    dispose() { this.disposals++; this.ready = false; }
  }
  const PaintedLens = h.Lens;
  class StageLens extends PaintedLens {
    constructor(renderer) { super(renderer); this.stage = null; lenses.push(this); }
    setStage(stage) { this.stage = stage; }
  }
  Object.assign(h, { ProjectedStage, Lens: StageLens, stages, lenses });
  return h;
}

async function testProjectedArenaMovement() {
  const h = projectedArenaHost(), arena = new RustArena(h, fakeCanvas(), { wide: .62, fpkey: 2.5 });
  try {
    arena.setFrame({ ...frame, id: "hall1" }); arena.resize(720, 1280, { x: 0, y: 0, w: 1, h: 1 });
    arena.setFirstPerson(true); arena.resume(); await arena.load("same.glb", "same.glb", 7);
    const stage = h.stages[0], renderer = h.renderers[0];
    assert.equal(stage.ready, true); assert.equal(h.lenses[0].stage, stage);
    assert.deepEqual(stage.loads, ["img/battle-stage/manifest.json"]);
    let ticks = 0;
    const stepFrames = (count = 180) => { for (let i = 0; i < count; i++) renderer.loop(++ticks * 1000 / 60); };
    const close = (a, b) => assert.ok(Math.abs(a - b) < 1e-9, `${a} != ${b}`);

    const walks = () => h.events.filter(event => event[0] === "walk").length;
    const beforeOwn = walks();
    arena.move(6, 1, 0); stepFrames();
    const ownStep = stage.travel;
    assert.ok(ownStep > .4 && ownStep < .6, `own step must move the world by about half a metre: ${ownStep}`);
    assert.equal(walks(), beforeOwn, "a stationary opponent must not walk when only our camera advances");
    assert.ok(stage.moves.some(([m]) => m > 0 && m < ownStep), "world movement must animate through intermediate positions");
    arena.move(7, -1, 0); stepFrames(); close(stage.travel, 0);

    const beforeEnemy = walks();
    arena.move(6, 0, 1); stepFrames(); close(stage.travel, 0);
    assert.ok(walks() > beforeEnemy, "enemy-only motion must animate the opponent without moving the world");

    arena.reset(7); arena.move(4, 3, 0); stepFrames();
    const rush = stage.travel;
    assert.ok(rush > 1.2 && rush < 1.5, `first-person rush must travel metres, not centimetres: ${rush}`);
    arena.setFirstPerson(false); close(stage.travel, rush);
    arena.setFirstPerson(true); close(stage.travel, rush);
    arena.reset(7); close(stage.travel, 0);

    // A view change settles an in-flight move once, and only once.
    arena.move(6, 1, 0); stepFrames(8);
    assert.ok(stage.travel > 0 && stage.travel < ownStep);
    arena.setFirstPerson(false); close(stage.travel, ownStep);
    stepFrames(); arena.setFirstPerson(true); close(stage.travel, ownStep);

    // Equal relative range can still mean both fighters have moved in world space.
    arena.reset(7); arena.move(7, 1, -1); stepFrames(); close(stage.travel, ownStep);
    arena.reset(1); arena.move(1, 3, 0); stepFrames(); close(stage.travel, 0);

    arena.dispose(); arena.dispose();
    assert.equal(stage.disposals, 1); assert.equal(renderer.loop, null); assert.equal(arena.ready, false);
  } finally { arena.free(); }
}

async function testProjectedArenaLifecycle() {
  // Switching to another hall must not wait for a pending optional projection,
  // and that stale completion must never reattach it to the new hall.
  const old = deferred(), h = projectedArenaHost(old.promise);
  const arena = new RustArena(h, fakeCanvas(), { wide: .62, fpkey: 2.5 });
  try {
    arena.setFrame({ ...frame, id: "hall1" });
    arena.setFrame({ ...frame, id: "hall2" }); arena.resume();
    assert.equal(h.lenses[0].stage, null);
    let finished = false;
    const loading = arena.load("same.glb", "same.glb", 7).then(() => { finished = true; });
    await flush(); await flush();
    assert.equal(finished, true, "the next hall must load independently of stale projection assets");
    old.resolve(); await loading; await flush();
    assert.equal(h.lenses[0].stage, null); assert.equal(arena.ready, true);
  } finally { old.resolve(); arena.free(); }

  const pending = deferred(), dying = projectedArenaHost(pending.promise);
  const disposed = new RustArena(dying, fakeCanvas(), { wide: .62, fpkey: 2.5 });
  disposed.setFrame({ ...frame, id: "hall1" }); disposed.resume();
  const late = disposed.load("same.glb", "same.glb", 7);
  disposed.dispose(); pending.resolve(); await late; await flush();
  assert.equal(disposed.ready, false); assert.equal(dying.renderers[0].loop, null);
  assert.equal(dying.stages[0].disposals, 1); assert.equal(dying.stages[0].ready, false);
  disposed.free(); assert.equal(dying.stages[0].disposals, 1);

  // Optional stage failure keeps the pre-existing painted hall usable.
  const unavailable = deferred(), fallback = projectedArenaHost(unavailable.promise);
  const painted = new RustArena(fallback, fakeCanvas(), { wide: .62, fpkey: 2.5 });
  try {
    painted.setFrame({ ...frame, id: "hall1" }); painted.resume();
    const loading = painted.load("same.glb", "same.glb", 7);
    unavailable.reject(new Error("optional projection asset unavailable")); await loading;
    assert.equal(painted.ready, true); assert.equal(fallback.lenses[0].stage, null);
    painted.move(6, 1, 0);
    for (let i = 0; i < 180; i++) fallback.renderers[0].loop(i * 1000 / 60);
    assert.equal(fallback.stages[0].travel, 0);
    assert.ok(fallback.events.some(event => event[0] === "walk"), "painted fallback retains its original camera-relative gait");
  } finally { painted.free(); }
}

async function testTeardown() {
  const h = makeHost(), renderer = new h.THREE.WebGLRenderer();
  const lens = new RustLens(h, renderer);
  lens.setBackdrop("late.jpg", undefined, "late-depth.png");
  lens.dispose(); const released = h.trace.filter(v => v[0] === "dispose").length;
  lens.dispose(); assert.equal(h.trace.filter(v => v[0] === "dispose").length, released);
  let lateDisposals = 0;
  for (const pending of h.loads) { const t = texture(pending.url); t.addEventListener("dispose", () => lateDisposals++); pending.resolve(t); }
  await flush(); assert.equal(lateDisposals, 2); lens.free();

  const a = arenaHost(), model = deferred(); a.loadModel = () => model.promise;
  const arena = new RustArena(a, fakeCanvas(), { wide: .62, fpkey: 2.5 });
  arena.setFrame(frame); arena.resume(); const loading = arena.load("late.glb", "late.glb", 7);
  arena.dispose(); arena.dispose(); arena.resume();
  assert.equal(a.renderers[0].loop, null); assert.equal(a.renderers[0].disposals, 1); assert.equal(arena.ready, false);
  model.resolve({ scene: new Three.Group(), animations: [] }); await loading;
  assert.equal(arena.ready, false); assert.equal(a.renderers[0].loop, null); arena.free();
  assert.equal(a.renderers[0].disposals, 1);

  const b = arenaHost(), active = new RustArena(b, fakeCanvas(), { wide: .62, fpkey: 2.5 });
  active.setFrame(frame); active.resume(); await active.load("same.glb", "same.glb", 7);
  assert.equal(typeof b.renderers[0].loop, "function"); active.free();
  assert.equal(b.renderers[0].loop, null); assert.equal(b.renderers[0].disposals, 1);
}

async function benchmarkArena(useRust) {
  const h = arenaHost();
  const Old = useRust ? null : (await original("arena.ts", { ...h, callRust, motionCompute, SkeletonUtils: { clone: h.clone }, QUALITY: quality })).Arena;
  const arena = useRust ? new RustArena(h, fakeCanvas(), { wide: .62, fpkey: 2.5 }) : new Old(fakeCanvas());
  arena.setFrame(frame); await arena.load("same.glb", "same.glb", 7); arena.setFirstPerson(true); arena.resume();
  const tick = h.renderers[0].loop;
  for (let i = 0; i < 200; i++) tick(i * 16);
  const start = performance.now();
  for (let i = 0; i < 3000; i++) tick(i * 16);
  const perFrame = (performance.now() - start) / 3000;
  arena.pause(); if (useRust) arena.free();
  return perFrame;
}

try {
  assert.deepEqual(await exerciseLens(true), await exerciseLens(false));
  await testTextureTiers();
  parity(await exerciseArena(true), await exerciseArena(false));
  await testTeardown();
  await testProjectedArenaMovement();
  await testProjectedArenaLifecycle();
  console.log("PASS Rust lens: identical shaders, quality targets, passes, uniforms, projection, async backdrop cancellation");
  console.log("PASS Rust texture tiers: shared maps, shrink, stale completion, failed tier fallback, atomic restoration");
  console.log("PASS Rust arena: lifecycle, quality, first-person framing, walking, firing, hit/fall, reflection, callback reentry");
  console.log("PASS Rust teardown: idempotent disposal, cancelled late loads, no renderer callback after Rust owner drops");
  console.log("PASS Rust projected combat: animated own step/retreat/rush, enemy-only movement, interrupted camera change, reset and blocked advance");
  console.log("PASS Rust projected lifecycle: independent hall switching, cancelled teardown and usable painted fallback");
  if (process.env.CD_RENDER_BENCH === "1") {
    console.log("CPU-only orchestration benchmark (ms/frame, stub renderer, no GPU):", { rust: await benchmarkArena(true), typescript: await benchmarkArena(false) });
  }
} finally {
  for (const [key, value] of Object.entries(saved)) { if (value === undefined) delete globalThis[key]; else globalThis[key] = value; }
}
