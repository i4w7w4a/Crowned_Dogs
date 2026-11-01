import assert from "node:assert/strict";
import { test, type TestContext } from "node:test";
import * as THREE from "three";
import { sceneLoadModel } from "../../../packages/runtime/src/index.ts";

const base = "https://beards.test/game/";
const styles = ["boxed", "goatee", "full", "tapered"];
type Counts = { geometry: number; material: number; texture: number; image: number };
const headPositions = [.01, .1, .02, .04, .1, .02, .01, .13, .02, .01, .1, .02];
const repairedNormals = [0, 0, 1, .6, 0, .8, 0, .6, .8, 0, 0, 1];
function resources() {
  const disposed: Counts = { geometry: 0, material: 0, texture: 0, image: 0 };
  const geometry = new THREE.BufferGeometry();
  geometry.setAttribute("position", new THREE.Float32BufferAttribute(headPositions, 3));
  geometry.setAttribute("normal", new THREE.Float32BufferAttribute([0, 0, 1, 0, 0, 1, 0, 0, 1, 0, 0, 1], 3));
  geometry.setAttribute("uv", new THREE.Float32BufferAttribute([0, 0, 1, 0, 0, 1, 1, 1], 2));
  geometry.setAttribute("skinIndex", new THREE.Uint16BufferAttribute(new Array(16).fill(0), 4));
  geometry.setAttribute("skinWeight", new THREE.Float32BufferAttribute([1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0], 4));
  geometry.setIndex([0, 1, 2, 3, 2, 1]);
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
  const animated = resources(), authored = resources();
  for (const [prefix, resource] of [["CleanNormals", animated], ["CleanNormalsAuthored", authored]] as const) {
    const order = [2, 3, 1, 0]; // Deliberately reordered, including the UV seam duplicate.
    const positions = order.flatMap(i => headPositions.slice(i * 3, i * 3 + 3));
    positions[0]! += .0000002; // Smaller than the 0.5µm matching budget.
    resource.geometry.setAttribute("position", new THREE.Float32BufferAttribute(positions, 3));
    resource.geometry.setAttribute("normal", new THREE.Float32BufferAttribute(order.flatMap(i =>
      prefix === "CleanNormals" ? repairedNormals.slice(i * 3, i * 3 + 3) : [.6, 0, .8]), 3));
    for (const id of ["m2", "m4"]) {
      const node = new THREE.Mesh(resource.geometry, resource.material);
      node.name = `${prefix}_${id}`; root.add(node);
    }
  }
  return { root, a, b, animated, authored };
}
function closeArray(actual: ArrayLike<number>, expected: ArrayLike<number>, tolerance = 1e-6) {
  assert.equal(actual.length, expected.length);
  for (let i = 0; i < actual.length; i++) assert.ok(Math.abs(actual[i]! - expected[i]!) <= tolerance, `index ${i}: ${actual[i]} vs ${expected[i]}`);
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
  const head = main.root.getObjectByName("Head_m2") as THREE.Mesh;
  assert.notEqual(head.geometry, main.old[0]!.geometry);
  closeArray(head.geometry.getAttribute("normal").array, repairedNormals);
  for (const name of ["position", "uv", "skinIndex", "skinWeight"]) {
    assert.deepEqual(head.geometry.getAttribute(name).array, main.old[0]!.geometry.getAttribute(name).array);
  }
  assert.deepEqual(head.geometry.index!.array, main.old[0]!.geometry.index!.array);
  assert.ok([...head.geometry.getAttribute("tangent").array].every(Number.isFinite));
  assert.ok(head.geometry.boundingBox && head.geometry.boundingSphere);
  assert.equal(main.old[0]!.disposed.geometry, 1);
  assert.deepEqual(skin.animated.disposed, { geometry: 1, material: 1, texture: 1, image: 1 });
  assert.deepEqual(skin.authored.disposed, { geometry: 1, material: 1, texture: 1, image: 1 });
  assert.equal(main.root.getObjectByName("CleanNormals_m2"), undefined);
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
  closeArray((main.root.getObjectByName("Head_m4") as THREE.Mesh).geometry.getAttribute("normal").array,
    [.6, 0, .8, .6, 0, .8, .6, 0, .8, .6, 0, .8]);
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

test("small seam repair targets follow coordinate matches without changing UVs, indices or skinning", async t => {
  const main = figure(), beards = beardPack(), skin = skinPack(), url = `${base}models/persona-male.json`;
  const carrierPositions = skin.animated.geometry.getAttribute("position");
  const targets = Array.from(carrierPositions.array, (v, i) => v + (i % 3 === 2 ? .001 : 0));
  skin.animated.geometry.setAttribute("_clean_position", new THREE.Float32BufferAttribute(targets, 3));
  const { host } = network(t, new Map([
    [url, main.root], [`${base}models/beards-male.json`, beards.root], [`${base}models/beards-skin.json`, skin.root],
  ]));
  await sceneLoadModel(host, url);
  assert.equal(main.root.userData.beardsReady, true);
  for (const [i, id] of ["m2", "m4"].entries()) {
    const geometry = (main.root.getObjectByName(`Head_${id}`) as THREE.Mesh).geometry;
    closeArray(geometry.getAttribute("position").array, headPositions.map((v, j) => v + (j % 3 === 2 ? .001 : 0)));
    closeArray(main.old[i]!.geometry.getAttribute("position").array, headPositions);
    closeArray(geometry.getAttribute("normal").array, repairedNormals);
    for (const name of ["uv", "skinWeight", "skinIndex"]) {
      assert.deepEqual(geometry.getAttribute(name).array, main.old[i]!.geometry.getAttribute(name).array);
    }
    assert.deepEqual(geometry.index!.array, main.old[i]!.geometry.index!.array);
    assert.ok(Math.abs(geometry.boundingBox!.min.z - .021) < 1e-6);
    assert.ok([...geometry.getAttribute("tangent").array].every(Number.isFinite));
  }
});

test("quantized heads match in physical rest space through weighted bind frames and carrier transforms", async t => {
  const main = figure(), beards = beardPack(), skin = skinPack(), url = `${base}models/persona-male.json`;
  const neck = new THREE.Bone(); neck.name = "neck_01"; neck.position.set(.03, 1.4, -.02); neck.rotation.z = -.2;
  main.root.add(neck); main.root.updateMatrixWorld(true);
  const raw = [3000, 10000, -9000, 12000, 10000, -9000, 3000, 22000, -9000, 3000, 10000, -9000];
  const weightA = [.25, .75, .5, .25];
  const a = new THREE.Matrix4().compose(new THREE.Vector3(.03, 1.4, -.04),
    new THREE.Quaternion().setFromEuler(new THREE.Euler(.1, .2, .3)), new THREE.Vector3(.12, .18, .09));
  const b = new THREE.Matrix4().compose(new THREE.Vector3(.034, 1.402, -.043),
    new THREE.Quaternion().setFromEuler(new THREE.Euler(.12, .19, .28)), new THREE.Vector3(.13, .17, .1));
  const frames = weightA.map(w => new THREE.Matrix4().fromArray(a.elements.map((v, i) => v * w + b.elements[i]! * (1 - w))));
  for (const [i, id] of ["m2", "m4"].entries()) {
    const resource = main.old[i]!;
    resource.geometry.setAttribute("position", new THREE.Int16BufferAttribute(raw, 3, true));
    resource.geometry.setAttribute("skinIndex", new THREE.Uint16BufferAttribute([0, 1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0], 4));
    resource.geometry.setAttribute("skinWeight", new THREE.Float32BufferAttribute(weightA.flatMap(w => [w, 1 - w, 0, 0]), 4));
    const mesh = new THREE.SkinnedMesh(resource.geometry, resource.material); mesh.name = `Head_${id}`;
    const skeleton = new THREE.Skeleton([main.anchor, neck], [main.anchor.matrixWorld.clone().invert().multiply(a), neck.matrixWorld.clone().invert().multiply(b)]);
    mesh.bind(skeleton, new THREE.Matrix4());
    main.root.remove(main.root.getObjectByName(mesh.name)!); main.root.add(mesh);
  }
  main.root.updateMatrixWorld(true);
  const positions = main.old[0]!.geometry.getAttribute("position");
  const physical = frames.map((m, i) => new THREE.Vector3().fromBufferAttribute(positions, i).applyMatrix4(m));
  const corrected = physical.map(p => p.clone().add(new THREE.Vector3(.0004, -.0003, .0007)));
  const physicalNormal = new THREE.Vector3(.2, .7, .6).normalize();
  skin.root.position.set(.2, .3, -.1); skin.root.rotation.set(.1, -.2, .15); skin.root.scale.set(1.2, .9, 1.1);
  skin.root.updateMatrixWorld(true);
  const carrierInverse = skin.root.matrixWorld.clone().invert();
  const carrierNormal = physicalNormal.clone().applyMatrix3(new THREE.Matrix3().setFromMatrix4(skin.root.matrixWorld).transpose()).normalize();
  const order = [2, 3, 1, 0];
  skin.animated.geometry.setAttribute("position", new THREE.Float32BufferAttribute(order.flatMap(i => physical[i]!.clone().applyMatrix4(carrierInverse).toArray()), 3));
  skin.animated.geometry.setAttribute("normal", new THREE.Float32BufferAttribute(order.flatMap(() => carrierNormal.toArray()), 3));
  skin.animated.geometry.setAttribute("_clean_position", new THREE.Float32BufferAttribute(order.flatMap(i => corrected[i]!.clone().applyMatrix4(carrierInverse).toArray()), 3));
  const { host } = network(t, new Map([
    [url, main.root], [`${base}models/beards-male.json`, beards.root], [`${base}models/beards-skin.json`, skin.root],
  ]));
  await sceneLoadModel(host, url);
  assert.equal(main.root.userData.beardsReady, true);
  const mesh = main.root.getObjectByName("Head_m2") as THREE.SkinnedMesh;
  for (let i = 0; i < 4; i++) {
    const vertex = new THREE.Vector3().fromBufferAttribute(mesh.geometry.getAttribute("position"), i);
    mesh.applyBoneTransform(i, vertex).applyMatrix4(mesh.matrixWorld);
    assert.ok(vertex.distanceTo(corrected[i]!) < .0000005, `physical repaired vertex ${i}`);
    const expected = physicalNormal.clone().applyMatrix3(new THREE.Matrix3().setFromMatrix4(frames[i]!).transpose()).normalize();
    const actual = new THREE.Vector3().fromBufferAttribute(mesh.geometry.getAttribute("normal"), i);
    assert.ok(actual.distanceTo(expected) < .000002, `raw bind-space normal ${i}`);
  }
  assert.deepEqual(main.old[0]!.geometry.getAttribute("position").array, new Int16Array(raw));
  assert.deepEqual(mesh.geometry.getAttribute("skinWeight").array, main.old[0]!.geometry.getAttribute("skinWeight").array);
  assert.deepEqual(mesh.geometry.index!.array, main.old[0]!.geometry.index!.array);
});

test("missing or invalid normal/repair data leaves both original heads untouched and frees prepared geometry", async t => {
  for (const fault of ["missing", "mismatch", "nonfinite", "zero", "over-budget"] as const) {
    const main = figure(), beards = beardPack(), skin = skinPack(), url = `${base}models/persona-male.json`;
    let preparedCount = 0, preparedDisposed = 0;
    for (const old of main.old) {
      const clone = old.geometry.clone.bind(old.geometry);
      t.mock.method(old.geometry, "clone", () => {
        preparedCount++;
        const result = clone();
        result.addEventListener("dispose", () => preparedDisposed++);
        return result;
      });
    }
    const carrier = skin.root.getObjectByName("CleanNormals_m4") as THREE.Mesh;
    if (fault === "missing") skin.root.remove(carrier);
    else {
      carrier.geometry = carrier.geometry.clone();
      if (fault === "mismatch") {
        const position = carrier.geometry.getAttribute("position");
        for (let i = 0; i < position.count; i++) position.setX(i, position.getX(i) + .000003);
      } else if (fault === "nonfinite" || fault === "zero") {
        carrier.geometry.getAttribute("normal").setXYZ(0, fault === "zero" ? 0 : NaN, 0, 0);
      } else {
        carrier.geometry.setAttribute("_CLEAN_POSITION", new THREE.Float32BufferAttribute(
          Array.from(carrier.geometry.getAttribute("position").array, (v, i) => v + (i % 3 === 2 ? .003 : 0)), 3));
      }
    }
    const { host } = network(t, new Map([
      [url, main.root], [`${base}models/beards-male.json`, beards.root], [`${base}models/beards-skin.json`, skin.root],
    ]));
    await sceneLoadModel(host, url);
    assert.equal(main.root.userData.beardsReady, false, fault);
    for (const [i, id] of ["m2", "m4"].entries()) {
      const head = main.root.getObjectByName(`Head_${id}`) as THREE.Mesh;
      assert.equal(head.geometry, main.old[i]!.geometry, fault);
      assert.equal(head.material, main.old[i]!.material, fault);
      assert.deepEqual(main.old[i]!.disposed, { geometry: 0, material: 0, texture: 0, image: 0 }, fault);
    }
    assert.equal(preparedCount, fault === "missing" ? 0 : 1, fault);
    assert.equal(preparedDisposed, preparedCount, fault);
    assert.equal(skin.a.disposed.geometry, 1, fault);
    assert.deepEqual(skin.animated.disposed, { geometry: 1, material: 1, texture: 1, image: 1 }, fault);
    assert.deepEqual(skin.authored.disposed, { geometry: 1, material: 1, texture: 1, image: 1 }, fault);
    assert.deepEqual(beards.disposed, { geometry: 1, material: 1, texture: 1, image: 1 }, fault);
    assert.equal(beards.root.parent, null);
    t.mock.restoreAll();
  }
});

test("retired head resources shared with another body part stay alive", async t => {
  const main = figure(), beards = beardPack(), skin = skinPack(), url = `${base}models/persona-male.json`;
  main.root.add(new THREE.Mesh(main.old[0]!.geometry, main.old[0]!.material));
  const { host } = network(t, new Map([
    [url, main.root], [`${base}models/beards-male.json`, beards.root], [`${base}models/beards-skin.json`, skin.root],
  ]));
  await sceneLoadModel(host, url);
  assert.equal(main.root.userData.beardsReady, true);
  assert.deepEqual(main.old[0]!.disposed, { geometry: 0, material: 0, texture: 0, image: 0 });
  assert.deepEqual(main.old[1]!.disposed, { geometry: 1, material: 1, texture: 1, image: 1 });
});

test("female and unrelated assets never request beard packs; a main-model failure still rejects", async t => {
  const female = new THREE.Group(), url = `${base}models/persona-female.json`;
  const { host, requested } = network(t, new Map([[url, female]]));
  assert.equal((await sceneLoadModel(host, url)).scene, female);
  assert.deepEqual(requested, [url]);
  await assert.rejects(sceneLoadModel(host, `${base}models/missing.json`));
});
