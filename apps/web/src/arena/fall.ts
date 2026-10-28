// How a Persona drops when it dies: the knees give, the body sinks onto them and slumps over on its side,
// right where it stood. The pelvis is moved by hand and the legs reach back down to the floor where the feet
// were, so nothing slides or flies: the feet stay put and the body folds above them.

import * as THREE from "three";
import { reach, setWorldQuat, turn as aim } from "./ik.ts";

/** Seconds from the last hit to lying still. */
export const FALL_TIME = 1.25;

const va = new THREE.Vector3(), vb = new THREE.Vector3(), qa = new THREE.Quaternion(), qb = new THREE.Quaternion(), qc = new THREE.Quaternion(), qd = new THREE.Quaternion();
const ve = new THREE.Vector3(), vf = new THREE.Vector3(), vg = new THREE.Vector3(), m4 = new THREE.Matrix4();
const FWD = new THREE.Vector3(0, 0, 1), SIDE = new THREE.Vector3(1, 0, 0), DOWN = new THREE.Vector3(0, -1, 0), UP = new THREE.Vector3(0, 1, 0);

type Leg = { thigh: THREE.Object3D; calf: THREE.Object3D; foot: THREE.Object3D; at: THREE.Vector3; turn: THREE.Quaternion };
/** A bone that must stay above the floor, and how far its flesh (and armour) reaches past the joint. */
type Solid = { bone: THREE.Object3D; r: number };

/** Turns a bone about a world axis. */
const spin = (o: THREE.Object3D, axis: THREE.Vector3, a: number) => setWorldQuat(o, qc.setFromAxisAngle(axis, a).multiply(o.getWorldQuaternion(qd)));
const ease = (u: number) => u * u * (3 - 2 * u);
const clamp01 = (u: number) => Math.min(1, Math.max(0, u));

export class Collapse {
  private pelvis: THREE.Object3D | null;
  private spine: THREE.Object3D[];
  private head: THREE.Object3D | null;
  private legs: Leg[] = [];
  /** The gun's frame in the right hand (weapons.ts Hand.mount: x along the barrel, y up), if it holds one. */
  gun: THREE.Object3D | null = null;
  /** The arm on the figure's left, then on its right. */
  private arms: { upper: THREE.Object3D; lower: THREE.Object3D; hand: THREE.Object3D }[] = [];
  private solids: Solid[] = [];
  /** Where the pelvis stood (world), its turn, the figure's front and side (world, unit) and which way it falls. */
  private from = { at: new THREE.Vector3(), turn: new THREE.Quaternion(), fwd: new THREE.Vector3(), side: new THREE.Vector3(), dir: 1 };

  constructor(model: THREE.Object3D) {
    const get = (...n: string[]) => n.map((x) => model.getObjectByName(x)).find((o) => o) ?? null;
    this.pelvis = get("pelvis", "mixamorigHips");
    this.spine = [get("spine_01", "mixamorigSpine"), get("spine_02", "mixamorigSpine1"), get("spine_03", "mixamorigSpine2")].filter((o): o is THREE.Object3D => !!o);
    this.head = get("head", "mixamorigHead");
    for (const [s, m] of [["l", "Left"], ["r", "Right"]] as const) {
      const thigh = get(`thigh_${s}`, `mixamorig${m}UpLeg`), calf = get(`calf_${s}`, `mixamorig${m}Leg`), foot = get(`foot_${s}`, `mixamorig${m}Foot`);
      if (thigh && calf && foot) this.legs.push({ thigh, calf, foot, at: new THREE.Vector3(), turn: new THREE.Quaternion() });
      const upper = get(`upperarm_${s}`, `mixamorig${m}Arm`), lower = get(`lowerarm_${s}`, `mixamorig${m}ForeArm`), hand = get(`hand_${s}`, `mixamorig${m}Hand`);
      if (upper && lower && hand) this.arms.push({ upper, lower, hand });
    }
    // the upper body, which must lie on the floor and not sink into it
    this.spine[0]?.traverse((o) => {
      if (!(o as THREE.Bone).isBone) return;
      const n = o.name.toLowerCase();
      if (/spine|clavicle|neck|head|upperarm|arm$/.test(n)) this.solids.push({ bone: o, r: /head/.test(n) ? 0.1 : 0.12 });
      else if (/lowerarm|forearm|hand$/.test(n)) this.solids.push({ bone: o, r: 0.05 });
    });
  }

  /** Starts the fall from the pose the figure has now. `side` 1 falls towards the figure's left (+x of its group). */
  begin(group: THREE.Object3D, side: number): void {
    const p = this.pelvis;
    if (!p) return;
    group.updateMatrixWorld(true);
    const g = group.getWorldQuaternion(qa);
    this.from.at.copy(p.getWorldPosition(va));
    this.from.turn.copy(p.getWorldQuaternion(qb));
    this.from.fwd.copy(FWD).applyQuaternion(g);
    this.from.side.copy(SIDE).applyQuaternion(g);
    this.from.dir = side;
    for (const l of this.legs) { l.foot.getWorldPosition(l.at); l.foot.getWorldQuaternion(l.turn); }
  }

  /** Poses the body `t` seconds into the fall, over whatever the animation did this frame. */
  apply(t: number): void {
    const p = this.pelvis, f = this.from;
    if (!p || !p.parent) return;
    // the knees give at once; the slump onto the side starts while they are still bending and speeds up as a
    // falling body does, then the body settles with a small rebound where it hits the floor
    const knees = ease(clamp01(t / 0.42));
    const u = clamp01((t - 0.28) / 0.72), over = u * u;
    const land = t > 1 ? Math.exp(-(t - 1) * 10) * Math.sin((t - 1) * 30) * 0.06 : 0;
    const h0 = f.at.y;
    const y = THREE.MathUtils.lerp(h0, h0 * 0.62, knees) + (0.14 - h0 * 0.62) * over + land * 0.4;
    const along = 0.06 * knees + 0.06 * over, across = 0.34 * over;
    const target = va.copy(f.at).setY(y).addScaledVector(f.fwd, along).addScaledVector(f.side, across * f.dir);
    // bent at the hips as the knees give, then rolled over onto the side
    const roll = (1.42 + land) * over, bend = 0.18 * knees - 0.1 * over;
    const turn = qa.setFromAxisAngle(f.fwd, -roll * f.dir).multiply(qb.setFromAxisAngle(f.side, bend)).multiply(f.turn);
    this.place(target, turn);
    // the spine slumps forward, then lies out along the floor; the head drops and rolls onto the floor last
    const slump = 0.1 * knees * (1 - 0.6 * over);
    for (const s of this.spine) spin(s, f.side, slump);
    if (this.head) { spin(this.head, f.side, 0.2 * knees); spin(this.head, f.fwd, 0.9 * over * f.dir); }
    // the arm it falls on is flung out ahead along the floor, not left under the shoulder to prop the body up
    const arm = this.arms.length === 2 ? this.arms[f.dir > 0 ? 0 : 1] : undefined;
    if (arm && over > 0) {
      const shoulder = arm.upper.getWorldPosition(ve), now = arm.lower.getWorldPosition(vf).sub(shoulder).normalize();
      const out = vg.copy(f.fwd).multiplyScalar(0.85).addScaledVector(f.side, 0.45 * f.dir).addScaledVector(DOWN, 0.1).normalize();
      aim(arm.upper, now, vg.copy(out).lerp(now, 1 - over).normalize());
      // and the forearm lies on along the floor, the hand a little ahead of the body
      const elbow = arm.lower.getWorldPosition(ve), fore = arm.hand.getWorldPosition(vf).sub(elbow).normalize();
      aim(arm.lower, fore, out.addScaledVector(DOWN, 0.15).addScaledVector(f.side, -0.3 * f.dir).normalize().lerp(fore, 1 - over).normalize());
    }
    p.updateMatrixWorld(true);
    // the hand goes slack and the gun ends up lying on its side on the floor, along the body
    const gun = this.gun, hand = gun?.parent;
    if (gun && hand && over > 0) {
      const along = ve.copy(f.fwd).multiplyScalar(0.5).addScaledVector(f.side, 0.85 * f.dir).normalize();
      const flat = qb.setFromRotationMatrix(m4.makeBasis(along, vf.crossVectors(UP, along), UP));
      const now = gun.getWorldQuaternion(qd).slerp(flat, over);
      setWorldQuat(hand, now.multiply(qc.copy(gun.quaternion).invert()));
    }
    // whatever still reaches through the floor lifts the body out of it
    let sink = 0;
    for (const s of this.solids) sink = Math.max(sink, s.r - s.bone.getWorldPosition(vb).y);
    if (sink > 0) { target.y += sink; this.place(target, turn); }
    this.plant(over);
  }

  /** Puts the pelvis at a world position and turn. */
  private place(at: THREE.Vector3, turn: THREE.Quaternion): void {
    const p = this.pelvis!;
    p.position.copy(p.parent!.worldToLocal(vb.copy(at)));
    setWorldQuat(p, turn);
  }

  /** The feet stay where they stood; the knees point ahead of the body; the feet roll a little with it. */
  private plant(over: number): void {
    const f = this.from;
    for (const l of this.legs) {
      const hip = l.thigh.getWorldPosition(vb);
      const pole = hip.lerp(l.at, 0.5).addScaledVector(f.fwd, 0.7);
      reach(l.thigh, l.calf, l.foot, l.at.clone(), pole.clone());
      setWorldQuat(l.foot, qb.setFromAxisAngle(f.fwd, -0.9 * over * f.dir).multiply(l.turn));
    }
  }
}
