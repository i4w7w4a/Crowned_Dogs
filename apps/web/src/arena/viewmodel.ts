// First person: our arms hold the gun the way a bodycam sees it (Art's references), low in the middle of
// the picture and aimed at the enemy, whatever the third-person clip does with them. The gun is placed
// first, in the camera's frame; the arms then reach for it and the fingers close on it.

import * as THREE from "three";
import { motionCompute } from "../../../../packages/runtime/src/index.ts";
import { Grip, reach, setWorldQuat } from "./ik.ts";
import type { Stance } from "./motion.ts";
import { gripOf, type Hand } from "./weapons.ts";

type Curl = Record<string, [number, number, number]>;
interface Hold { right: Curl; left: Curl }

// the trigger finger lies straight along the frame, the thumb and the rest close round the grip
const TRIGGER: Curl = { thumb: [15, 35, 25], index: [5, 10, 5], middle: [75, 85, 45], ring: [78, 85, 45], pinky: [80, 85, 45] };
/** How far the grip sits in from the hand's middle line towards the palm, metres: the clips carry the gun
 *  through the back of the hand, so in close-up the fingers closed on nothing beside it. */
const PALM = 0.025;
// a long gun's fore-end lies across the left palm, the fingers wrap it
const CUP: Curl = { thumb: [10, 20, 10], index: [45, 55, 35], middle: [50, 60, 35], ring: [52, 60, 35], pinky: [55, 60, 35] };

const HOLD: Record<Stance, Hold> = {
  // two hands: the left wraps the right on the grip
  pistol: {
    // both thumbs lie forward along the frame, one over the other
    right: { ...TRIGGER, thumb: [-25, -10, 0] }, left: { thumb: [-25, -10, 0], index: [65, 75, 40], middle: [68, 78, 40], ring: [70, 80, 40], pinky: [72, 80, 40] },
  },
  // the stock at the right shoulder, the barrel running up the middle of the picture
  // aiming, the stock comes up into the shoulder and the barrel up the middle towards the eye line
  rifle: { right: TRIGGER, left: CUP },
  shotgun: { right: TRIGGER, left: CUP },
  sniper: { right: TRIGGER, left: CUP },
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
    const stanceIndex = ["pistol", "rifle", "shotgun", "sniper"].indexOf(stance);
    const frame = motionCompute("viewmodel", new Float64Array([
      dt, +on, +aiming, stanceIndex, this.w, this.aiming, this.since, +this.was, this.kick, this.t, ...this.at.toArray(), +this.placed,
    ]));
    this.w = frame[0]!; this.aiming = frame[1]!; this.since = frame[2]!; this.was = !!frame[3];
    this.kick = frame[4]!; this.t = frame[5]!; this.at.fromArray(frame, 6); this.placed = !!frame[9];
    const mount = this.hand.mount;
    mount.position.copy(this.base).add(tmp.s.set(0, 0, -PALM * this.w).multiply(mount.scale).applyQuaternion(mount.quaternion));
    if (!this.arm || this.w < .01) { this.placed = false; return; }
    camera.updateMatrixWorld(); mount.updateMatrix();
    const matrices = motionCompute("viewmodelTransforms", new Float64Array([
      ...camera.matrixWorld.elements, ...aim.toArray(), ...frame.slice(10, 13), frame[13]!,
      ...mount.getWorldScale(tmp.s).toArray(), ...mount.matrix.elements, stanceIndex,
      ...(this.palm ?? new THREE.Matrix4()).elements,
    ]));
    this.place("r", tmp.m.fromArray(matrices), camera, this.w);
    if (this.palm) this.place("l", tmp.m.fromArray(matrices, 16), camera, this.w);
    this.right.close(HOLD[stance].right, this.w);
    this.left.close(HOLD[stance].left, this.w);
  }

  off(): void { this.w = 0; this.placed = false; this.hand.mount.position.copy(this.base); }

  private place(side: "l" | "r", to: THREE.Matrix4, camera: THREE.Camera, w: number): void {
    const [upper, lower, hand] = this.arm![side];
    const pose = motionCompute("placeHand", new Float64Array([
      ...to.elements, ...hand.getWorldPosition(tmp.p).toArray(), ...camera.matrixWorld.elements, w, side === "r" ? 1 : -1,
      ...hand.getWorldQuaternion(tmp.q).toArray(),
    ]));
    reach(upper, lower, hand, new THREE.Vector3().fromArray(pose), new THREE.Vector3().fromArray(pose, 3));
    const turn = motionCompute("worldBlend", new Float64Array([...hand.getWorldQuaternion(tmp.q).toArray(), ...pose.slice(6, 10), w]));
    setWorldQuat(hand, new THREE.Quaternion().fromArray(turn));
  }
}
