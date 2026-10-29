// Bones placed by hand after the animation: a two-bone reach for an arm or a leg, feet planted flat on
// the floor while a figure stands, and fingers closed on a grip.

import * as THREE from "three";
import { motionCompute } from "../../../../packages/runtime/src/index.ts";

const qa = new THREE.Quaternion(), qb = new THREE.Quaternion(), qc = new THREE.Quaternion();
const va = new THREE.Vector3(), vb = new THREE.Vector3(), vc = new THREE.Vector3(), vd = new THREE.Vector3(), ve = new THREE.Vector3();
const UP = new THREE.Vector3(0, 1, 0);

/** Sets a bone's world rotation. */
export function setWorldQuat(o: THREE.Object3D, q: THREE.Quaternion): void {
  o.parent!.getWorldQuaternion(qc);
  o.quaternion.fromArray(motionCompute("worldQuat", new Float64Array([...qc.toArray(), ...q.toArray()])));
  o.updateMatrixWorld(true);
}

/** Turns a bone so that the world direction `from` (unit) it carries points along `to` (unit). */
export function turn(o: THREE.Object3D, from: THREE.Vector3, to: THREE.Vector3): void {
  o.quaternion.fromArray(motionCompute("turn", new Float64Array([
    ...from.toArray(), ...to.toArray(), ...o.getWorldQuaternion(qb).toArray(), ...o.parent!.getWorldQuaternion(qc).toArray(),
  ])));
  o.updateMatrixWorld(true);
}

/** Bends a limb (upper `a`, lower `b`, end `c`) so that `c` lands on `target`, the middle joint towards `pole`.
 *  A target out of reach is met with the limb straight. */
export function reach(a: THREE.Object3D, b: THREE.Object3D, c: THREE.Object3D, target: THREE.Vector3, pole: THREE.Vector3): void {
  // Read the scene once; Rust solves both joints together, with no JSON per bone.
  const pose = motionCompute("reach", new Float64Array([
    ...a.getWorldPosition(va).toArray(), ...b.getWorldPosition(vb).toArray(), ...c.getWorldPosition(vc).toArray(),
    ...target.toArray(), ...pole.toArray(), ...a.getWorldQuaternion(qa).toArray(), ...b.getWorldQuaternion(qb).toArray(),
    ...a.parent!.getWorldQuaternion(qc).toArray(), ...b.parent!.getWorldQuaternion(qc).toArray(),
  ]));
  if (!pose[0]) return;
  a.quaternion.fromArray(pose, 1); b.quaternion.fromArray(pose, 5);
  a.updateMatrixWorld(true);
}

/** Fingers bent from their rest shape: each joint turns about the line across the palm. */
export class Grip {
  private joints: { bone: THREE.Object3D; rest: THREE.Quaternion; axis: THREE.Vector3; finger: string; n: number }[] = [];

  /** Reads the rest shape: call while the model is in its rest pose. `side` is "l" or "r". */
  constructor(model: THREE.Object3D, side: "l" | "r") {
    const bone = (n: string) => model.getObjectByName(`${n}_${side}`);
    const hand = bone("hand"), mid = bone("middle_01"), index = bone("index_01"), pinky = bone("pinky_01");
    if (!hand || !mid || !index || !pinky) return;
    model.updateMatrixWorld(true);
    const p = (o: THREE.Object3D) => o.getWorldPosition(new THREE.Vector3());
    const along = p(mid).sub(p(hand)).normalize();
    const across = p(index).sub(p(pinky)); across.addScaledVector(along, -across.dot(along)).normalize();
    // a right hand's palm faces away from along × across, a left hand's (mirrored) along it
    const palm = new THREE.Vector3().crossVectors(along, across).multiplyScalar(side === "r" ? -1 : 1);
    for (const finger of ["thumb", "index", "middle", "ring", "pinky"]) {
      for (let n = 1; n <= 3; n++) {
        const b = bone(`${finger}_0${n}`), next = bone(`${finger}_0${n + 1}`);
        if (!b) continue;
        const dir = next ? p(next).sub(p(b)).normalize() : p(b).sub(p(b.parent!)).normalize();
        // turning about dir × palm by a positive angle brings the finger towards the palm
        const axis = new THREE.Vector3().crossVectors(dir, palm).normalize();
        axis.applyQuaternion(b.getWorldQuaternion(new THREE.Quaternion()).invert());
        this.joints.push({ bone: b, rest: b.quaternion.clone(), axis, finger, n });
      }
    }
  }

  /** Closes the hand: degrees for each finger's three joints; `w` (0..1) blends from the animated hand. */
  close(curl: Record<string, [number, number, number]>, w: number): void {
    const joints = this.joints.filter(j => curl[j.finger]?.[j.n - 1] !== undefined);
    const input = new Float64Array(joints.length * 13);
    joints.forEach((j, i) => input.set([...j.bone.quaternion.toArray(), ...j.rest.toArray(), ...j.axis.toArray(), curl[j.finger]![j.n - 1]!, w], i * 13));
    const result = motionCompute("grip", input);
    joints.forEach((j, i) => j.bone.quaternion.fromArray(result, i * 4));
  }
}

/** How an in-place walk (or run) loop covers the ground: `speed` is the pace of its planted foot sliding back
 *  under the body, metres per second at normal speed (in the model's own units); `passing` is the moment the
 *  free foot swings past the planted one, a good one to stop on (the free foot then lands in the stance). */
export type Gait = { speed: number; passing: number };

/** Measures a walk loop: plays it on a scratch mixer and follows whichever foot is lower. Call at rest. */
export function gaitOf(model: THREE.Object3D, clip: THREE.AnimationClip): Gait | null {
  const l = model.getObjectByName("foot_l") ?? model.getObjectByName("mixamorigLeftFoot");
  const r = model.getObjectByName("foot_r") ?? model.getObjectByName("mixamorigRightFoot");
  if (!l || !r) return null;
  const bones: [THREE.Object3D, THREE.Vector3, THREE.Quaternion, THREE.Vector3][] = [];
  model.traverse((o) => { if ((o as THREE.Bone).isBone) bones.push([o, o.position.clone(), o.quaternion.clone(), o.scale.clone()]); });
  const mixer = new THREE.AnimationMixer(model), act = mixer.clipAction(clip).play();
  const N = 96, dt = clip.duration / N, at = (o: THREE.Object3D) => model.worldToLocal(o.getWorldPosition(new THREE.Vector3()));
  const pts: { l: THREE.Vector3; r: THREE.Vector3 }[] = [];
  for (let i = 0; i < N; i++) {
    mixer.setTime(i * dt);
    model.updateMatrixWorld(true);
    pts.push({ l: at(l), r: at(r) });
  }
  act.stop(); mixer.stopAllAction(); mixer.uncacheRoot(model);
  for (const [o, p, q, s] of bones) { o.position.copy(p); o.quaternion.copy(q); o.scale.copy(s); }
  model.updateMatrixWorld(true);
  // the planted foot is the lower one; how fast it slides back (the figure faces +z) is how fast the body walks
  const [speed = 0, passing = 0] = motionCompute("gait", new Float64Array([dt, ...pts.flatMap(p => [...p.l.toArray(), ...p.r.toArray()])]));
  return speed > 0.1 ? { speed, passing } : null;
}

type Footprint = { at: THREE.Vector3; turn: THREE.Quaternion; toe: THREE.Vector3 };
type Leg = { thigh: THREE.Object3D; calf: THREE.Object3D; foot: THREE.Object3D; ball: THREE.Object3D };

/** While a figure stands in a stance, its feet stay flat on the floor where the stance puts them on
 *  average: the video mocap leaves them on tiptoe, rolled and sliding. Footprints are in model space. */
export class Feet {
  private legs: Record<"l" | "r", Leg> | null = null;
  private prints = new Map<string, Record<"l" | "r", Footprint>>();
  private w = 0;
  private last: Record<"l" | "r", Footprint> | null = null;
  private model: THREE.Object3D;

  /** Reads the rest pose and averages each stance clip's footprint: call while the model is at rest. */
  constructor(model: THREE.Object3D, clips: THREE.AnimationClip[], stances: string[]) {
    this.model = model;
    const get = (n: string) => model.getObjectByName(n);
    const leg = (s: string): Leg | null => {
      const thigh = get(`thigh_${s}`), calf = get(`calf_${s}`), foot = get(`foot_${s}`), ball = get(`ball_${s}`);
      return thigh && calf && foot && ball ? { thigh, calf, foot, ball } : null;
    };
    const l = leg("l"), r = leg("r");
    if (!l || !r) return;
    this.legs = { l, r };
    // the rest pose: feet flat, the ankle at its standing height
    const local = (o: THREE.Object3D) => model.worldToLocal(o.getWorldPosition(new THREE.Vector3()));
    model.updateMatrixWorld(true);
    const rest = { l: { ankle: local(l.foot), toe: local(l.ball) }, r: { ankle: local(r.foot), toe: local(r.ball) } };
    const restTurn = { l: this.modelQuat(l.foot), r: this.modelQuat(r.foot) };
    // play each stance on a scratch mixer and average where its feet stand and which way they point
    const bones: [THREE.Object3D, THREE.Vector3, THREE.Quaternion, THREE.Vector3][] = [];
    model.traverse((o) => { if ((o as THREE.Bone).isBone) bones.push([o, o.position.clone(), o.quaternion.clone(), o.scale.clone()]); });
    const mixer = new THREE.AnimationMixer(model);
    for (const name of stances) {
      const clip = clips.find((c) => c.name === name);
      if (!clip) continue;
      const act = mixer.clipAction(clip).play();
      const sum = { l: { at: new THREE.Vector3(), dir: new THREE.Vector3() }, r: { at: new THREE.Vector3(), dir: new THREE.Vector3() } };
      const N = 12;
      for (let i = 0; i < N; i++) {
        mixer.setTime((clip.duration * i) / N);
        model.updateMatrixWorld(true);
        for (const s of ["l", "r"] as const) {
          const a = local(this.legs[s].foot), t = local(this.legs[s].ball);
          sum[s].at.add(a);
          sum[s].dir.add(t.sub(a).setY(0).normalize());
        }
      }
      act.stop();
      const print = {} as Record<"l" | "r", Footprint>;
      for (const s of ["l", "r"] as const) {
        const p = motionCompute("footprint", new Float64Array([
          ...sum[s].at.divideScalar(N).toArray(), ...sum[s].dir.toArray(), ...rest[s].ankle.toArray(), ...rest[s].toe.toArray(), ...restTurn[s].toArray(),
        ]));
        print[s] = { at: new THREE.Vector3().fromArray(p), turn: new THREE.Quaternion().fromArray(p, 3), toe: new THREE.Vector3().fromArray(p, 7) };
      }
      this.prints.set(name, print);
    }
    mixer.stopAllAction(); mixer.uncacheRoot(model);
    for (const [o, p, q, s] of bones) { o.position.copy(p); o.quaternion.copy(q); o.scale.copy(s); }
    model.updateMatrixWorld(true);
  }

  /** After the mixer: plants the feet of stance `name` (null: let the animation have them), easing in and out. */
  update(dt: number, name: string | null): void {
    const legs = this.legs, print = name ? this.prints.get(name) : undefined;
    this.w += ((print ? 1 : 0) - this.w) * (1 - Math.exp(-dt * 8));
    if (!legs || this.w < 0.01) return;
    const p = print ?? this.last;
    if (!p) return;
    this.last = p;
    const model = this.model;
    model.updateMatrixWorld(true);
    const mq = model.getWorldQuaternion(new THREE.Quaternion());
    for (const s of ["l", "r"] as const) {
      const { thigh, calf, foot } = legs[s];
      const pose = motionCompute("feet", new Float64Array([
        this.w, ...model.localToWorld(vc.copy(p[s].at)).toArray(), ...foot.getWorldPosition(va).toArray(), ...p[s].toe.toArray(),
        ...thigh.getWorldPosition(vb).toArray(), ...mq.toArray(), ...p[s].turn.toArray(), ...foot.getWorldQuaternion(qb).toArray(),
      ]));
      reach(thigh, calf, foot, new THREE.Vector3().fromArray(pose), new THREE.Vector3().fromArray(pose, 3));
      const turn = motionCompute("worldBlend", new Float64Array([...foot.getWorldQuaternion(qb).toArray(), ...pose.slice(6, 10), this.w]));
      setWorldQuat(foot, qa.fromArray(turn));
    }
  }
  private modelQuat(o: THREE.Object3D): THREE.Quaternion {
    return this.model.getWorldQuaternion(new THREE.Quaternion()).invert().multiply(o.getWorldQuaternion(new THREE.Quaternion()));
  }
}
