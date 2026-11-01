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
  const images: Array<{ url: string; texture: THREE.Texture; disposed: boolean }> = [];
  t.mock.method(THREE.TextureLoader.prototype, "loadAsync", async (url: string) => {
    const texture = new THREE.Texture();
    texture.userData.url = url;
    const image = { url, texture, disposed: false };
    texture.addEventListener("dispose", () => { image.disposed = true; });
    images.push(image);
    return texture;
  });
  const draws: string[][] = [];
  const scenes: THREE.Scene[] = [];
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
      scenes.push(scene);
      const names: string[] = [];
      scene.traverseVisible((object) => { if (object instanceof THREE.Mesh && object.layers.test(camera.layers)) names.push(object.name); });
      draws.push(names);
    },
  };
  const stage = new ProjectedStage(renderer as unknown as THREE.WebGLRenderer);
  t.after(() => stage.dispose());
  if (autoLoad) await stage.load("manifest.json");
  return { stage, renderer, draws, images, scenes, camera: new THREE.PerspectiveCamera() };
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

test("a time variant replaces its images while preserving geometry, occlusion and extension lighting", async (t) => {
  const { stage, images, scenes, draws, camera } = await fixture(t);
  const morning = {
    textures: { clean: "img/morning/clean.webp", equipment: "img/morning/hall.webp" },
    extensionTint: 0xa3b4cc, extensionExposure: .6,
  };
  await stage.load("manifest.json", morning);
  assert.equal(images.length, 4);
  assert.ok(images.slice(0, 2).every(image => image.disposed));
  assert.deepEqual(images.slice(2).map(image => image.url), ["http://stage.test/img/morning/clean.webp", "http://stage.test/img/morning/hall.webp"]);
  stage.render(camera, 64, 128);
  stage.occlude(camera);
  assert.deepEqual(draws, [["wet-floor", "near-machine"], ["wet-floor", "near-machine"], ["near-machine"]]);
  const mesh = scenes[0]!.getObjectByName("near-machine") as THREE.Mesh<THREE.BufferGeometry, THREE.ShaderMaterial>;
  assert.equal(mesh.material.uniforms.photo!.value.userData.url, "http://stage.test/img/morning/hall.webp");
  assert.equal(mesh.material.uniforms.extensionTint!.value.getHex(), 0xa3b4cc);
  assert.equal(mesh.material.uniforms.extensionExposure!.value, .6);
  await stage.load("manifest.json", morning);
  assert.equal(images.length, 4, "the same ready time must reuse its GPU assets");
});

test("a late time variant cannot overwrite the newly selected time or leak its images", async (t) => {
  const { stage, camera, scenes } = await fixture(t, false);
  const pending: Array<{ url: string; resolve: (texture: THREE.Texture) => void }> = [];
  t.mock.method(THREE.TextureLoader.prototype, "loadAsync", (url: string) => new Promise<THREE.Texture>((resolve) => pending.push({ url, resolve })));
  const waitFor = async (count: number) => {
    for (let i = 0; pending.length < count && i < 50; i++) await new Promise(resolve => setImmediate(resolve));
    assert.equal(pending.length, count);
  };
  const old = stage.load("manifest.json", { textures: { clean: "morning-clean.webp", equipment: "morning.webp" } });
  const rejected = assert.rejects(old, /cancelled/);
  await waitFor(2);
  const current = stage.load("manifest.json", { textures: { clean: "night-clean.webp", equipment: "night.webp" } });
  await waitFor(4);
  let disposed = 0;
  const finish = (request: typeof pending[number]) => {
    const texture = new THREE.Texture(); texture.userData.url = request.url;
    texture.addEventListener("dispose", () => { disposed++; }); request.resolve(texture);
  };
  pending.slice(2).forEach(finish); await current;
  pending.slice(0, 2).forEach(finish); await rejected;
  assert.equal(disposed, 2);
  assert.equal(stage.ready, true);
  stage.render(camera, 64, 128);
  const mesh = scenes[0]!.getObjectByName("near-machine") as THREE.Mesh<THREE.BufferGeometry, THREE.ShaderMaterial>;
  assert.equal(mesh.material.uniforms.photo!.value.userData.url, "http://stage.test/night.webp");
});
