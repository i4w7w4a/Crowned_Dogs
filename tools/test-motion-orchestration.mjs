// Real Three skeleton/mixer parity against the frozen browser90 implementation.
// Runs without a renderer, browser, network or audio device.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import ts from "typescript";
import * as THREE from "three";
import {
  RustMotion, RustPose, RustGrip, RustFeet, RustCollapse, RustViewmodel, RustHoldOut, RustFlash, RustDust, RustArmsOnly,
  rig_set_world_quat, rig_turn, rig_reach, rig_gait_of, rig_find_bones, rig_axes_pitch, rig_axes_roll, callRust, motionCompute,
} from "../packages/runtime/src/index.ts";

let nextFixture = 0;
async function original(file, extra = {}) {
  const key = `__motionFixture${++nextFixture}`;
  const bindings = { THREE, motionCompute, callRust, ...extra };
  globalThis[key] = bindings;
  const source = readFileSync(resolve(import.meta.dirname, "../crates/game-wasm/reference/browser90", file), "utf8");
  const javascript = ts.transpileModule(source, { compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 } }).outputText.replace(/^import .*?;\s*$/gm, "");
  const module = await import(`data:text/javascript;base64,${Buffer.from(`const {${Object.keys(bindings).join(",")}} = globalThis.${key};\n${javascript}`).toString("base64")}`);
  delete globalThis[key];
  return module;
}
function equal(actual, expected, path = "motion") {
  if (typeof actual === "number" && typeof expected === "number") {
    assert.ok(Number.isFinite(actual) && Math.abs(actual - expected) < 1e-9 * Math.max(1, Math.abs(expected)), `${path}: ${actual} != ${expected}`);
  } else if (actual && expected && typeof actual === "object" && typeof expected === "object") {
    assert.deepEqual(Object.keys(actual), Object.keys(expected), path);
    for (const key of Object.keys(actual)) equal(actual[key], expected[key], `${path}.${key}`);
  } else assert.equal(actual, expected, path);
}
function rig() {
  const group = new THREE.Group(), model = new THREE.Group(); group.add(model);
  const bones = {};
  function bone(name, parent, x, y, z) {
    const b = new THREE.Bone(); b.name = name; b.position.set(x, y, z); parent.add(b); bones[name] = b; return b;
  }
  const pelvis = bone("pelvis", model, 0, 1, 0);
  const spine = bone("spine_01", pelvis, 0, .18, 0), spine1 = bone("spine_02", spine, 0, .16, 0), spine2 = bone("spine_03", spine1, 0, .14, 0);
  const neck = bone("neck_01", spine2, 0, .12, 0); bone("head", neck, 0, .13, 0);
  for (const [side, sign] of [["l", 1], ["r", -1]]) {
    const thigh = bone(`thigh_${side}`, pelvis, sign * .13, -.07, 0), calf = bone(`calf_${side}`, thigh, 0, -.4, .015);
    const foot = bone(`foot_${side}`, calf, 0, -.4, -.015); bone(`ball_${side}`, foot, 0, -.06, .17);
    const upper = bone(`upperarm_${side}`, spine2, sign * .2, .02, 0), lower = bone(`lowerarm_${side}`, upper, sign * .24, -.1, 0);
    const hand = bone(`hand_${side}`, lower, sign * .23, -.07, .02);
    for (const [i, finger] of ["thumb", "index", "middle", "ring", "pinky"].entries()) {
      let parent = hand;
      for (let n = 1; n <= 3; n++) parent = bone(`${finger}_0${n}_${side}`, parent, sign * .026, n === 1 ? (i - 2) * .013 : 0, n === 1 ? .015 : 0);
    }
  }
  group.position.set(.1, .08, -.2); group.rotation.y = .2; group.updateMatrixWorld(true);
  return { group, model, bones };
}
function state(r) {
  r.group.updateMatrixWorld(true);
  return Object.fromEntries(Object.entries(r.bones).map(([name, b]) => [name, [...b.position.toArray(), ...b.quaternion.toArray(), ...b.scale.toArray()]]));
}
function clips() {
  const names = ["idle", "ready_pistol", "ready_rifle", "ready_shotgun", "walk", "run", "fire_pistol", "fire_rifle", "fire_shotgun", "attack_knife", "grenade_throw"];
  return names.map((name, i) => {
    const quats = [0, .5, 1, 1.5, 2].flatMap(t => new THREE.Quaternion().setFromEuler(new THREE.Euler(.04 * Math.sin(t * 4 + i), .06 * Math.cos(t * 5 + i), .03 * Math.sin(t * 3))).toArray());
    return new THREE.AnimationClip(name, 2, [
      new THREE.QuaternionKeyframeTrack("pelvis.quaternion", [0, .5, 1, 1.5, 2], quats),
      new THREE.QuaternionKeyframeTrack("spine_01.quaternion", [0, .5, 1, 1.5, 2], quats.slice().reverse().map((v, n) => n % 4 === 3 ? 1 : v * .1)),
      new THREE.VectorKeyframeTrack("foot_l.position", [0, .5, 1, 1.5, 2], [0,-.4,.15, 0,-.4,-.15, 0,-.29,.15, 0,-.4,-.15, 0,-.4,.15]),
      new THREE.VectorKeyframeTrack("foot_r.position", [0, .5, 1, 1.5, 2], [0,-.29,-.15, 0,-.4,.15, 0,-.4,-.15, 0,-.29,.15, 0,-.29,-.15]),
    ]);
  });
}
const oldIK = await original("ik.ts");
const oldMotion = await original("motion.ts");
const oldFall = await original("fall.ts", { ...oldIK, aim: oldIK.turn });
const oldHold = await original("hold.ts", oldIK);
const oldRig = await original("rig.ts");
const oldDust = await original("dust.ts");

function constraints(useRust) {
  const r = rig(), b = r.bones;
  const world = useRust ? (...args) => rig_set_world_quat(THREE, ...args) : oldIK.setWorldQuat;
  const turn = useRust ? (...args) => rig_turn(THREE, ...args) : oldIK.turn;
  const reach = useRust ? (...args) => rig_reach(THREE, ...args) : oldIK.reach;
  world(b.spine_01, new THREE.Quaternion().setFromEuler(new THREE.Euler(.1, -.1, .03)));
  turn(b.upperarm_l, new THREE.Vector3(1, 0, 0), new THREE.Vector3(.8, -.2, .3).normalize());
  reach(b.upperarm_r, b.lowerarm_r, b.hand_r, new THREE.Vector3(-.2, 1.1, .4), new THREE.Vector3(-.5, .8, .1));
  const grip = useRust ? new RustGrip(THREE, r.model, "r") : new oldIK.Grip(r.model, "r");
  for (const w of [.1, .5, 1]) grip.close({ thumb: [10,20,30], index: [0,15,25], middle: [30,40,50] }, w);
  const hold = useRust ? new RustHoldOut(THREE, r.model) : new oldHold.HoldOut(r.model);
  hold.apply(r.group, .005); hold.apply(r.group, .5); hold.apply(r.group, 1);
  const output = state(r); if (useRust) { grip.free(); hold.free(); }
  return output;
}
equal(constraints(true), constraints(false), "IK/grip/hold");

function footprints(useRust) {
  const r = rig(), animation = clips(), before = state(r);
  const gait = useRust ? rig_gait_of(THREE, r.model, animation[4]) : oldIK.gaitOf(r.model, animation[4]);
  equal(state(r), before, "scratch gait restores rest pose");
  assert.ok(gait?.speed > .1, "fixture exercises a measurable walk cycle");
  const feet = useRust ? new RustFeet(THREE, r.model, animation, ["ready_pistol", "ready_rifle", "missing"]) : new oldIK.Feet(r.model, animation, ["ready_pistol", "ready_rifle", "missing"]);
  equal(state(r), before, "scratch footprints restore rest pose");
  const output = [gait];
  for (const [dt, name] of [[.1,"ready_pistol"], [.05,"ready_rifle"], [.08,null], [.25,null], [.6,"missing"]]) { feet.update(dt, name); output.push(state(r)); }
  if (useRust) feet.free(); return output;
}
equal(footprints(true), footprints(false), "gait/footprints");

function animation(useRust) {
  const r = rig(), tracks = clips(), mixer = new THREE.AnimationMixer(r.model);
  const motion = useRust ? new RustMotion(THREE, mixer, tracks) : new oldMotion.Motion(mixer, tracks);
  const pose = useRust ? new RustPose(r.model) : new oldMotion.Pose(r.model);
  const events = [], frames = [];
  motion.onHold = value => events.push(["hold", value]);
  motion.onMark = value => { events.push(["mark", value]); motion.arm("shotgun"); };
  motion.reset(.2); motion.arm("rifle"); motion.walk("walk", .4); motion.pace(1.3);
  function tick(n) { for (let i = 0; i < n; i++) {
    pose.restore(); motion.update(.05); mixer.update(.05); pose.save(); motion.settle();
    frames.push([state(r), motion.attackKind, motion.pistolReady, motion.planted, motion.gait]);
  } }
  tick(4); motion.attack("rifle", .3); tick(30); motion.stand(); tick(4);
  motion.attack("grenade", 1.4); tick(55); motion.fall(); tick(5); motion.reset(.1); tick(2);
  const filtered = tracks.map(clip => clip.tracks.map(track => [...track.values]));
  if (useRust) {
    assert.equal(mixer._listeners.finished.length, 1);
    motion.dispose(); motion.dispose(); assert.equal(mixer._listeners.finished.length, 0, "dispose detaches mixer callback");
    const count = events.length; motion.attack("knife", .1); mixer.update(4); assert.equal(events.length, count);
    motion.free(); pose.free();
  }
  return { events, frames, filtered };
}
equal(animation(true), animation(false), "mixer/timeline/pose");

function collapse(useRust) {
  const output = [];
  for (const side of [-1, 1]) {
    const r = rig(), pose = new oldMotion.Pose(r.model), gun = new THREE.Group();
    gun.position.set(0, .03, .02); gun.rotation.z = .1; r.bones.hand_r.add(gun);
    const fall = useRust ? new RustCollapse(THREE, r.model) : new oldFall.Collapse(r.model); fall.gun = gun; fall.begin(r.group, side);
    for (const t of [0, .1, .5, .9, 1.25, 1.8]) { pose.restore(); fall.apply(t); output.push(state(r)); }
    if (useRust) fall.free();
  }
  return output;
}
equal(collapse(true), collapse(false), "collapse/floor contact");

function finite(r, path) {
  for (const [name, values] of Object.entries(state(r))) assert.ok(values.every(Number.isFinite), `${path}: ${name}`);
}
// The grips were redrawn on purpose (Art, 2026-11-01: a straight pistol wrist, the support hand on the pump,
// the forend or the foregrip), so first person and the support hand no longer match browser90; they must stay sane.
function firstPerson() {
  const r = rig(), mount = new THREE.Group(); mount.position.set(.02, .01, -.03); mount.scale.setScalar(.9); r.bones.hand_r.add(mount);
  const hand = { mount, foregrip: false }, camera = new THREE.PerspectiveCamera(50, 9/16, .1, 50); camera.position.set(0, 1.7, 0); camera.updateMatrixWorld(true);
  const view = new RustViewmodel(THREE, r.model, hand, new THREE.Matrix4());
  const pose = new oldMotion.Pose(r.model);
  for (const [stance, foregrip] of [["pistol", false], ["pistol", true], ["rifle", false], ["rifle", true], ["shotgun", false], ["sniper", false]]) {
    hand.foregrip = foregrip;
    for (let frame = 0; frame < 12; frame++) {
      pose.restore(); if (frame === 4) view.shot();
      view.update(.04, frame < 10, frame > 2 && frame < 8, stance, camera, new THREE.Vector3(.1, 1.5, -6));
      finite(r, `first person ${stance}`);
    }
  }
  view.off(); equal(mount.position.toArray(), [.02,.01,-.03], "viewmodel off restores mount");
  view.free();
}
firstPerson();

function support() {
  const r = rig(), b = r.bones, clavicle = new THREE.Bone(), mount = new THREE.Group();
  clavicle.name = "clavicle_l"; clavicle.position.set(.05, .1, 0); b.spine_03.add(clavicle); clavicle.attach(b.upperarm_l);
  mount.position.set(.02, .01, -.03); b.hand_r.add(mount); r.group.updateMatrixWorld(true);
  const hold = new RustHoldOut(THREE, r.model), before = state(r);
  hold.support({ held: "rifle", mount, foregrip: false }, 0); equal(state(r), before, "support waits for a raised gun");
  hold.support({ held: "knife", mount, foregrip: false }, 1); equal(state(r), before, "a knife takes no support hand");
  for (const [held, foregrip] of [["pistol", false], ["pistol", true], ["rifle", false], ["rifle", true], ["shotgun", false], ["sniper", false]]) {
    hold.support({ held, mount, foregrip }, 1); finite(r, `support ${held}`);
  }
  hold.free();
}
support();

function meshMask(useRust) {
  const r = rig(), bounds = new THREE.Box3(new THREE.Vector3(-1, 0, -1), new THREE.Vector3(1, 2, 1));
  const bones = useRust ? rig_find_bones(THREE, r.model, bounds) : oldRig.findBones(r.model, bounds);
  (useRust ? rig_axes_pitch : oldRig.pitch)(bones.spine, .12); (useRust ? rig_axes_roll : oldRig.roll)(bones.head, -.08);
  const geometry = new THREE.BufferGeometry();
  geometry.setAttribute("position", new THREE.Float32BufferAttribute(new Float32Array(18), 3));
  geometry.setAttribute("skinIndex", new THREE.Uint16BufferAttribute([0,0,0,0, 0,0,0,0, 0,0,0,0, 1,0,0,0, 1,0,0,0, 1,0,0,0], 4));
  geometry.setAttribute("skinWeight", new THREE.Float32BufferAttribute([1,0,0,0, 1,0,0,0, 1,0,0,0, 1,0,0,0, 1,0,0,0, 1,0,0,0], 4)); geometry.setIndex([0,1,2,3,4,5]);
  const mesh = new THREE.SkinnedMesh(geometry, new THREE.MeshBasicMaterial()); mesh.bind(new THREE.Skeleton([r.bones.upperarm_l, r.bones.pelvis])); r.model.add(mesh);
  const originalIndex = geometry.index;
  const mask = useRust ? new RustArmsOnly(THREE, r.model) : oldRig.armsOnly(r.model);
  const frames = [];
  for (const on of [true, false, true]) { useRust ? mask.set(on) : mask(on); frames.push([mesh.visible, [...geometry.index.array]]); }
  if (useRust) { mask.dispose(); mask.dispose(); assert.equal(geometry.index, originalIndex); assert.equal(mesh.visible, true); mask.free(); }
  return { frames, bones: state(r), crown: bones.top.position.toArray() };
}
equal(meshMask(true), meshMask(false), "skeleton/first-person masking");

function particles(useRust) {
  const descriptor = Object.getOwnPropertyDescriptor(globalThis, "crypto");
  Object.defineProperty(globalThis, "crypto", { configurable: true, value: { getRandomValues: out => { out[0] = 123456; return out; } } });
  try {
    const dust = useRust ? new RustDust(THREE) : new oldDust.Dust(), frames = [];
    let previous = dust.points.geometry, disposed = 0;
    previous.addEventListener("dispose", () => disposed++);
    for (const count of [12, 4, 0, 20]) {
      dust.setCount(count);
      if (dust.points.geometry !== previous) { previous = dust.points.geometry; previous.addEventListener("dispose", () => disposed++); }
      frames.push([dust.points.visible, dust.points.geometry.drawRange.count, [...dust.points.geometry.attributes.position.array], [...dust.points.geometry.attributes.seed.array]]);
    }
    dust.update(1.2, 600, new THREE.Vector3(.2,.4,-.8), null, .12, 3);
    const u = dust.points.material.uniforms;
    frames.push([u.time.value,u.pxPerMetre.value,u.sunDir.value.toArray(),u.hasRays.value,u.focusInv.value,u.dofPx.value]);
    assert.equal(disposed, 2, "only capacity growth replaces geometry");
    if (useRust) {
      let materialDisposed = 0; dust.points.material.addEventListener("dispose", () => materialDisposed++);
      dust.dispose(); dust.dispose(); assert.equal(disposed, 3); assert.equal(materialDisposed, 1);
      dust.setCount(100); assert.equal(dust.points.visible, false); dust.free();
    }
    return frames;
  } finally { if (descriptor) Object.defineProperty(globalThis, "crypto", descriptor); else delete globalThis.crypto; }
}
equal(particles(true), particles(false), "dust capacity/uniforms");

function flash(useRust) {
  const random = Math.random; Math.random = () => .375;
  try {
    const map = new THREE.Texture(), flash = useRust ? new RustFlash(THREE, map, 3) : new oldHold.Flash(map, 3), frames = [];
    const hand = { muzzle: out => { out.set(.3,1.5,-2); return true; } };
    flash.fire(hand, .24);
    for (const dt of [0,.01,.025,.035,.1]) { const k = flash.update(dt); frames.push([k,flash.sprite.visible,flash.sprite.position.toArray(),flash.sprite.scale.toArray(),flash.sprite.material.opacity,flash.sprite.material.rotation]); }
    flash.fire({ muzzle: () => false }, .1); assert.equal(flash.update(.01), 0); flash.stop();
    if (useRust) {
      let disposals = 0; flash.sprite.material.addEventListener("dispose", () => disposals++);
      flash.dispose(); flash.dispose(); flash.fire(hand, .2); assert.equal(flash.update(0), 0); assert.equal(disposals, 1); flash.free();
    }
    return frames;
  } finally { Math.random = random; }
}
equal(flash(true), flash(false), "muzzle-flash lifetime");
console.log("Motion orchestration: 9 Three.js parity, grip and lifecycle suites passed (silent).");
