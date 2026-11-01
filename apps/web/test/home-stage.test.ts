import assert from "node:assert/strict";
import { test, type TestContext } from "node:test";
import * as THREE from "three";
import { RustHomeStage } from "../../../packages/runtime/src/index.ts";

const calibration = { fov: 63, aspect: .6, camHeight: 1.75, horizon: .4 };
const cameraPaths = {
  battle: [
    { position: [0, 1.75, 0], target: [0, 1.75, -8] },
    { position: [.4, 1.6, -1], target: [.1, 1.2, -5] },
    { position: [-.6, 1.4, -4], target: [-.2, 1, -6] },
  ],
  fitting: [
    { position: [0, 1.75, 0], target: [0, 1.75, -8] },
    { position: [.7, 1.8, -2], target: [-.5, 1.8, -10] },
    { position: [-1.1, 1.8, -7], target: [-1.2, 1.8, -18] },
  ],
};

async function fixture(t: TestContext, paths: unknown = cameraPaths) {
  const old = Object.getOwnPropertyDescriptor(globalThis, "document");
  Object.defineProperty(globalThis, "document", { configurable: true, value: { baseURI: "http://base.test/" } });
  t.after(() => {
    if (old) Object.defineProperty(globalThis, "document", old);
    else Reflect.deleteProperty(globalThis, "document");
  });
  let fetches = 0, texturesDisposed = 0, rendererDisposed = 0;
  t.mock.method(globalThis, "fetch", async () => {
    fetches++;
    return new Response(JSON.stringify({
      version: 1, calibration, cameraPaths: paths,
      textures: { clean: "clean.png", equipment: "original.webp" },
      meshes: [{ name: "crown", material: "equipment", positions: [-1, 0, -4, 1, 0, -4, 0, 2, -4], indices: [0, 1, 2] }],
    }));
  });
  t.mock.method(THREE.TextureLoader.prototype, "loadAsync", async () => {
    const tex = new THREE.Texture();
    tex.addEventListener("dispose", () => texturesDisposed++);
    return tex;
  });
  const shots: Array<{ position: number[]; direction: number[]; projection: number[]; meshes: string[] }> = [];
  class Renderer {
    capabilities = { getMaxAnisotropy: () => 4 };
    target: unknown = null;
    setClearColor() {} setPixelRatio() {} setSize() {}
    setRenderTarget(target: unknown) { this.target = target; }
    getRenderTarget() { return this.target; }
    render(scene: THREE.Scene, camera: THREE.PerspectiveCamera) {
      const meshes: string[] = [];
      scene.traverse(o => { if (o instanceof THREE.Mesh) meshes.push(o.name); });
      shots.push({
        position: camera.position.toArray(), direction: camera.getWorldDirection(new THREE.Vector3()).toArray(),
        projection: camera.projectionMatrix.toArray(), meshes,
      });
    }
    dispose() { rendererDisposed++; }
  }
  const attrs = new Map<string, string>();
  const canvas = { clientWidth: 600, clientHeight: 1000, setAttribute: (k: string, v: string) => attrs.set(k, v) };
  const stage = new RustHomeStage({ THREE: { ...THREE, WebGLRenderer: Renderer } }, canvas, "manifest.json");
  await stage.ready;
  t.after(() => { stage.dispose(); stage.free(); });
  return { stage, shots, attrs, canvas, counts: () => ({ fetches, texturesDisposed, rendererDisposed }) };
}

function close(actual: number[], expected: number[], message: string) {
  assert.equal(actual.length, expected.length);
  actual.forEach((value, i) => assert.ok(Math.abs(value - expected[i]!) < 1e-10, `${message}: ${i}: ${value} != ${expected[i]}`));
}

test("base reads the Blender camera and eased samples from the same manifest as its geometry", async (t) => {
  const { stage, shots, attrs, counts } = await fixture(t);
  stage.draw("base", 0);
  const first = shots.at(-1)!;
  close(first.position, cameraPaths.battle[0]!.position, "exported origin");
  assert.deepEqual(first.meshes, ["crown"]);
  // Deliberately use a different lens, horizon and height from the old base.
  const focal = 1 / Math.tan(THREE.MathUtils.degToRad(calibration.fov / 2));
  assert.ok(Math.abs(first.projection[5]! - focal) < 1e-10);
  assert.ok(Math.abs(first.projection[0]! - focal / calibration.aspect) < 1e-10);
  assert.ok(Math.abs(first.projection[9]! - (2 * calibration.horizon - 1)) < 1e-10);

  stage.draw("battle", .25);
  close(shots.at(-1)!.position, [.2, 1.675, -.5], "interpolation of already eased samples");
  stage.draw("battle", .5);
  close(shots.at(-1)!.position, cameraPaths.battle[1]!.position, "middle authored sample");
  const direction = new THREE.Vector3().fromArray(cameraPaths.battle[1]!.target)
    .sub(new THREE.Vector3().fromArray(cameraPaths.battle[1]!.position)).normalize().toArray();
  close(shots.at(-1)!.direction, direction, "authored aim point");
  stage.draw("battle", 1);
  close(shots.at(-1)!.position, cameraPaths.battle[2]!.position, "crown destination");
  stage.draw("fitting", 1);
  close(shots.at(-1)!.position, cameraPaths.fitting[2]!.position, "tunnel destination");
  for (const shot of shots) assert.deepEqual(shot.projection, first.projection, "dolly keeps the exported lens fixed");
  assert.equal(attrs.get("data-home-target"), "fitting");
  assert.equal(counts().fetches, 1, "geometry and camera must not fetch separate manifest versions");

  const count = shots.length;
  stage.draw("fitting", 1);
  assert.equal(shots.length, count, "idle frame is cached");
  stage.dispose(); stage.dispose();
  assert.deepEqual(counts(), { fetches: 1, texturesDisposed: 2, rendererDisposed: 1 });
});

test("missing or incomplete routes stay at the calibrated origin and resizing keeps their framing", async (t) => {
  const { stage, shots, canvas } = await fixture(t, {
    battle: [cameraPaths.battle[0], { position: [0, 1], target: [0, 1, -8] }, cameraPaths.battle[2]],
  });
  for (const target of ["base", "battle", "fitting"]) {
    stage.draw(target, 1);
    close(shots.at(-1)!.position, [0, calibration.camHeight, 0], "safe static fallback");
    close(shots.at(-1)!.direction, [0, 0, -1], "fixed projector direction");
  }
  const fullFrame = shots.at(-1)!.projection;
  canvas.clientWidth = 390; canvas.clientHeight = 844;
  stage.draw("battle", 1);
  const phone = shots.at(-1)!.projection;
  assert.ok(phone[0]! > fullFrame[0]!, "portrait viewport crops the sides");
  assert.equal(phone[5], fullFrame[5], "portrait crop retains the calibrated vertical focal length");
  assert.equal(phone[9], fullFrame[9], "portrait crop retains the calibrated horizon");
  stage.draw("battle", Number.NaN);
  assert.ok(shots.at(-1)!.projection.every(Number.isFinite));
});
