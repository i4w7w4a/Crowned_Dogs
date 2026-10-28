// First person: our arms hold the gun the way a bodycam sees it (Art's references), low in the middle of
// the picture and aimed at the enemy, whatever the third-person clip does with them. The gun is placed
// first, in the camera's frame; the arms then reach for it and the fingers close on it.

import * as THREE from "three";
import { Grip, reach, setWorldQuat } from "./ik.ts";
import type { Stance } from "./motion.ts";
import { gripOf, type Hand } from "./weapons.ts";

type V3 = [number, number, number];
type Curl = Record<string, [number, number, number]>;

/** The left hand on the gun, in the gun's frame (x forward along the barrel, y up, z right, the grip at the
 *  origin): where its palm is, which way its fingers run, and which way its index finger's side faces. */
interface Support { at: V3; along: V3; across: V3 }

interface Hold {
  /** The grip, in the camera's frame: metres to the right, down, forward. */
  at: V3;
  support: Support;
  right: Curl;
  left: Curl;
}

// the trigger finger rests along the frame, the rest close round the grip
const TRIGGER: Curl = { thumb: [5, 25, 20], index: [20, 30, 15], middle: [75, 85, 45], ring: [78, 85, 45], pinky: [80, 85, 45] };
// a long gun's fore-end lies across the left palm, the fingers wrap it
const FOREEND: Support = { at: [0.2, 0.0, -0.005], along: [0.35, 0.35, 0.87], across: [1, 0, 0] };
const CUP: Curl = { thumb: [10, 20, 10], index: [45, 55, 35], middle: [50, 60, 35], ring: [52, 60, 35], pinky: [55, 60, 35] };

const HOLD: Record<Stance, Hold> = {
  // two hands: the left wraps the right on the grip
  pistol: {
    at: [0.0, 0.23, 0.4],
    support: { at: [-0.01, -0.03, -0.035], along: [0.55, -0.45, 0.7], across: [0.5, 0.85, 0] },
    right: TRIGGER, left: { thumb: [10, 20, 10], index: [65, 75, 40], middle: [68, 78, 40], ring: [70, 80, 40], pinky: [72, 80, 40] },
  },
  // the stock at the right shoulder, the barrel running up the middle of the picture
  rifle: { at: [0.07, 0.33, 0.22], support: FOREEND, right: TRIGGER, left: CUP },
  shotgun: { at: [0.07, 0.33, 0.2], support: { ...FOREEND, at: [0.24, 0.0, -0.005] }, right: TRIGGER, left: CUP },
  sniper: { at: [0.07, 0.33, 0.22], support: FOREEND, right: TRIGGER, left: CUP },
};

const v = () => new THREE.Vector3();
const tmp = { p: v(), f: v(), u: v(), r: v(), s: v(), t: v(), q: new THREE.Quaternion(), q2: new THREE.Quaternion(), m: new THREE.Matrix4(), m2: new THREE.Matrix4() };

export class Viewmodel {
  private right: Grip;
  private left: Grip;
  /** The left palm's frame in the left hand bone's space. */
  private palm: THREE.Matrix4 | null;
  private arm: Record<"l" | "r", [THREE.Object3D, THREE.Object3D, THREE.Object3D]> | null = null;
  private w = 0;
  private kick = 0;
  private t = 0;
  private at = new THREE.Vector3();
  private placed = false;
  private hand: Hand;

  /** Reads the hands' rest shape: call while the model is in its rest pose. */
  constructor(model: THREE.Object3D, hand: Hand) {
    this.hand = hand;
    this.right = new Grip(model, "r");
    this.left = new Grip(model, "l");
    this.palm = gripOf(model, "l");
    const b = (n: string) => model.getObjectByName(n);
    const side = (s: string) => [b(`upperarm_${s}`), b(`lowerarm_${s}`), b(`hand_${s}`)] as const;
    const l = side("l"), r = side("r");
    if (l.every((o) => o) && r.every((o) => o)) this.arm = { l: l as unknown as [THREE.Object3D, THREE.Object3D, THREE.Object3D], r: r as unknown as [THREE.Object3D, THREE.Object3D, THREE.Object3D] };
  }

  /** The gun fires: it kicks back and up, and settles. */
  shot(): void { this.kick = 1; }

  /** After the animation: `on` holds the gun of `stance` in view, aimed at `aim` (world); off hands the arms
   *  back to the clip, easing either way. */
  update(dt: number, on: boolean, stance: Stance, camera: THREE.Camera, aim: THREE.Vector3): void {
    this.w += ((on ? 1 : 0) - this.w) * (1 - Math.exp(-dt * 10));
    this.kick *= Math.exp(-dt * 9);
    this.t += dt;
    if (!this.arm || this.w < 0.01) { this.placed = false; return; }
    const h = HOLD[stance], w = this.w, k = this.kick;
    // the grip glides to the stance's spot when the gun changes, and breathes a little
    tmp.t.fromArray(h.at);
    if (this.placed) this.at.lerp(tmp.t, 1 - Math.exp(-dt * 6)); else this.at.copy(tmp.t);
    this.placed = true;
    const sway = 0.004;
    camera.updateMatrixWorld();
    const P = camera.localToWorld(tmp.p.set(
      this.at.x + sway * Math.sin(this.t * 1.3), -this.at.y + sway * Math.sin(this.t * 1.7 + 1) + 0.012 * k, -this.at.z + 0.05 * k));
    // aimed at the enemy, kicked up by the shot
    const f = tmp.f.subVectors(aim, P).normalize();
    const r = tmp.r.crossVectors(f, tmp.u.set(0, 1, 0)).normalize();
    f.applyAxisAngle(r, 0.12 * k);
    const u = tmp.u.crossVectors(r, f);
    const mount = this.hand.mount;
    tmp.m.makeBasis(f, u, r).setPosition(P).scale(mount.getWorldScale(tmp.s));
    // the right hand where it holds the gun so: hand = gun · mount⁻¹
    mount.updateMatrix();
    this.place("r", tmp.m2.copy(tmp.m).multiply(new THREE.Matrix4().copy(mount.matrix).invert()), camera, w);
    // the left hand on the gun's support: hand = gun · support · palm⁻¹
    if (this.palm) {
      const s = h.support, along = new THREE.Vector3(...s.along).normalize(), across = new THREE.Vector3(...s.across);
      across.addScaledVector(along, -across.dot(along)).normalize();
      const support = new THREE.Matrix4().makeBasis(along, across, new THREE.Vector3().crossVectors(along, across)).setPosition(...s.at);
      this.place("l", tmp.m2.copy(tmp.m).multiply(support).multiply(this.palm.clone().invert()), camera, w);
    }
    this.right.close(h.right, w);
    this.left.close(h.left, w);
  }

  /** Reaches one arm so that its hand bone takes the world matrix `to` (blended by `w`). */
  private place(side: "l" | "r", to: THREE.Matrix4, camera: THREE.Camera, w: number): void {
    const [upper, lower, hand] = this.arm![side];
    const target = new THREE.Vector3(), q = new THREE.Quaternion(), s = new THREE.Vector3();
    to.decompose(target, q, s);
    const now = hand.getWorldPosition(new THREE.Vector3());
    target.lerp(now, 1 - w);
    // the elbow hangs down and out
    const out = side === "r" ? 1 : -1;
    const pole = camera.localToWorld(new THREE.Vector3(0.3 * out, -0.8, 0.05)).sub(camera.getWorldPosition(new THREE.Vector3())).add(target);
    reach(upper, lower, hand, target, pole);
    setWorldQuat(hand, hand.getWorldQuaternion(new THREE.Quaternion()).slerp(q, w));
  }
}
