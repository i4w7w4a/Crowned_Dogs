// First person: our arms hold the gun the way a bodycam sees it (Art's references), low in the middle of
// the picture and aimed at the enemy, whatever the third-person clip does with them. The gun is placed
// first, in the camera's frame; the arms then reach for it and the fingers close on it.

import * as THREE from "three";
import { Grip, reach, setWorldQuat } from "./ik.ts";
import type { Stance } from "./motion.ts";
import { gripOf, type Hand } from "../../../apps/web/src/arena/weapons.ts";

type V3 = [number, number, number];
type Curl = Record<string, [number, number, number]>;

/** The left hand on the gun, in the gun's frame (x forward along the barrel, y up, z right, the grip at the
 *  origin): where its palm is, which way its fingers run, and which way its index finger's side faces. */
interface Support { at: V3; along: V3; across: V3 }

interface Hold {
  /** The grip, in the camera's frame: metres to the right, down, forward. */
  at: V3;
  /** The grip while it aims, raised towards the eye (kept a little low and right, so the gun never hides the enemy). */
  aim: V3;
  /** The shot's kick: back (m), up (m), and the muzzle's climb (radians). */
  kick: V3;
  support: Support;
  right: Curl;
  left: Curl;
}

// the trigger finger lies straight along the frame, the thumb and the rest close round the grip
const TRIGGER: Curl = { thumb: [15, 35, 25], index: [5, 10, 5], middle: [75, 85, 45], ring: [78, 85, 45], pinky: [80, 85, 45] };
/** How far the grip sits in from the hand's middle line towards the palm, metres: the clips carry the gun
 *  through the back of the hand, so in close-up the fingers closed on nothing beside it. */
const PALM = 0.025;
// a long gun's fore-end lies across the left palm, the fingers wrap it
const FOREEND: Support = { at: [0.2, 0.0, -0.005], along: [0.35, 0.35, 0.87], across: [1, 0, 0] };
const CUP: Curl = { thumb: [10, 20, 10], index: [45, 55, 35], middle: [50, 60, 35], ring: [52, 60, 35], pinky: [55, 60, 35] };

const HOLD: Record<Stance, Hold> = {
  // two hands: the left wraps the right on the grip
  pistol: {
    at: [0.0, 0.23, 0.4], aim: [0.02, 0.15, 0.47], kick: [0.035, 0.02, 0.3],
    support: { at: [-0.01, -0.03, -0.035], along: [0.55, -0.45, 0.7], across: [0.5, 0.85, 0] },
    // both thumbs lie forward along the frame, one over the other
    right: { ...TRIGGER, thumb: [-25, -10, 0] }, left: { thumb: [-25, -10, 0], index: [65, 75, 40], middle: [68, 78, 40], ring: [70, 80, 40], pinky: [72, 80, 40] },
  },
  // the stock at the right shoulder, the barrel running up the middle of the picture
  // aiming, the stock comes up into the shoulder and the barrel up the middle towards the eye line
  rifle: { at: [0.07, 0.33, 0.22], aim: [0.07, 0.27, 0.25], kick: [0.05, 0.012, 0.1], support: FOREEND, right: TRIGGER, left: CUP },
  shotgun: { at: [0.07, 0.33, 0.2], aim: [0.07, 0.275, 0.23], kick: [0.09, 0.025, 0.2], support: { ...FOREEND, at: [0.24, 0.0, -0.005] }, right: TRIGGER, left: CUP },
  sniper: { at: [0.07, 0.33, 0.22], aim: [0.07, 0.265, 0.25], kick: [0.08, 0.02, 0.14], support: FOREEND, right: TRIGGER, left: CUP },
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
  private aiming = 0;
  /** Seconds since the shot, -1 while an attack is still coming up to it. */
  private since = -1;
  private was = false;
  private kick = 0;
  private t = 0;
  private at = new THREE.Vector3();
  private placed = false;
  private hand: Hand;
  /** Where the clips have the gun in the hand. */
  private base: THREE.Vector3;

  /** Reads the hands' rest shape: call while the model is in its rest pose. */
  constructor(model: THREE.Object3D, hand: Hand) {
    this.hand = hand;
    this.base = hand.mount.position.clone();
    this.right = new Grip(model, "r");
    this.left = new Grip(model, "l");
    this.palm = gripOf(model, "l");
    const b = (n: string) => model.getObjectByName(n);
    const side = (s: string) => [b(`upperarm_${s}`), b(`lowerarm_${s}`), b(`hand_${s}`)] as const;
    const l = side("l"), r = side("r");
    if (l.every((o) => o) && r.every((o) => o)) this.arm = { l: l as unknown as [THREE.Object3D, THREE.Object3D, THREE.Object3D], r: r as unknown as [THREE.Object3D, THREE.Object3D, THREE.Object3D] };
  }

  /** The gun fires: it kicks back and up, and settles. */
  shot(): void { this.kick = 1; this.since = 0; }

  /** After the animation: `on` holds the gun of `stance` in view, aimed at `aim` (world), raised to the eye
   *  while `aiming` (an attack is under way); off hands the arms back to the clip, easing either way. */
  update(dt: number, on: boolean, aiming: boolean, stance: Stance, camera: THREE.Camera, aim: THREE.Vector3): void {
    this.w += ((on ? 1 : 0) - this.w) * (1 - Math.exp(-dt * 10));
    // up quickly (the shot comes a third of a second after the attack starts), held through the kick, then down at ease
    if (aiming && !this.was) this.since = -1;
    this.was = aiming;
    if (this.since >= 0) this.since += dt;
    const raise = aiming && this.since < 0.45;
    this.aiming += ((raise ? 1 : 0) - this.aiming) * (1 - Math.exp(-dt * (raise ? 13 : 5)));
    this.kick *= Math.exp(-dt * 11);
    this.t += dt;
    // the grip comes into the palm
    const mount = this.hand.mount;
    mount.position.copy(this.base).add(tmp.s.set(0, 0, -PALM * this.w).multiply(mount.scale).applyQuaternion(mount.quaternion));
    if (!this.arm || this.w < 0.01) { this.placed = false; return; }
    const h = HOLD[stance], w = this.w, k = this.kick, [back, up, climb] = h.kick;
    // the grip glides to the stance's spot when the gun changes, and breathes a little (less while it aims)
    const a = this.aiming * this.aiming * (3 - 2 * this.aiming);
    tmp.t.fromArray(h.at).lerp(tmp.s.fromArray(h.aim), a);
    if (this.placed) this.at.lerp(tmp.t, 1 - Math.exp(-dt * 6)); else this.at.copy(tmp.t);
    this.placed = true;
    const sway = 0.004 * (1 - 0.6 * a);
    camera.updateMatrixWorld();
    const P = camera.localToWorld(tmp.p.set(
      this.at.x + sway * Math.sin(this.t * 1.3), -this.at.y + sway * Math.sin(this.t * 1.7 + 1) + up * k, -this.at.z + back * k));
    // aimed at the enemy, the muzzle kicked up by the shot
    const f = tmp.f.subVectors(aim, P).normalize();
    const r = tmp.r.crossVectors(f, tmp.u.set(0, 1, 0)).normalize();
    f.applyAxisAngle(r, climb * k);
    const u = tmp.u.crossVectors(r, f);
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

  /** The third-person camera: the gun back where the clips hold it. */
  off(): void { this.w = 0; this.placed = false; this.hand.mount.position.copy(this.base); }

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