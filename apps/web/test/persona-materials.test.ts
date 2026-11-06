import assert from "node:assert/strict";
import test from "node:test";
import * as THREE from "three";
import { clone } from "three/addons/utils/SkeletonUtils.js";
import { sceneApplyLook, sceneOwnMaterials } from "../../../packages/runtime/src/index.ts";

// A second player's skin/hair selection must never repaint the first player or asset cache.
for (const sex of ["male", "female"]) {
  test(`two ${sex} Personas keep independent skin and eye uniforms`, () => {
    const source = new THREE.Group();
    const skin = new THREE.MeshStandardMaterial({ color: 0xffffff, map: new THREE.Texture() });
    skin.name = sex === "male" ? "M_Male_Body" : "M_Fem_Body";
    const head = skin.clone();
    head.name = `M_Head_${sex === "male" ? "m1" : "f1"}`;
    source.add(new THREE.Mesh(new THREE.BoxGeometry(), skin), new THREE.Mesh(new THREE.BoxGeometry(), head));
    const a = clone(source), b = clone(source);
    sceneOwnMaterials(a); sceneOwnMaterials(b);
    const material = (root: THREE.Object3D, index: number) => (root.children[index] as THREE.Mesh).material as THREE.MeshStandardMaterial;
    sceneApplyLook(a, { sex, skin: "deep", eyes: "blue", hair: sex === "male" ? "buzz" : "shaved" });
    const painted = material(a, 0).color.toArray();
    const eyes = structuredClone(material(a, 1).userData.cdLook.cdIris);
    sceneApplyLook(b, { sex, skin: "light", eyes: "gold", hair: sex === "male" ? "buzz" : "shaved" });
    assert.deepEqual(material(a, 0).color.toArray(), painted);
    assert.deepEqual(material(a, 1).userData.cdLook.cdIris, eyes);
    assert.deepEqual(skin.color.toArray(), [1, 1, 1]);
    assert.equal(head.userData.cdLook, undefined);
    assert.notEqual(material(a, 0), material(b, 0));
    assert.equal(material(a, 0).map, skin.map, "immutable texture data stays shared");
    const owned = material(a, 0);
    sceneApplyLook(a, { sex, skin: "warm" });
    assert.equal(material(a, 0), owned, "repaint does not allocate another material");
  });
}

