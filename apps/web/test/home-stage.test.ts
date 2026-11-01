import assert from "node:assert/strict";
import { test, type TestContext } from "node:test";
import * as THREE from "three";
import { RustHomeStage } from "../../../packages/runtime/src/index.ts";

const aspect = 1882 / 3344;
const manifest = {
  version: 1, kind: "depth-photo", image: "../base/salt-base.webp", depth: "depth.png",
  aspect, zoom: 1.06, parallax: .002,
  targets: { battle: [.235, .527], fitting: [.275, .36] },
};
type Shot = {
  cover: number[]; pivot: number[]; zoom: number; parallax: number;
  triangles: number; meshes: number; orthographic: boolean; color: THREE.Texture; depth: THREE.Texture;
};
async function fixture(t: TestContext, options: {
  manifest?: Record<string, unknown>; ready?: boolean;
  load?: (url: string) => Promise<THREE.Texture>;
} = {}) {
  const old = Object.getOwnPropertyDescriptor(globalThis, "document");
  Object.defineProperty(globalThis, "document", { configurable: true, value: { baseURI: "http://base.test/" } });
  t.after(() => { if (old) Object.defineProperty(globalThis, "document", old); else Reflect.deleteProperty(globalThis, "document"); });
  let fetches = 0, texturesDisposed = 0, rendererDisposed = 0, geometriesDisposed = 0, materialsDisposed = 0;
  const loaded: string[] = [], shots: Shot[] = [];
  t.mock.method(globalThis, "fetch", async () => {
    fetches++;
    return new Response(JSON.stringify({ ...manifest, ...options.manifest }));
  });
  t.mock.method(THREE.TextureLoader.prototype, "loadAsync", async (url: string) => {
    loaded.push(url);
    const texture = options.load ? await options.load(url) : new THREE.Texture({ width: 512, height: 910 });
    texture.addEventListener("dispose", () => texturesDisposed++);
    return texture;
  });
  const tracked = new WeakSet<THREE.Mesh>();
  let renderer: Renderer;
  class Renderer {
    toneMapping = THREE.ACESFilmicToneMapping;
    outputColorSpace = THREE.SRGBColorSpace;
    sizes: number[][] = [];
    dpr = 0;
    constructor() { renderer = this; }
    setPixelRatio(value: number) { this.dpr = value; }
    setSize(w: number, h: number) { this.sizes.push([w, h]); }
    render(scene: THREE.Scene, camera: THREE.Camera) {
      const meshes: THREE.Mesh[] = [];
      scene.traverse(object => { if (object instanceof THREE.Mesh) meshes.push(object); });
      for (const mesh of meshes) if (!tracked.has(mesh)) {
        tracked.add(mesh);
        mesh.geometry.addEventListener("dispose", () => geometriesDisposed++);
        (mesh.material as THREE.Material).addEventListener("dispose", () => materialsDisposed++);
      }
      const mesh = meshes[0]!;
      const uniforms = (mesh.material as THREE.ShaderMaterial).uniforms;
      shots.push({
        cover: uniforms.uCover!.value.toArray(), pivot: uniforms.uPivot!.value.toArray(),
        zoom: uniforms.uZoom!.value, parallax: uniforms.uParallax!.value,
        triangles: mesh.geometry.index!.count / 3, meshes: meshes.length,
        orthographic: camera instanceof THREE.OrthographicCamera,
        color: uniforms.uColor!.value, depth: uniforms.uDepth!.value,
      });
    }
    dispose() { rendererDisposed++; }
  }
  const attrs = new Map<string, string>();
  const canvas = { clientWidth: 390, clientHeight: 844, setAttribute: (key: string, value: string) => attrs.set(key, value) };
  const stage = new RustHomeStage({ THREE: { ...THREE, WebGLRenderer: Renderer } }, canvas, "/img/base-depth/manifest.json");
  t.after(() => { stage.dispose(); stage.free(); });
  if (options.ready !== false) await stage.ready;
  return { stage, shots, attrs, canvas, loaded, renderer: renderer!, counts: () => ({ fetches, texturesDisposed, rendererDisposed, geometriesDisposed, materialsDisposed }) };
}
function close(actual: number[], expected: number[], message: string) {
  assert.equal(actual.length, expected.length);
  actual.forEach((value, index) => assert.ok(Math.abs(value - expected[index]!) < 1e-10, `${message}: ${value} != ${expected[index]}`));
}
function sourceUv(shot: Shot, u: number, v: number): number[] {
  return [u, v].map((value, index) => {
    const base = (value - .5) * shot.cover[index]! + .5;
    return shot.pivot[index]! + (base - shot.pivot[index]!) / shot.zoom;
  });
}

test("home remains one intact two-triangle photograph with exact cover framing at rest", async (t) => {
  const { stage, shots, canvas, renderer, loaded } = await fixture(t);
  for (const [width, height] of [[390, 844], [480, 688], [941, 1672]]) {
    canvas.clientWidth = width!; canvas.clientHeight = height!;
    stage.draw("base", 1);
    const shot = shots.at(-1)!;
    close(shot.cover, [width! / Math.max(width!, height! * aspect), height! / Math.max(height!, width! / aspect)], "object-fit cover without overscan");
    assert.equal(shot.zoom, 1); assert.equal(shot.parallax, 0);
    assert.equal(shot.meshes, 1); assert.equal(shot.triangles, 2); assert.equal(shot.orthographic, true);
    close(sourceUv(shot, .5, .5), [.5, .5], "unchanged photograph centre");
    assert.equal(shot.color.colorSpace, THREE.NoColorSpace);
    assert.equal(shot.depth.colorSpace, THREE.NoColorSpace);
    assert.equal(shot.depth.minFilter, THREE.LinearFilter);
    assert.equal(shot.color.wrapS, THREE.ClampToEdgeWrapping);
    assert.equal(shot.color.wrapT, THREE.ClampToEdgeWrapping);
  }
  assert.equal(renderer.outputColorSpace, THREE.LinearSRGBColorSpace);
  assert.equal(renderer.toneMapping, THREE.NoToneMapping);
  assert.deepEqual(loaded, ["http://base.test/img/base/salt-base.webp", "http://base.test/img/base-depth/depth.png"]);
});

test("crown and tunnel approach their own photograph coordinates by exactly six percent", async (t) => {
  const { stage, shots, attrs, counts } = await fixture(t);
  for (const route of ["battle", "fitting"] as const) {
    stage.draw(route, 0);
    const start = shots.at(-1)!;
    assert.equal(start.zoom, 1); assert.equal(start.parallax, 0);
    close(start.pivot, [manifest.targets[route][0]!, 1 - manifest.targets[route][1]!], "top-origin target converted once");
    stage.draw(route, .5);
    const middle = shots.at(-1)!;
    assert.ok(Math.abs(middle.zoom - 1.03) < 1e-12);
    assert.ok(Math.abs(middle.parallax - .001) < 1e-12);
    stage.draw(route, 1);
    const end = shots.at(-1)!;
    assert.equal(end.zoom, 1.06); assert.equal(end.parallax, .002);
    const centre = sourceUv(end, .5, .5);
    for (let axis = 0; axis < 2; axis++) {
      assert.ok(Math.abs(centre[axis]! - end.pivot[axis]!) < Math.abs(.5 - end.pivot[axis]!), "sampling advances toward the selected point");
    }
    for (const u of [0, .25, .5, .75, 1]) for (const v of [0, .25, .5, .75, 1]) {
      const at = sourceUv(end, u, v);
      assert.ok(at.every(value => value >= 0 && value <= 1), "the entire view stays inside original pixels");
    }
  }
  assert.equal(attrs.get("data-home-zoom"), "1.060000");
  assert.equal(attrs.get("data-depth-photo"), "true");
  assert.equal(counts().fetches, 1);
});

test("reset, unsupported targets and non-finite input cannot leave drift or NaN behind", async (t) => {
  const { stage, shots, canvas } = await fixture(t);
  stage.draw("battle", 1);
  for (const [route, progress] of [["base", 1], ["unknown", 1], ["fitting", Number.NaN], ["battle", Number.POSITIVE_INFINITY], ["battle", -1]] as const) {
    stage.draw(route, progress);
    const shot = shots.at(-1)!;
    assert.equal(shot.zoom, 1); assert.equal(shot.parallax, 0);
    assert.ok([...shot.cover, ...shot.pivot, shot.zoom, shot.parallax].every(Number.isFinite));
  }
  stage.draw("fitting", 9);
  assert.equal(shots.at(-1)!.zoom, 1.06);
  const before = shots.length;
  stage.draw("fitting", 9);
  assert.equal(shots.length, before, "identical idle frames are cached");
  canvas.clientWidth = 480; canvas.clientHeight = 688;
  stage.draw("fitting", 9);
  assert.equal(shots.length, before + 1, "resize updates framing even at the same progress");
  const resized = shots.length;
  canvas.clientWidth = Number.NaN;
  stage.draw("battle", 1);
  assert.equal(shots.length, resized, "invalid size does not reach WebGL");
});

test("manifest cannot increase the approach beyond seven percent or depth motion beyond .002 UV", async (t) => {
  const { stage, shots } = await fixture(t, { manifest: { zoom: 10, parallax: 10 } });
  for (const progress of [0, .1, .25, .5, .75, 1]) {
    stage.draw("battle", progress);
    const shot = shots.at(-1)!;
    assert.ok(shot.zoom >= 1 && shot.zoom <= 1.07);
    assert.ok(shot.parallax >= 0 && shot.parallax <= .002);
  }
  assert.equal(shots.at(-1)!.zoom, 1.07);
});

test("home releases its own textures and quad once, and never draws after disposal", async (t) => {
  const { stage, shots, counts } = await fixture(t);
  stage.draw("base", 0);
  stage.dispose(); stage.dispose();
  stage.draw("battle", 1);
  assert.equal(shots.length, 1);
  assert.deepEqual(counts(), { fetches: 1, texturesDisposed: 2, rendererDisposed: 1, geometriesDisposed: 1, materialsDisposed: 1 });
});

test("disposing a pending load releases both late textures and never activates the canvas", async (t) => {
  const finish: Array<(texture: THREE.Texture) => void> = [];
  let started!: () => void;
  const bothStarted = new Promise<void>(resolve => { started = resolve; });
  const { stage, shots, counts } = await fixture(t, { ready: false, load: () => new Promise(resolve => {
    finish.push(resolve); if (finish.length === 2) started();
  }) });
  const rejected = assert.rejects(stage.ready, /cancelled/);
  await bothStarted;
  stage.dispose();
  for (const resolve of finish) resolve(new THREE.Texture());
  await rejected;
  stage.draw("battle", 1);
  assert.equal(shots.length, 0);
  assert.equal(counts().texturesDisposed, 2);
  assert.equal(counts().rendererDisposed, 1);
});

test("a failed depth image waits for and releases its late colour-image sibling", async (t) => {
  let finish!: (texture: THREE.Texture) => void;
  let started!: () => void;
  const imageStarted = new Promise<void>(resolve => { started = resolve; });
  const { stage, shots, counts } = await fixture(t, { ready: false, load: async url => {
    if (url.endsWith("depth.png")) throw new Error("depth unavailable");
    return new Promise(resolve => { finish = resolve; started(); });
  } });
  const rejected = assert.rejects(stage.ready, /depth unavailable/);
  await imageStarted;
  finish(new THREE.Texture());
  await rejected;
  stage.draw("battle", 1);
  assert.equal(shots.length, 0);
  assert.equal(counts().texturesDisposed, 1);
});

test("invalid depth-photo manifest fails before starting either image", async (t) => {
  const { stage, loaded, shots } = await fixture(t, { ready: false, manifest: { aspect: 0 } });
  await assert.rejects(stage.ready, /Invalid base depth-photo manifest/);
  stage.draw("base", 0);
  assert.deepEqual(loaded, []); assert.deepEqual(shots, []);
});
