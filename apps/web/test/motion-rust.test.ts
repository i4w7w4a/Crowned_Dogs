import assert from "node:assert/strict";
import { test } from "node:test";
import * as THREE from "three";
import { motionCompute } from "../../../packages/runtime/src/index.ts";
import { reach } from "../src/arena/ik.ts";
import { reach as referenceReach } from "../../../crates/game-motion/reference/ik.ts";
import { Motion } from "../src/arena/motion.ts";
import { Motion as ReferenceMotion } from "../../../crates/game-motion/reference/motion.ts";
import { Viewmodel } from "../src/arena/viewmodel.ts";
import { Viewmodel as ReferenceViewmodel } from "../../../crates/game-motion/reference/viewmodel.ts";
import { Hand, gripOf } from "../src/arena/weapons.ts";

function close(a: ArrayLike<number>, b: ArrayLike<number>, message = "", tolerance = 1e-8): void {
  assert.equal(a.length, b.length, message);
  for (let i = 0; i < a.length; i++) assert.ok(Math.abs(a[i]! - b[i]!) < tolerance, `${message}[${i}]: ${a[i]} != ${b[i]}`);
}
function limb(angle: number): [THREE.Group, THREE.Bone, THREE.Bone, THREE.Bone] {
  const group = new THREE.Group(), upper = new THREE.Bone(), lower = new THREE.Bone(), hand = new THREE.Bone();
  group.position.set(.15, .8, -.4); group.rotation.set(.13, angle, -.2); group.scale.setScalar(.97);
  upper.position.set(.2, .3, .1); upper.rotation.set(.15, .2, -.1);
  lower.position.set(.27, -.08, .06); lower.rotation.set(.05, .25, .14); hand.position.set(.24, -.03, .04);
  group.add(upper); upper.add(lower); lower.add(hand); group.updateMatrixWorld(true);
  return [group, upper, lower, hand];
}
test("Rust two-bone IK agrees with the original Three solver at reachable and unreachable targets", () => {
  for (let i = 0; i < 80; i++) {
    const [, a, b, c] = limb(i * .17), [, aa, bb, cc] = limb(i * .17);
    const target = new THREE.Vector3(.15 + .6 * Math.sin(i * 2.1), .8 + .3 * Math.cos(i), -.2 + .7 * Math.cos(i * .9));
    const pole = new THREE.Vector3(.4, -.5, 1);
    referenceReach(aa, bb, cc, target.clone(), pole.clone()); reach(a, b, c, target.clone(), pole.clone());
    close(c.getWorldPosition(new THREE.Vector3()).toArray(), cc.getWorldPosition(new THREE.Vector3()).toArray(), `IK target ${i}`);
    // q and -q encode the same orientation; matrix decomposition may choose either.
    assert.ok(1 - Math.abs(a.quaternion.dot(aa.quaternion)) < 1e-10, `upper ${i}`);
    assert.ok(1 - Math.abs(b.quaternion.dot(bb.quaternion)) < 1e-10, `lower ${i}`);
  }
});

function animation() {
  const model = new THREE.Group(), pelvis = new THREE.Bone(), spine = new THREE.Bone();
  pelvis.name = "pelvis"; spine.name = "spine_01"; model.add(pelvis); pelvis.add(spine);
  const clips = ["ready_pistol", "ready_rifle", "ready_shotgun", "fire_pistol", "fire_rifle", "fire_shotgun", "attack_knife", "grenade_throw", "walk", "run", "idle"].map((name, i) => {
    const times = [0, .6, 1.2, 1.8, 2.4];
    const values = times.flatMap(t => new THREE.Quaternion().setFromEuler(new THREE.Euler(.1 * Math.sin(t * 2 + i), .14 * Math.cos(t + i), .04)).toArray());
    return new THREE.AnimationClip(name, 2.4, [new THREE.QuaternionKeyframeTrack("pelvis.quaternion", times, values), new THREE.QuaternionKeyframeTrack("spine_01.quaternion", times, values)]);
  });
  return { model, pelvis, spine, clips, mixer: new THREE.AnimationMixer(model) };
}
test("Rust animation timeline preserves clip marks, hands, crossfades and foot planting", () => {
  const a = animation(), b = animation(), rust = new Motion(a.mixer, a.clips), old = new ReferenceMotion(b.mixer, b.clips);
  const rEvents: unknown[] = [], oEvents: unknown[] = [];
  rust.onHold = v => rEvents.push(["hold", v]); old.onHold = v => oEvents.push(["hold", v]);
  rust.onMark = v => rEvents.push(["mark", v]); old.onMark = v => oEvents.push(["mark", v]);
  for (let frame = 0; frame < 700; frame++) {
    for (const m of [rust, old]) {
      if (frame === 0) m.reset(.27);
      if (frame === 5) m.arm("rifle");
      if (frame === 10) m.walk("run", .3);
      if (frame >= 10 && frame < 60) m.pace(.8);
      if (frame === 20) m.attack("rifle", .8);
      if (frame === 60) m.stand();
      if (frame === 180) m.attack("grenade", 1.3);
      if (frame === 350) m.arm("pistol");
      if (frame === 370) m.attack("knife", .2);
      if (frame === 500) m.fall();
      if (frame === 550) m.reset(.71);
      m.update(1 / 60);
    }
    a.mixer.update(1 / 60); b.mixer.update(1 / 60); rust.settle(); old.settle();
    assert.equal(rust.attackKind, old.attackKind, `attack frame ${frame}`);
    assert.equal(rust.planted, old.planted, `planted frame ${frame}`);
    assert.equal(rust.gait, old.gait, `gait frame ${frame}`);
    assert.deepEqual(rEvents, oEvents, `events frame ${frame}`);
    close(a.pelvis.quaternion.toArray(), b.pelvis.quaternion.toArray(), `pelvis ${frame}`, 1e-7);
    close(a.spine.quaternion.toArray(), b.spine.quaternion.toArray(), `spine ${frame}`, 1e-7);
  }
});
test("Rust weapon roll preserves camera-facing controls from arbitrary orientations", () => {
  for (let i = 0; i < 100; i++) {
    const object = new THREE.Object3D(); object.rotation.set(i * .17, i * .31, -.6);
    const input = object.quaternion.toArray(); object.rotateOnWorldAxis(new THREE.Vector3(0, 1, 0), .13); object.rotateOnWorldAxis(new THREE.Vector3(0, 0, 1), -.19);
    close(motionCompute("viewerTurn", new Float64Array([...input, .13, .19])), object.quaternion.toArray());
  }
});
test("Rust arm triangle filtering respects dominant bones and drops torso triangles", () => {
  const indices = [0, 1, 2, 1, 2, 3];
  const packed = new Float64Array([4, 2, indices.length,
    0, 1, 0, 0, 0, 1, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0,
    .1, .9, 0, 0, .2, .8, 0, 0, .8, .2, 0, 0, .9, .1, 0, 0,
    0, 1, ...indices]);
  close(motionCompute("armTriangles", packed), [0, 1, 2]);
});

function arms() {
  const model = new THREE.Group(); model.position.set(0, 1.1, .1);
  for (const side of ["l", "r"] as const) {
    const upper = new THREE.Bone(), lower = new THREE.Bone(), hand = new THREE.Bone();
    upper.name = `upperarm_${side}`; lower.name = `lowerarm_${side}`; hand.name = `hand_${side}`;
    upper.position.set(side === "l" ? -.2 : .2, .35, 0);
    lower.position.set(side === "l" ? -.18 : .18, -.2, .05); hand.position.set(0, -.15, -.2);
    model.add(upper); upper.add(lower); lower.add(hand);
    for (const [index, finger] of ["thumb", "index", "middle", "ring", "pinky"].entries()) {
      let parent: THREE.Object3D = hand;
      for (let joint = 1; joint <= 3; joint++) {
        const bone = new THREE.Bone(); bone.name = `${finger}_0${joint}_${side}`;
        bone.position.set(joint === 1 ? .025 * (index - 2) : 0, 0, joint === 1 ? -.06 : -.025);
        parent.add(bone); parent = bone;
      }
    }
  }
  model.updateMatrixWorld(true);
  const hand = new Hand(model, gripOf(model)!, new THREE.MeshStandardMaterial());
  return { model, hand };
}
test("Rust first-person recoil and support-arm placement preserve all four weapon stances", () => {
  for (const stance of ["pistol", "rifle", "shotgun", "sniper"] as const) {
    const a = arms(), b = arms(), rust = new Viewmodel(a.model, a.hand), old = new ReferenceViewmodel(b.model, b.hand);
    const camera = new THREE.PerspectiveCamera(); camera.position.set(.08, 1.65, .3); camera.lookAt(.1, 1.4, -5); camera.updateMatrixWorld(true);
    const target = new THREE.Vector3(.1, 1.4, -5);
    for (let frame = 0; frame < 90; frame++) {
      if (frame === 20) { rust.shot(); old.shot(); }
      rust.update(1/60, true, frame >= 5 && frame < 70, stance, camera, target);
      old.update(1/60, true, frame >= 5 && frame < 70, stance, camera, target);
      a.model.updateMatrixWorld(true); b.model.updateMatrixWorld(true);
      close(a.hand.mount.getWorldPosition(new THREE.Vector3()).toArray(), b.hand.mount.getWorldPosition(new THREE.Vector3()).toArray(), `${stance} frame ${frame} grip`, 1e-6);
      for (const name of ["hand_l", "hand_r", "lowerarm_l", "lowerarm_r"]) {
        close(a.model.getObjectByName(name)!.getWorldPosition(new THREE.Vector3()).toArray(), b.model.getObjectByName(name)!.getWorldPosition(new THREE.Vector3()).toArray(), `${stance} frame ${frame} ${name}`, 1e-6);
      }
    }
  }
});
