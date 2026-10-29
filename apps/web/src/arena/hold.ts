// The arms and the gun on top of the clips: the pistol held out clear of the chest, and the flash at the
// muzzle when a gun fires.

import * as THREE from "three";
import { motionCompute } from "../../../../packages/runtime/src/index.ts";
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
    for (const a of this.arms) {
      const keep = a.hand.getWorldQuaternion(qb);
      const pose = motionCompute("holdOut", new Float64Array([
        ...qa.toArray(), w, a.out, ...a.hand.getWorldPosition(vt).toArray(), ...a.lower.getWorldPosition(vp).toArray(), ...a.upper.getWorldPosition(vu).toArray(),
      ]));
      reach(a.upper, a.lower, a.hand, new THREE.Vector3().fromArray(pose), new THREE.Vector3().fromArray(pose, 3));
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
    const frame = motionCompute("flash", new Float64Array([this.life, dt, this.size]));
    this.life = frame[0]!;
    const s = this.sprite, k = frame[1]!;
    s.visible = k > 0 && !!this.hand?.muzzle(s.position);
    if (!s.visible) return 0;
    s.scale.setScalar(frame[2]!);
    s.material.rotation = Math.random() * Math.PI * 2;
    s.material.opacity = k;
    return k;
  }

  stop(): void { this.life = 0; this.sprite.visible = false; }
}

/** How long a muzzle flash lasts, seconds. */
const FLASH = 0.07;
