// Bones placed by hand after the animation: a two-bone reach for an arm or a leg, feet planted flat on
// the floor while a figure stands, and fingers closed on a grip.

import * as THREE from "three";

const qa = new THREE.Quaternion(), qb = new THREE.Quaternion(), qc = new THREE.Quaternion();
const va = new THREE.Vector3(), vb = new THREE.Vector3(), vc = new THREE.Vector3(), vd = new THREE.Vector3(), ve = new THREE.Vector3();
const UP = new THREE.Vector3(0, 1, 0);

/** Sets a bone's world rotation. */
export function setWorldQuat(o: THREE.Object3D, q: THREE.Quaternion): void {
  o.parent!.getWorldQuaternion(qc);
  o.quaternion.copy(qc.invert().multiply(q));
  o.updateMatrixWorld(true);
}

/** Turns a bone so that the world direction `from` (unit) it carries points along `to` (unit). */
export function turn(o: THREE.Object3D, from: THREE.Vector3, to: THREE.Vector3): void {
  qa.setFromUnitVectors(from, to).multiply(o.getWorldQuaternion(qb));
  setWorldQuat(o, qa);
}

/** Bends a limb (upper `a`, lower `b`, end `c`) so that `c` lands on `target`, the middle joint towards `pole`.
 *  A target out of reach is met with the limb straight. */
export function reach(a: THREE.Object3D, b: THREE.Object3D, c: THREE.Object3D, target: THREE.Vector3, pole: THREE.Vector3): void {
  const A = a.getWorldPosition(va), B = b.getWorldPosition(vb), C = c.getWorldPosition(vc);
  const l1 = A.distanceTo(B), l2 = B.distanceTo(C);
  const n = vd.subVectors(target, A), len = n.length();
  if (len < 1e-5) return;
  n.divideScalar(len);
  const d = THREE.MathUtils.clamp(len, Math.abs(l1 - l2) + 1e-4, l1 + l2 - 1e-4);
  // the bend: towards the pole, across the line to the target
  const m = ve.subVectors(pole, A); m.addScaledVector(n, -m.dot(n));
  if (m.lengthSq() < 1e-10) return;
  m.normalize();
  const cos = (l1 * l1 + d * d - l2 * l2) / (2 * l1 * d), sin = Math.sqrt(Math.max(0, 1 - cos * cos));
  // the middle joint goes to A + n·l1·cos + m·l1·sin
  const from = B.sub(A).normalize();
  const to = m.multiplyScalar(l1 * sin).addScaledVector(n, l1 * cos).normalize();
  turn(a, from, to);
  const B2 = b.getWorldPosition(vb), C2 = c.getWorldPosition(vc);
  const end = va.copy(A).addScaledVector(n, d); // A is va: the reachable target
  turn(b, C2.sub(B2).normalize(), end.sub(B2).normalize());
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
    for (const j of this.joints) {
      const deg = curl[j.finger]?.[j.n - 1];
      if (deg === undefined) continue;
      qa.setFromAxisAngle(j.axis, THREE.MathUtils.degToRad(deg));
      qb.copy(j.rest).multiply(qa);
      j.bone.quaternion.slerp(qb, w);
    }
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
  const low = Math.min(...pts.map((p) => Math.min(p.l.y, p.r.y))), v: number[] = [];
  let passing = 0, gap = Infinity;
  for (let i = 0; i < N; i++) {
    const a = pts[i]!, b = pts[(i + 1) % N]!, s = a.l.y < a.r.y ? "l" : "r";
    if (a[s].y < low + 0.03) v.push((a[s].z - b[s].z) / dt);
    const g = Math.abs(a.l.z - a.r.z);
    if (g < gap) { gap = g; passing = i * dt; }
  }
  v.sort((x, y) => x - y);
  const speed = v[v.length >> 1] ?? 0;
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
        const at = sum[s].at.divideScalar(N).setY(rest[s].ankle.y);
        const restDir = vd.copy(rest[s].toe).sub(rest[s].ankle).setY(0).normalize();
        // a foot turned out more than 40° from the body's front looks twisted: no further
        const yawTo = THREE.MathUtils.clamp(Math.atan2(sum[s].dir.x, sum[s].dir.z), -0.7, 0.7);
        const dir = new THREE.Vector3(Math.sin(yawTo), 0, Math.cos(yawTo));
        // the same flat foot as at rest, turned about the vertical to the stance's average heading
        const yaw = new THREE.Quaternion().setFromUnitVectors(restDir, dir);
        print[s] = { at, turn: yaw.multiply(restTurn[s]), toe: dir.clone() };
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
      const target = model.localToWorld(vc.copy(p[s].at));
      const now = foot.getWorldPosition(new THREE.Vector3());
      target.lerp(now, 1 - this.w);
      // the knee points the way the foot does, a little outwards
      const toe = vb.copy(p[s].toe).applyQuaternion(mq);
      const hip = thigh.getWorldPosition(new THREE.Vector3());
      const pole = hip.lerp(target, 0.5).addScaledVector(toe, 0.6).addScaledVector(UP, 0.05);
      reach(thigh, calf, foot, target.clone(), pole);
      const flat = qa.copy(mq).multiply(p[s].turn);
      setWorldQuat(foot, foot.getWorldQuaternion(qb).slerp(flat, this.w));
    }
  }
  private modelQuat(o: THREE.Object3D): THREE.Quaternion {
    return this.model.getWorldQuaternion(new THREE.Quaternion()).invert().multiply(o.getWorldQuaternion(new THREE.Quaternion()));
  }
}
