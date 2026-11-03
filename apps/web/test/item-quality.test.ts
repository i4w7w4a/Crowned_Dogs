import test from "node:test";
import assert from "node:assert/strict";
import * as THREE from "three";
import { RustItemViewer } from "../../../packages/runtime/src/index.ts";
class FakeElement extends EventTarget {
  dataset: Record<string, string> = {};
  style: Record<string, unknown> = {};
  classList = { add() {}, remove() {}, contains() { return false; } };
  clientWidth = 400;
  clientHeight = 400;
  hidden = false;
  textContent = "";
  private parts = new Map<string, FakeElement>();

  querySelector(selector: string) {
    if (!this.parts.has(selector)) {
      const part = new FakeElement();
      if (selector === "canvas") Object.assign(part, { width: 300, height: 150 });
      this.parts.set(selector, part);
    }
    return this.parts.get(selector)!;
  }

  setAttribute(_name: string, _value: string) {}
  replaceChildren() { this.parts.clear(); }
}

class FakeRenderer {
  constructor(..._args: unknown[]) {}
  pixelRatio = 0;
  capabilities = { getMaxAnisotropy: () => 16 };
  shadowMap = {};
  disposed = false;
  setClearColor() {}
  getContext() { return {}; }
  setPixelRatio(value: number) { this.pixelRatio = value; }
  setSize() {}
  render() {}
  dispose() { this.disposed = true; }
}

class FakeResizeObserver {
  constructor(_callback: () => void) {}
  observe() {}
  disconnect() {}
}

function quality(id: "high" | "low") {
  return id === "high"
    ? { id, env: true, ssaa: 1, maxDpr: 3, maxPixels: 3_800_000, textures: 2048, anisotropy: 8 }
    : { id, env: false, ssaa: 1, maxDpr: 1, maxPixels: 500_000, textures: 512, anisotropy: 1 };
}

function fixture(initial: "high" | "low") {
  let selected = initial;
  let qualityReads = 0;
  let pmremBuilds = 0;
  let materialTunes = 0;
  const tierCalls: Array<[number, number]> = [];
  const tierDisposals: number[] = [];
  const rendererInstances: FakeRenderer[] = [];
  class Renderer extends FakeRenderer {
    constructor(...args: unknown[]) { super(...args); rendererInstances.push(this); }
  }

  class Tiers {
    current = 0;
    disposed = false;
    constructor(_roots: unknown, _model: string) {}
    set(size: number, anisotropy: number) {
      this.current = size;
      tierCalls.push([size, anisotropy]);
      return Promise.resolve();
    }
    dispose() { this.disposed = true; tierDisposals.push(1); }
  }
  class PMREM {
    constructor(_renderer: unknown) { pmremBuilds++; }
    fromScene() { return { texture: { name: "pmrem" }, dispose() {} }; }
    dispose() {}
  }
  class Environment { dispose() {} }
  const scenes: THREE.Scene[] = [];
  class Scene extends THREE.Scene {
    constructor() { super(); scenes.push(this); }
  }

  const host = {
    THREE: { ...THREE, Scene, WebGLRenderer: Renderer, PMREMGenerator: PMREM },
    TextureTiers: Tiers,
    RoomEnvironment: Environment,
    QUALITY: { high: quality("high"), low: quality("low") },
    ITEM_MODELS: { varyag: "varyag" },
    MOD_SLOTS: [],
    base: "/",
    startQuality: () => { qualityReads++; return { id: selected }; },
    loadModel: async () => {
      const root = new THREE.Group();
      const material = new THREE.MeshStandardMaterial({ map: new THREE.Texture() });
      material.name = "M_Test";
      root.add(new THREE.Mesh(new THREE.BoxGeometry(), material));
      return { scene: root };
    },
    prepareVaryag() {},
    fitVaryag() {},
    tune: () => { materialTunes++; },
  };
  const element = new FakeElement();
  const viewer = new RustItemViewer(host, element, {});
  return {
    viewer, element, tierCalls, tierDisposals, scene: scenes[0],
    get renderer() { return rendererInstances[0]; },
    get qualityReads() { return qualityReads; },
    get pmremBuilds() { return pmremBuilds; },
    get materialTunes() { return materialTunes; },
    select(value: "high" | "low") { selected = value; },
    async settle() { await new Promise(resolve => setTimeout(resolve, 20)); },
  };
}

Object.assign(globalThis, { ResizeObserver: FakeResizeObserver, devicePixelRatio: 2 });

test("inventory refreshes an already cached model when quality changes", async () => {
  const f = fixture("high");
  await f.viewer.show("varyag", "Варяг", undefined);
  assert.equal(f.pmremBuilds, 1, "high builds one environment");
  assert.equal(f.scene!.environment!.name, "pmrem");
  assert.ok(f.tierCalls.some(([size]) => size === 2048));
  const readsBefore = f.qualityReads;
  const tunesBefore = f.materialTunes;

  f.select("low");
  f.viewer.setActive(true);
  await f.settle();

  assert.ok(f.qualityReads > readsBefore, "activation rereads the selected quality");
  assert.ok(f.tierCalls.some(([size, aniso]) => size === 512 && aniso === 1), "cached tiers receive low settings");
  assert.ok(f.materialTunes > tunesBefore, "materials are retuned for low");
  assert.equal(f.renderer!.pixelRatio, 1, "DPR follows low quality");
  assert.equal(f.scene!.environment, null, "low clears the expensive environment");
  f.viewer.dispose();
  await f.settle();
  assert.ok(f.tierDisposals.length > 0, "cached tiers are disposed with the viewer");
});

test("low inventory initialization never constructs PMREM", async () => {
  const f = fixture("low");
  await f.viewer.show("varyag", "Варяг", undefined);
  assert.equal(f.pmremBuilds, 0);
  f.viewer.dispose();
});
