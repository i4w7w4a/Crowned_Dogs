import assert from "node:assert/strict";
import { test, type TestContext } from "node:test";
import * as THREE from "three";
import { ProjectedStage } from "../src/arena/projected-stage.ts";

async function fixture(t: TestContext, autoLoad = true) {
  const documentDescriptor = Object.getOwnPropertyDescriptor(globalThis, "document");
  Object.defineProperty(globalThis, "document", { configurable: true, value: { baseURI: "http://stage.test/" } });
  t.after(() => {
    if (documentDescriptor) Object.defineProperty(globalThis, "document", documentDescriptor);
    else Reflect.deleteProperty(globalThis, "document");
  });
  const triangle = { positions: [0, 0, -2, 1, 0, -2, 0, 0, -3], indices: [0, 1, 2] };
  t.mock.method(globalThis, "fetch", async () => new Response(JSON.stringify({
    version: 1,
    calibration: { fov: 55, aspect: 941 / 1672, camHeight: 2, horizon: .3724444444 },
    textures: { clean: "clean.png", equipment: "equipment.jpg" },
    meshes: [
      { ...triangle, name: "wet-floor", material: "floor", occludesFigures: false },
      { ...triangle, name: "near-machine", material: "equipment" },
    ],
  })));
  t.mock.method(THREE.TextureLoader.prototype, "loadAsync", async () => new THREE.Texture());
  const draws: string[][] = [];
  let target: THREE.WebGLRenderTarget | null = null;
  const renderer = {
    capabilities: { getMaxAnisotropy: () => 4 },
    shadowMap: { autoUpdate: true }, autoClear: true,
    toneMapping: THREE.NoToneMapping as THREE.ToneMapping, outputColorSpace: THREE.LinearSRGBColorSpace as string,
    getRenderTarget: () => target,
    setRenderTarget: (next: THREE.WebGLRenderTarget | null) => { target = next; },
    getClearColor: (out: THREE.Color) => out.set(0x222222), getClearAlpha: () => .5,
    setClearColor: () => {}, clear: () => {},
    render: (scene: THREE.Scene, camera: THREE.Camera) => {
      const names: string[] = [];
      scene.traverseVisible((object) => { if (object instanceof THREE.Mesh && object.layers.test(camera.layers)) names.push(object.name); });
      draws.push(names);
    },
  };
  const stage = new ProjectedStage(renderer as unknown as THREE.WebGLRenderer);
  t.after(() => stage.dispose());
  if (autoLoad) await stage.load("manifest.json");
  return { stage, renderer, draws, camera: new THREE.PerspectiveCamera() };
}

test("wet floor remains in photo/depth passes but does not hide the reflected fighter", async (t) => {
  const { stage, renderer, draws, camera } = await fixture(t);
  camera.layers.enable(5);
  const initialLayers = camera.layers.mask;
  stage.occlude(camera);
  assert.deepEqual(draws.pop(), ["near-machine"]);
  assert.equal(camera.layers.mask, initialLayers);
  assert.equal(renderer.autoClear, true);
  assert.equal(renderer.shadowMap.autoUpdate, true);
  stage.render(camera, 64, 128);
  assert.deepEqual(draws, [["wet-floor", "near-machine"], ["wet-floor", "near-machine"]]);
});

test("an interrupted occlusion draw restores camera and renderer state", async (t) => {
  const { stage, renderer, camera } = await fixture(t);
  camera.layers.enable(5);
  const initialLayers = camera.layers.mask;
  t.mock.method(renderer, "render", () => { throw new Error("context interrupted"); });
  assert.throws(() => stage.occlude(camera), /context interrupted/);
  assert.equal(camera.layers.mask, initialLayers);
  assert.equal(renderer.autoClear, true);
  assert.equal(renderer.shadowMap.autoUpdate, true);
});

test("disposing during texture loading releases late images and never reactivates the stage", async (t) => {
  const { stage } = await fixture(t, false);
  const finished: Array<(texture: THREE.Texture) => void> = [];
  let started!: () => void;
  const allStarted = new Promise<void>((resolve) => { started = resolve; });
  t.mock.method(THREE.TextureLoader.prototype, "loadAsync", () => new Promise<THREE.Texture>((resolve) => {
    finished.push(resolve);
    if (finished.length === 2) started();
  }));
  const pending = stage.load("manifest.json");
  await allStarted;
  stage.dispose();
  let disposed = 0;
  for (const resolve of finished) {
    const texture = new THREE.Texture();
    texture.addEventListener("dispose", () => { disposed++; });
    resolve(texture);
  }
  await assert.rejects(pending, /cancelled/);
  assert.equal(disposed, 2);
  assert.equal(stage.ready, false);
  await assert.rejects(stage.load("manifest.json"), /disposed/);
});

test("a failed colour pass restores its caller render target and colour settings", async (t) => {
  const { stage, renderer, camera } = await fixture(t);
  const callerTarget = new THREE.WebGLRenderTarget(2, 2);
  t.after(() => callerTarget.dispose());
  renderer.setRenderTarget(callerTarget);
  renderer.toneMapping = THREE.ACESFilmicToneMapping;
  renderer.outputColorSpace = THREE.SRGBColorSpace;
  t.mock.method(renderer, "render", () => { throw new Error("colour pass interrupted"); });
  assert.throws(() => stage.render(camera, 64, 128), /colour pass interrupted/);
  assert.equal(renderer.getRenderTarget(), callerTarget);
  assert.equal(renderer.toneMapping, THREE.ACESFilmicToneMapping);
  assert.equal(renderer.outputColorSpace, THREE.SRGBColorSpace);
  assert.equal(renderer.shadowMap.autoUpdate, true);
});
