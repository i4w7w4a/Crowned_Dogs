// The arms and the gun on top of the clips: the pistol held out clear of the chest, and the flash at the
// muzzle when a gun fires.

import * as THREE from "three";
import { reach, setWorldQuat } from "./ik.ts";
import type { Hand } from "./weapons.ts";

const vf = new THREE.Vector3(), vs = new THREE.Vector3(), vt = new THREE.Vector3(), vp = new THREE.Vector3(), vu = new THREE.Vector3();
const qa = new THREE.Quaternion(), qb = new THREE.Quaternion();
const UP = new THREE.Vector3(0, 1, 0);

/** How far both hands are carried forward (and up), metres, out of the chest. */
const AHEAD = 0.24, LIFT = 0.02;

/** The pistol stance from Art's video holds the gun in close at the chest, and on this body the forearms sink
 *  into it. Both hands are carried forward together, so the two-handed grip holds and the gun keeps its aim,
 *  and the elbows come round to hang down and out. */
export class HoldOut {
  private arms: { upper: THREE.Object3D; lower: THREE.Object3D; hand: THREE.Object3D; out: number }[] = [];

  constructor(model: THREE.Object3D) {
    for (const [s, out] of [["l", 1], ["r", -1]] as const) {
      const b = (n: string) => model.getObjectByName(`${n}_${s}`);
      const upper = b("upperarm"), lower = b("lowerarm"), hand = b("hand");
      if (upper && lower && hand) this.arms.push({ upper, lower, hand, out });
    }
  }

  /** `w`: how much of the body holds the pistol stance now (0..1); the figure faces its group's +z, its left is +x. */
  apply(group: THREE.Object3D, w: number): void {
    if (w < 0.01 || this.arms.length < 2) return;
    group.getWorldQuaternion(qa);
    const fwd = vf.set(0, 0, 1).applyQuaternion(qa), side = vs.set(1, 0, 0).applyQuaternion(qa);
    for (const a of this.arms) {
      const keep = a.hand.getWorldQuaternion(qb);
      const target = a.hand.getWorldPosition(vt).addScaledVector(fwd, AHEAD * w).addScaledVector(UP, LIFT * w);
      // the elbow eases from where the clip has it to hanging down and out
      const elbow = a.lower.getWorldPosition(vp);
      const pole = a.upper.getWorldPosition(vu).addScaledVector(UP, -0.5).addScaledVector(side, 0.35 * a.out).addScaledVector(fwd, 0.15);
      reach(a.upper, a.lower, a.hand, target, pole.lerp(elbow, 1 - w));
      setWorldQuat(a.hand, keep);
    }
  }
}

/** The flash at the muzzle: a burst of light that is gone in a few frames, lighting whoever stands near. */
export class Flash {
  readonly sprite: THREE.Sprite;
  private life = 0;
  private hand: Hand | null = null;
  private size = 0.2;

  constructor(map: THREE.Texture, layer: number) {
    this.sprite = new THREE.Sprite(new THREE.SpriteMaterial({
      map, blending: THREE.AdditiveBlending, depthWrite: false, transparent: true, fog: false,
    }));
    this.sprite.layers.set(layer);
    this.sprite.visible = false;
  }

  /** The gun in `hand` fires; `size`, metres across. */
  fire(hand: Hand, size: number): void {
    this.hand = hand; this.size = size; this.life = FLASH;
  }

  /** Follows the muzzle while it lasts; returns how bright it is now (0..1), for the light. */
  update(dt: number): number {
    this.life = Math.max(0, this.life - dt);
    const s = this.sprite, k = this.life / FLASH;
    s.visible = k > 0 && !!this.hand?.muzzle(s.position);
    if (!s.visible) return 0;
    s.scale.setScalar(this.size * (0.7 + 0.5 * k));
    s.material.rotation = Math.random() * Math.PI * 2;
    s.material.opacity = k;
    return k;
  }

  stop(): void { this.life = 0; this.sprite.visible = false; }
}

/** How long a muzzle flash lasts, seconds. */
const FLASH = 0.07;
