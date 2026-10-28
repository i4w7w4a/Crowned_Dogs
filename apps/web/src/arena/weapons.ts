// What a figure holds in its right hand. Until the real guns are modelled these are stand-ins: plain
// dark blocks the size of each gun in the weapons spec, held the way the animations hold a gun. The
// mocap (anim/scripts/mocap.py) puts the gun's frame in the right hand: forward along the hand from the
// wrist to the knuckles, up to the index finger's side, the grip in the middle of the palm.

import * as THREE from "three";

export type Held = "pistol" | "rifle" | "shotgun" | "sniper" | "knife" | "grenade";

type Box = [x0: number, x1: number, y0: number, y1: number, z0: number, z1: number];
/** Metres in the gun's frame: x forward along the barrel, y up, z to the right; the grip is at the origin. */
const BOXES: Record<Held, Box[]> = {
  // «Пустельга»: slide over the hand, the grip in the fist
  pistol: [[-0.045, 0.155, 0.028, 0.064, -0.014, 0.014], [-0.03, 0.02, -0.07, 0.03, -0.015, 0.015]],
  // «Варяг»: butt at the shoulder 30 cm behind the grip, receiver and barrel ahead, magazine below
  rifle: [[-0.30, 0.42, 0.03, 0.095, -0.022, 0.022], [0.42, 0.68, 0.055, 0.075, -0.01, 0.01],
    [0.08, 0.15, -0.14, 0.03, -0.016, 0.016], [-0.03, 0.02, -0.07, 0.03, -0.015, 0.015]],
  // «Таран»: long receiver, barrel with the magazine tube under it, fore-end where the left hand pumps
  shotgun: [[-0.30, 0.30, 0.03, 0.09, -0.022, 0.022], [0.30, 0.72, 0.065, 0.085, -0.011, 0.011],
    [0.30, 0.66, 0.035, 0.06, -0.012, 0.012], [0.22, 0.40, 0.025, 0.07, -0.024, 0.024], [-0.03, 0.02, -0.07, 0.03, -0.015, 0.015]],
  // «Беркут»: longest, with a scope on top
  sniper: [[-0.30, 0.50, 0.03, 0.09, -0.022, 0.022], [0.50, 0.86, 0.055, 0.075, -0.01, 0.01],
    [0.02, 0.30, 0.10, 0.145, -0.02, 0.02], [0.10, 0.16, -0.08, 0.03, -0.015, 0.015], [-0.03, 0.02, -0.07, 0.03, -0.015, 0.015]],
  // the knife's blade comes out of the thumb side of the fist
  knife: [[-0.012, 0.012, 0.035, 0.205, -0.003, 0.003], [-0.016, 0.016, -0.06, 0.035, -0.012, 0.012]],
  // a hand grenade in the fist, its fuse up at the thumb side
  grenade: [[-0.03, 0.03, -0.04, 0.025, -0.028, 0.028], [-0.009, 0.009, 0.025, 0.05, -0.009, 0.009]],
};

/** Where the grip sits in a hand bone's space (the right hand by default), from the model's rest pose; null
 *  without such bones. The frame: x along the hand, y to the index finger's side, z = x × y. */
export function gripOf(scene: THREE.Object3D, side: "l" | "r" = "r"): THREE.Matrix4 | null {
  scene.updateMatrixWorld(true);
  const bone = (n: string) => scene.getObjectByName(`${n}_${side}`);
  const hand = bone("hand"), mid = bone("middle_01"), index = bone("index_01"), pinky = bone("pinky_01");
  if (!hand || !mid || !index || !pinky) return null;
  const p = (o: THREE.Object3D) => o.getWorldPosition(new THREE.Vector3());
  const h = p(hand), along = p(mid).sub(h);
  const x = along.clone().normalize();
  const y = p(index).sub(p(pinky)); y.addScaledVector(x, -y.dot(x)).normalize();
  const z = new THREE.Vector3().crossVectors(x, y);
  const grip = new THREE.Matrix4().makeBasis(x, y, z).setPosition(h.addScaledVector(along, 0.55));
  return hand.matrixWorld.clone().invert().multiply(grip);
}

/** The weapons in one figure's right hand, all made at once; one shows at a time. */
export class Hand {
  private items = new Map<Held, THREE.Object3D>();
  /** The gun's frame, carried by the hand at the grip. */
  readonly mount: THREE.Object3D;

  constructor(model: THREE.Object3D, grip: THREE.Matrix4, material: THREE.Material, layer = 0) {
    const hand = model.getObjectByName("hand_r")!;
    const at = this.mount = new THREE.Group();
    grip.decompose(at.position, at.quaternion, at.scale);
    hand.add(at);
    for (const [held, boxes] of Object.entries(BOXES) as [Held, Box[]][]) {
      const g = new THREE.Group(); g.visible = false;
      for (const [x0, x1, y0, y1, z0, z1] of boxes) {
        const m = new THREE.Mesh(new THREE.BoxGeometry(x1 - x0, y1 - y0, z1 - z0), material);
        m.position.set((x0 + x1) / 2, (y0 + y1) / 2, (z0 + z1) / 2);
        m.castShadow = true; m.frustumCulled = false; m.layers.set(layer);
        g.add(m);
      }
      at.add(g);
      this.items.set(held, g);
    }
  }

  hold(what: Held | null): void {
    for (const [k, g] of this.items) g.visible = k === what;
  }
}
