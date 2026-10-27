import assert from "node:assert/strict";
import { test } from "node:test";
import * as THREE from "three";
import { findBones, pitch, roll } from "../src/arena/rig.ts";

/** A bare UE Mannequin chain, with bones rolled the odd ways an export can leave them. */
function ueRig(): THREE.Object3D {
  const root = new THREE.Object3D();
  const chain = (parent: THREE.Object3D, names: string[], y: number, turn: THREE.Euler): void => {
    let p = parent;
    for (const n of names) {
      const b = new THREE.Bone();
      b.name = n;
      b.position.set(0, y, 0);
      b.quaternion.setFromEuler(turn);
      p.add(b);
      p = b;
    }
  };
  const pelvis = new THREE.Bone();
  pelvis.name = "pelvis";
  pelvis.position.set(0, 1, 0);
  pelvis.rotation.set(-Math.PI / 2, 0, Math.PI / 2);
  root.add(pelvis);
  chain(pelvis, ["spine_01", "spine_02", "spine_03", "neck_01", "head"], 0.12, new THREE.Euler(0, 0.1, 0));
  chain(pelvis, ["thigh_r", "calf_r"], 0.45, new THREE.Euler(Math.PI, 0.3, 0));
  chain(pelvis, ["thigh_l", "calf_l"], 0.45, new THREE.Euler(Math.PI, -0.3, 0));
  root.updateMatrixWorld(true);
  return root;
}

const worldQ = (o: THREE.Object3D) => { o.updateWorldMatrix(true, false); return o.getWorldQuaternion(new THREE.Quaternion()); };

test("a UE rig is found by its own bone names", () => {
  const model = ueRig();
  const b = findBones(model, new THREE.Box3(new THREE.Vector3(-0.3, 0, -0.2), new THREE.Vector3(0.3, 1.8, 0.2)));
  assert.equal(b.hips.name, "pelvis");
  assert.equal(b.spine2.name, "spine_03");
  assert.equal(b.rLeg.name, "calf_r");
  assert.equal(b.top.parent, b.head, "the crown marker rides on the head");
  model.updateMatrixWorld(true);
  assert.ok(Math.abs(b.top.getWorldPosition(new THREE.Vector3()).y - 1.8) < 1e-6, "the crown sits at the top of the model");
});

test("a flinch bends about the figure's own lines, however the bone is rolled", () => {
  const model = ueRig();
  const b = findBones(model, new THREE.Box3(new THREE.Vector3(), new THREE.Vector3(1, 1.8, 1)));
  for (const [bend, axis] of [[pitch, new THREE.Vector3(1, 0, 0)], [roll, new THREE.Vector3(0, 0, 1)]] as const) {
    for (const bone of [b.spine2, b.rUpLeg, b.head]) {
      const before = worldQ(bone);
      bend(bone, 0.4);
      const turned = worldQ(bone).multiply(before.clone().invert());
      const want = new THREE.Quaternion().setFromAxisAngle(axis, 0.4);
      assert.ok(Math.abs(turned.dot(want)) > 0.99999, `${bone.name} turned about the wrong line`);
      bend(bone, -0.4);
    }
  }
});

test("a Mixamo rig keeps its own head-top bone", () => {
  const model = new THREE.Object3D();
  for (const n of ["Hips", "Spine", "Spine1", "Spine2", "Neck", "Head", "HeadTop_End", "RightUpLeg", "RightLeg", "LeftUpLeg", "LeftLeg"]) {
    const b = new THREE.Bone();
    b.name = "mixamorig" + n;
    model.add(b);
  }
  model.updateMatrixWorld(true);
  const b = findBones(model, new THREE.Box3());
  assert.equal(b.top.name, "mixamorigHeadTop_End");
});
