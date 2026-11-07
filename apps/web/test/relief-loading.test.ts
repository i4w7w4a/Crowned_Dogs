import assert from "node:assert/strict";
import { test, type TestContext } from "node:test";
import * as THREE from "three";
import { sceneLoadModel } from "../../../packages/runtime/src/index.ts";

const base = "https://relief.test/";
const modelUrl = `${base}models/varyag.json`;
const manifestUrl = `${base}tex/relief/manifest.json`;
const reliefUrl = `${base}tex/relief/varyag-M_Varyag.relief.png`;

function fixture(t: TestContext, quality: "high" | "low", deferred: boolean) {
  for (const key of ["location", "document", "localStorage"]) {
    const previous = Object.getOwnPropertyDescriptor(globalThis, key);
    t.after(() => {
      if (previous) Object.defineProperty(globalThis, key, previous);
      else Reflect.deleteProperty(globalThis, key);
    });
  }
  Object.defineProperty(globalThis, "location", { configurable: true, value: { search: `?q=${quality}` } });
  Object.defineProperty(globalThis, "document", { configurable: true, value: {
    createElement: () => ({ getContext: () => null }),
  } });
  Object.defineProperty(globalThis, "localStorage", { configurable: true, value: {
    getItem: () => null, setItem: () => {},
  } });

  const geometry = new THREE.BufferGeometry();
  geometry.setAttribute("uv", new THREE.Float32BufferAttribute([0, 0, 1, 0, 1, 1], 2));
  const normalMap = new THREE.Texture();
  normalMap.flipY = false;
  const material = new THREE.MeshStandardMaterial({ normalMap });
  material.name = "M_Varyag";
  const root = new THREE.Group();
  root.add(new THREE.Mesh(geometry, material));

  let resolveTexture!: (texture: THREE.Texture) => void;
  const texturePromise = new Promise<THREE.Texture>((resolve) => { resolveTexture = resolve; });
  t.after(() => resolveTexture(new THREE.Texture()));
  let manifestCalls = 0, reliefCalls = 0;
  t.mock.method(globalThis, "fetch", async (input: unknown) => {
    const url = String(input);
    if (url === modelUrl) return new Response(JSON.stringify({ data: Buffer.from(url).toString("base64") }));
    if (url === manifestUrl) {
      manifestCalls++;
      return new Response(JSON.stringify({ version: 1, models: { varyag: {
        unitsPerMeter: 1,
        materials: { M_Varyag: { eligible: true, relief: "tex/relief/varyag-M_Varyag.relief.png" } },
      } } }));
    }
    throw new Error(`unexpected fetch: ${url}`);
  });
  class TextureLoader {
    loadAsync(url: string): Promise<THREE.Texture> {
      assert.equal(url, reliefUrl);
      reliefCalls++;
      return deferred ? texturePromise : Promise.resolve(new THREE.Texture());
    }
  }
  class GLTFLoader {
    setMeshoptDecoder() {}
    async parseAsync() { return { scene: root, animations: [] }; }
  }
  const host = {
    THREE: { ...THREE, TextureLoader }, GLTFLoader, MeshoptDecoder: {}, baseUrl: base,
    hasFile: () => false, ktx2: () => null,
    callRust: () => ({ depthUnits: 0.001, midpoint: 0.5 }),
  };
  return { root, material, host, resolveTexture, calls: () => ({ manifestCalls, reliefCalls }) };
}

const settle = async () => { for (let i = 0; i < 20; i++) await new Promise(resolve => setImmediate(resolve)); };

test("A model returns before its relief texture and applies it later", async t => {
  const f = fixture(t, "high", true);
  let finished = false;
  const loading = sceneLoadModel(f.host, modelUrl);
  loading.then(() => { finished = true; });
  await settle();
  assert.equal(finished, true, "the model must not wait for the relief texture");

  const version = f.material.version;
  f.resolveTexture(new THREE.Texture());
  await loading;
  await settle();
  assert.ok(f.material.bumpMap, "the late relief becomes the material bump map");
  assert.ok(f.material.version > version, "the late texture refreshes the material");
});

test("Low quality never downloads the relief manifest or textures", async t => {
  const f = fixture(t, "low", false);
  await sceneLoadModel(f.host, modelUrl);
  assert.deepEqual(f.calls(), { manifestCalls: 0, reliefCalls: 0 });
});
