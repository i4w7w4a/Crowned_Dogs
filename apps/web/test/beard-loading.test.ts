import assert from "node:assert/strict";
import { test, type TestContext } from "node:test";
import * as THREE from "three";
import { sceneLoadModel } from "../../../packages/runtime/src/index.ts";

const base = "https://beards.test/game/";
const styles = ["boxed", "goatee", "full", "tapered"];
type Counts = { geometry: number; material: number; texture: number; image: number };
function resources() {
  const disposed: Counts = { geometry: 0, material: 0, texture: 0, image: 0 };
  const geometry = new THREE.BufferGeometry();
  geometry.addEventListener("dispose", () => disposed.geometry++);
  const image = { close() { disposed.image++; } };
  const texture = new THREE.Texture(image);
  texture.addEventListener("dispose", () => disposed.texture++);
  const material = new THREE.MeshStandardMaterial({ map: texture, normalMap: texture });
  material.addEventListener("dispose", () => disposed.material++);
  return { disposed, geometry, material, texture };
}
function figure(animated = true) {
  const root = new THREE.Group();
  const anchor = new THREE.Bone(); anchor.name = "head";
  anchor.position.set(.01, 1.7, .03); anchor.rotation.y = .3;
  if (animated) root.add(anchor);
  const old = [resources(), resources()];
  for (const [i, id] of ["m2", "m4"].entries()) {
    old[i]!.material.name = `M_Head_${id}`;
    const node = new THREE.Mesh(old[i]!.geometry, old[i]!.material);
    node.name = `Head_${id}`; root.add(node);
  }
  return { root, anchor, old };
}
function beardPack() {
  const root = new THREE.Group();
  const owned = resources();
  owned.material.name = "M_Beard_Cards";
  for (let face = 1; face <= 4; face++) for (const style of styles) {
    const node = new THREE.Mesh(owned.geometry, owned.material);
    node.name = `Beard_m${face}_${style}_cards`;
    node.position.set(.02, -.05, .04);
    root.add(node);
  }
  return { root, ...owned };
}
function skinPack() {
  const root = new THREE.Group();
  const a = resources(), b = resources();
  // glTF may share one carrier geometry; dispose it once, retain adopted materials.
  for (const [id, material] of [["m2", a.material], ["m4", b.material]] as const) {
    const node = new THREE.Mesh(a.geometry, material);
    node.name = `CleanHead_${id}`; root.add(node);
  }
  return { root, a, b };
}
function network(t: TestContext, scenes: Map<string, THREE.Group>, failures = new Set<string>()) {
  const requested: string[] = [];
  t.mock.method(globalThis, "fetch", async (input: unknown) => {
    const url = String(input); requested.push(url);
    if (!scenes.has(url)) return new Response("missing", { status: 404 });
    return new Response(JSON.stringify({ data: Buffer.from(url).toString("base64") }), { status: 200 });
  });
  class Loader {
    setMeshoptDecoder() { return this; }
    async parseAsync(bytes: ArrayBuffer) {
      const url = Buffer.from(bytes).toString();
      if (failures.has(url)) throw new Error("invalid glTF");
      return { scene: scenes.get(url), animations: [] };
    }
  }
  return { requested, host: { GLTFLoader: Loader, MeshoptDecoder: {}, baseUrl: base, THREE } };
}

test("animated beard pack follows the existing head bone and adopts clean skin atomically", async t => {
  const main = figure(), beards = beardPack(), skin = skinPack();
  const url = `${base}models/persona-male.json`;
  const { host, requested } = network(t, new Map([
    [url, main.root], [`${base}models/beards-male.json`, beards.root], [`${base}models/beards-skin.json`, skin.root],
  ]));
  const result = await sceneLoadModel(host, url);
  assert.equal(result.scene, main.root);
  assert.equal(main.root.userData.beardsReady, true);
  assert.equal(beards.root.parent, main.anchor);
  assert.ok(beards.root.children.every(n => !n.visible && !n.frustumCulled));
  main.root.updateMatrixWorld(true);
  const before = beards.root.children[0]!.getWorldPosition(new THREE.Vector3());
  main.anchor.rotation.y += .5; main.root.updateMatrixWorld(true);
  assert.ok(before.distanceTo(beards.root.children[0]!.getWorldPosition(new THREE.Vector3())) > .001);
  assert.equal((main.root.getObjectByName("Head_m2") as THREE.Mesh).material, skin.a.material);
  assert.equal(skin.a.material.name, "M_Head_m2");
  assert.equal(skin.a.material.userData.cdFixedTextures, true);
  assert.equal(skin.a.disposed.geometry, 1);
  assert.equal(skin.a.disposed.material, 0);
  assert.equal(skin.a.disposed.texture, 0);
  assert.equal(beards.disposed.geometry, 0);
  assert.equal(main.old[0]!.disposed.material, 1);
  assert.equal(main.old[0]!.disposed.texture, 1);
  assert.deepEqual(requested, [url, `${base}models/beards-male.json`, `${base}models/beards-skin.json`]);
});

test("authored beard pack keeps its exported transform under the static figure", async t => {
  const main = figure(false), beards = beardPack(), skin = skinPack();
  beards.root.position.set(.2, .3, -.1);
  const url = `${base}preview/persona-authored.json`;
  const { host } = network(t, new Map([
    [url, main.root], [`${base}models/beards-authored.json`, beards.root], [`${base}models/beards-skin.json`, skin.root],
  ]));
  await sceneLoadModel(host, url);
  assert.equal(main.root.userData.beardsReady, true);
  assert.equal(beards.root.parent, main.root);
  assert.deepEqual(beards.root.position.toArray(), [.2, .3, -.1]);
});

test("missing or corrupt optional beard pack preserves the original model", async t => {
  for (const corrupt of [false, true]) {
    const main = figure(), pack = beardPack(), url = `${base}models/persona-male.json`;
    const scenes = new Map([[url, main.root]]);
    if (corrupt) scenes.set(`${base}models/beards-male.json`, pack.root);
    const { host } = network(t, scenes, new Set(corrupt ? [`${base}models/beards-male.json`] : []));
    const result = await sceneLoadModel(host, url);
    assert.equal(result.scene, main.root);
    assert.equal(main.root.userData.beardsReady, false);
    assert.equal((main.root.getObjectByName("Head_m2") as THREE.Mesh).material, main.old[0]!.material);
    assert.equal(main.old[0]!.disposed.texture, 0);
    t.mock.restoreAll();
  }
});

test("a duplicated fit cannot stand in for a missing head/style and rejected resources are freed once", async t => {
  const main = figure(), beards = beardPack(), url = `${base}models/persona-male.json`;
  beards.root.children[15]!.name = beards.root.children[0]!.name;
  const { host, requested } = network(t, new Map([[url, main.root], [`${base}models/beards-male.json`, beards.root]]));
  await sceneLoadModel(host, url);
  assert.equal(main.root.userData.beardsReady, false);
  assert.equal(beards.root.parent, null);
  assert.deepEqual(beards.disposed, { geometry: 1, material: 1, texture: 1, image: 1 });
  assert.ok(!requested.some(u => u.endsWith("beards-skin.json")));
});

test("incomplete clean skin pack cannot partially replace one of the original heads", async t => {
  const main = figure(), beards = beardPack(), skin = skinPack(), url = `${base}models/persona-male.json`;
  skin.b.material.normalMap = null;
  const { host } = network(t, new Map([
    [url, main.root], [`${base}models/beards-male.json`, beards.root], [`${base}models/beards-skin.json`, skin.root],
  ]));
  await sceneLoadModel(host, url);
  assert.equal(main.root.userData.beardsReady, false);
  assert.equal((main.root.getObjectByName("Head_m2") as THREE.Mesh).material, main.old[0]!.material);
  assert.equal(main.old[0]!.disposed.material, 0);
  assert.equal(skin.a.disposed.geometry, 1);
  assert.equal(skin.a.disposed.material, 1);
  assert.equal(skin.b.disposed.material, 1);
  assert.equal(beards.disposed.geometry, 1);
});

test("female and unrelated assets never request beard packs; a main-model failure still rejects", async t => {
  const female = new THREE.Group(), url = `${base}models/persona-female.json`;
  const { host, requested } = network(t, new Map([[url, female]]));
  assert.equal((await sceneLoadModel(host, url)).scene, female);
  assert.deepEqual(requested, [url]);
  await assert.rejects(sceneLoadModel(host, `${base}models/missing.json`));
});
