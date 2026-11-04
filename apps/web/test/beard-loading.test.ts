import assert from "node:assert/strict";
import { test, type TestContext } from "node:test";
import * as THREE from "three";
import { sceneApplyLook, sceneLoadModel } from "../../../packages/runtime/src/index.ts";

// Beards and hairstyles are one small file per piece (scripts/split-packs.ts); a Persona downloads
// only the ones its look shows, when the look first asks for them (beards.rs).
const base = "https://beards.test/game/";
type Counts = { geometry: number; material: number; texture: number; image: number };
const headPositions = [.01, .1, .02, .04, .1, .02, .01, .13, .02, .01, .1, .02];
function resources() {
  const disposed: Counts = { geometry: 0, material: 0, texture: 0, image: 0 };
  const geometry = new THREE.BufferGeometry();
  geometry.setAttribute("position", new THREE.Float32BufferAttribute(headPositions, 3));
  geometry.setAttribute("normal", new THREE.Float32BufferAttribute([0, 0, 1, 0, 0, 1, 0, 0, 1, 0, 0, 1], 3));
  geometry.setAttribute("uv", new THREE.Float32BufferAttribute([0, 0, 1, 0, 0, 1, 1, 1], 2));
  geometry.setIndex([0, 1, 2, 3, 2, 1]);
  geometry.addEventListener("dispose", () => disposed.geometry++);
  const image = { close() { disposed.image++; } };
  const texture = new THREE.Texture(image);
  texture.addEventListener("dispose", () => disposed.texture++);
  const material = new THREE.MeshStandardMaterial({ map: texture });
  material.addEventListener("dispose", () => disposed.material++);
  return { disposed, geometry, material, texture };
}
function figure() {
  const root = new THREE.Group();
  const anchor = new THREE.Bone(); anchor.name = "head";
  anchor.position.set(.01, 1.7, .03); anchor.rotation.y = .3;
  root.add(anchor);
  const old = [resources(), resources()];
  for (const [i, id] of ["m2", "m4"].entries()) {
    old[i]!.material.name = `M_Head_${id}`;
    const node = new THREE.Mesh(old[i]!.geometry, old[i]!.material);
    node.name = `Head_${id}`; root.add(node);
  }
  return { root, anchor, old };
}
function piece(names: string[], material = "M_Beard_Cards") {
  const root = new THREE.Group(), owned = resources();
  owned.material.name = material;
  for (const name of names) {
    const node = new THREE.Mesh(owned.geometry, owned.material);
    node.name = name; node.position.set(.02, -.05, .04); root.add(node);
  }
  return { root, ...owned };
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
const settle = async () => { for (let i = 0; i < 200; i++) await new Promise((done) => setImmediate(done)); };
const male = (face: string, beard: string, hair = "buzz") => ({ sex: "male", face, beard, hair });
const shown = (root: THREE.Object3D) => {
  const out: string[] = [];
  root.traverse((n) => { if ((n as THREE.Mesh).isMesh && n.visible && /^(Beard|Hair)_/.test(n.name)) out.push(n.name); });
  return out.sort();
};

test("a male Persona loads without any beard or hairstyle download and opens the beard choices", async t => {
  const main = figure(), url = `${base}models/persona-male.json`;
  const { host, requested } = network(t, new Map([[url, main.root]]));
  const result = await sceneLoadModel(host, url);
  assert.equal(result.scene, main.root);
  assert.equal(main.root.userData.beardsReady, true);
  assert.deepEqual(requested, [url]);
});

test("the picked beard downloads once, hangs under the head bone and shows; the heads stay untouched", async t => {
  const main = figure(), url = `${base}models/persona-male.json`;
  const beard = piece(["Beard_m2_boxed_cards", "Beard_m2_boxed_root"]);
  const file = `${base}models/beard-m2-boxed.json`;
  const other = piece(["Beard_m2_goatee_cards"]), next = `${base}models/beard-m2-goatee.json`;
  const { host, requested } = network(t, new Map([[url, main.root], [file, beard.root], [next, other.root]]));
  await sceneLoadModel(host, url);
  sceneApplyLook(main.root, male("m2", "boxed"));
  sceneApplyLook(main.root, male("m2", "boxed"));
  await settle();
  assert.deepEqual(requested, [url, file]);
  const worn = main.root.getObjectByName("Beard_m2_boxed_cards") as THREE.Mesh;
  assert.ok(worn, "the beard is on");
  assert.equal(worn.parent!.parent, main.anchor);
  assert.notEqual(worn.material, beard.material, "each Persona tints a material of its own");
  assert.equal(worn.geometry, beard.geometry);
  assert.deepEqual(shown(main.root), ["Beard_m2_boxed_cards", "Beard_m2_boxed_root"]);
  main.root.updateMatrixWorld(true);
  const before = worn.getWorldPosition(new THREE.Vector3());
  main.anchor.rotation.y += .5; main.root.updateMatrixWorld(true);
  assert.ok(before.distanceTo(worn.getWorldPosition(new THREE.Vector3())) > .001);
  for (const [i, id] of ["m2", "m4"].entries()) {
    const head = main.root.getObjectByName(`Head_${id}`) as THREE.Mesh;
    assert.equal(head.geometry, main.old[i]!.geometry);
    assert.equal(head.material, main.old[i]!.material);
  }
  // another beard: only its own file, the first one stays but hides
  sceneApplyLook(main.root, male("m2", "goatee"));
  await settle();
  assert.deepEqual(requested, [url, file, next]);
  assert.deepEqual(shown(main.root), ["Beard_m2_goatee_cards"]);
  // back to the first beard: already on this Persona, nothing to download
  sceneApplyLook(main.root, male("m2", "boxed"));
  await settle();
  assert.deepEqual(requested, [url, file, next]);
  assert.deepEqual(shown(main.root), ["Beard_m2_boxed_cards", "Beard_m2_boxed_root"]);
});

test("two Personas with the same beard share one download", async t => {
  const a = figure(), b = figure(), url = `${base}models/persona-male.json`;
  const beard = piece(["Beard_m3_full_cards"]), file = `${base}models/beard-m3-full.json`;
  const scenes = new Map([[url, a.root], [file, beard.root]]);
  const { host, requested } = network(t, scenes);
  await sceneLoadModel(host, url);
  b.root.userData = { ...a.root.userData };
  sceneApplyLook(a.root, male("m3", "full"));
  sceneApplyLook(b.root, male("m3", "full"));
  await settle();
  assert.equal(requested.filter((u) => u === file).length, 1);
  assert.ok(a.root.getObjectByName("Beard_m3_full_cards"));
  assert.ok(b.root.getObjectByName("Beard_m3_full_cards"));
  assert.notEqual(a.root.getObjectByName("Beard_m3_full_cards"), b.root.getObjectByName("Beard_m3_full_cards"));
});

test("a missing or corrupt beard file leaves the Persona as it was and is tried again later", async t => {
  for (const corrupt of [false, true]) {
    const main = figure(), url = `${base}models/persona-male.json`, file = `${base}models/beard-m4-tapered.json`;
    const scenes = new Map([[url, main.root]]);
    if (corrupt) scenes.set(file, piece(["Beard_m4_tapered_cards"]).root);
    const { host, requested } = network(t, scenes, new Set(corrupt ? [file] : []));
    await sceneLoadModel(host, url);
    sceneApplyLook(main.root, male("m4", "tapered"));
    await settle();
    assert.equal(main.root.getObjectByName("Beard_m4_tapered_cards"), undefined);
    assert.equal((main.root.getObjectByName("Head_m2") as THREE.Mesh).material, main.old[0]!.material);
    sceneApplyLook(main.root, male("m4", "tapered"));
    await settle();
    assert.equal(requested.filter((u) => u === file).length, 2);
    t.mock.restoreAll();
  }
});

test("a female Persona downloads only her picked hairstyle, never a beard", async t => {
  const { root, anchor } = figure(), url = `${base}models/persona-female.json`;
  const bob = piece(["Hair_bob_cards", "Hair_bob_cap"], "M_Hair_Cards"), file = `${base}models/hair-female-bob.json`;
  const { host, requested } = network(t, new Map([[url, root], [file, bob.root]]));
  assert.equal((await sceneLoadModel(host, url)).scene, root);
  assert.deepEqual(requested, [url]);
  assert.equal(root.userData.beardsReady, undefined);
  sceneApplyLook(root, { sex: "female", face: "f1", hair: "bob", beard: "full" });
  await settle();
  assert.deepEqual(requested, [url, file]);
  assert.equal(root.getObjectByName("Hair_bob_cards")!.parent!.parent, anchor);
  assert.deepEqual(shown(root), ["Hair_bob_cap", "Hair_bob_cards"]);
});

test("a male hairstyle hangs next to his beard, each from its own file", async t => {
  const { root, anchor } = figure(), url = `${base}models/persona-male.json`;
  const crop = piece(["Hair_crop_cards", "Hair_crop_cap"], "M_Hair_Cards");
  const beard = piece(["Beard_m1_stubble_cards"]);
  const files = [`${base}models/beard-m1-stubble.json`, `${base}models/hair-male-crop.json`];
  const { host, requested } = network(t, new Map([[url, root], [files[0]!, beard.root], [files[1]!, crop.root]]));
  await sceneLoadModel(host, url);
  sceneApplyLook(root, male("m1", "stubble", "crop"));
  await settle();
  assert.deepEqual(requested.slice(1).sort(), files);
  assert.equal(root.getObjectByName("Hair_crop_cards")!.parent!.parent, anchor);
  assert.equal(root.getObjectByName("Beard_m1_stubble_cards")!.parent!.parent, anchor);
  assert.deepEqual(shown(root), ["Beard_m1_stubble_cards", "Hair_crop_cap", "Hair_crop_cards"]);
});
