// What a figure holds in its right hand: the real gun where its model is made (MODELS), elsewhere a
// stand-in: plain dark blocks the size of each gun in the weapons spec, held the way the animations hold a gun. The
// mocap (anim/scripts/mocap.py) puts the gun's frame in the right hand: forward along the hand from the
// wrist to the knuckles, up to the index finger's side, the grip in the middle of the palm.

import * as THREE from "three";
import { loadModel } from "./load-model.ts";

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

/** The real guns, by what is held. Each model has its origin at the grip, in the gun's frame above (glTF's axes are
 *  the same), the factory parts on their sockets (nodes socket_<slot>) and the mods loose at the origin, marked in
 *  userData: slot, factory, and for a mod the factory part it replaces. */
const MODELS: Partial<Record<Held, string>> = { rifle: "models/varyag.json" };
/** Weapon items whose own gun is modelled (the rest are drawn as their class's stand-in). */
export const MODELLED_GUNS: readonly string[] = ["rifle-varyag"];
/** Mod items that have a mesh in their gun's model: item id → mesh name. */
export const MOD_MESHES: Record<string, string> = { "optic-sokol": "optic_sokol", "muzzle-tishina": "muzzle_tishina" };

export type Guns = Partial<Record<Held, THREE.Object3D>>;

/** Loads the real guns, with each mod moved onto its socket; a gun that does not load stays a stand-in. */
export async function loadGuns(): Promise<Guns> {
  const guns: Guns = {};
  await Promise.all((Object.entries(MODELS) as [Held, string][]).map(async ([held, url]) => {
    try {
      const gun = (await loadModel(url)).scene;
      for (const mod of gun.children.filter((o) => o.userData.factory === false)) {
        gun.getObjectByName(`socket_${mod.userData.slot}`)?.add(mod);
      }
      guns[held] = gun;
    } catch { /* the stand-in stays */ }
  }));
  return guns;
}

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

  /** `material` paints the stand-ins; a real gun from `guns` keeps its own materials, passed through `look`
   *  (once each), e.g. to give this hand copies of its own. */
  constructor(model: THREE.Object3D, grip: THREE.Matrix4, material: THREE.Material,
    { layer = 0, guns = {}, look = (m) => m }: { layer?: number; guns?: Guns; look?: (m: THREE.Material) => THREE.Material } = {}) {
    const hand = model.getObjectByName("hand_r")!;
    const at = this.mount = new THREE.Group();
    grip.decompose(at.position, at.quaternion, at.scale);
    hand.add(at);
    const looks = new Map<THREE.Material, THREE.Material>();
    for (const [held, boxes] of Object.entries(BOXES) as [Held, Box[]][]) {
      const g = new THREE.Group(); g.visible = false;
      const gun = guns[held];
      if (gun) {
        g.add(gun.clone());
        g.traverse((o) => {
          if (!(o instanceof THREE.Mesh)) return;
          const m = o.material as THREE.Material;
          if (!looks.has(m)) looks.set(m, look(m));
          o.material = looks.get(m)!;
        });
      } else {
        for (const [x0, x1, y0, y1, z0, z1] of boxes) {
          const m = new THREE.Mesh(new THREE.BoxGeometry(x1 - x0, y1 - y0, z1 - z0), material);
          m.position.set((x0 + x1) / 2, (y0 + y1) / 2, (z0 + z1) / 2);
          g.add(m);
        }
      }
      g.traverse((o) => { if (o instanceof THREE.Mesh) { o.castShadow = true; o.frustumCulled = false; o.layers.set(layer); } });
      at.add(g);
      this.items.set(held, g);
    }
    this.fit();
  }

  /** Puts the kit's mods on the real guns (slot → item id, as in a Loadout); a mod without a mesh, or no mod,
   *  leaves the factory part. */
  fit(mods: Partial<Record<string, string>> = {}): void {
    this.muzzles.clear(); // a silencer moves the muzzle
    for (const g of this.items.values()) {
      const on = new Set<string>();
      g.traverse((o) => {
        if (o.userData.factory !== false) return;
        o.visible = MOD_MESHES[mods[o.userData.slot] ?? ""] === o.name;
        if (o.visible && o.userData.replaces) on.add(o.userData.replaces);
      });
      g.traverse((o) => { if (o.userData.factory === true) o.visible = !on.has(o.name); });
    }
  }

  hold(what: Held | null): void {
    for (const [k, g] of this.items) g.visible = k === what;
    this.shown = what;
  }

  /** Puts the muzzle of the gun in hand, the front end of its barrel, into `out` (world). False when the hand
   *  holds no gun. */
  muzzle(out: THREE.Vector3): boolean {
    const what = this.shown, g = what && this.items.get(what);
    if (!g || what === "knife" || what === "grenade") return false;
    let at = this.muzzles.get(what);
    if (!at) this.muzzles.set(what, at = frontOf(g));
    out.copy(at);
    this.mount.localToWorld(out);
    return true;
  }

  private shown: Held | null = null;
  /** Each gun's muzzle in the gun's frame, found once from its meshes. */
  private muzzles = new Map<Held, THREE.Vector3>();
}

/** The front end of a gun, in its frame: the middle of whatever reaches furthest forward along the barrel. */
function frontOf(gun: THREE.Object3D): THREE.Vector3 {
  gun.updateMatrixWorld(true);
  const into = gun.matrixWorld.clone().invert(), p = new THREE.Vector3(), front: THREE.Vector3[] = [];
  let max = -Infinity;
  gun.traverse((o) => {
    if (!(o instanceof THREE.Mesh) || !o.visible) return;
    o.geometry.computeBoundingBox();
    const b = o.geometry.boundingBox!, m = new THREE.Matrix4().multiplyMatrices(into, o.matrixWorld);
    for (const x of [b.min.x, b.max.x]) for (const y of [b.min.y, b.max.y]) for (const z of [b.min.z, b.max.z]) {
      p.set(x, y, z).applyMatrix4(m);
      front.push(p.clone());
      max = Math.max(max, p.x);
    }
  });
  const near = front.filter((v) => v.x > max - 0.03);
  const at = near.reduce((s, v) => s.add(v), new THREE.Vector3()).divideScalar(Math.max(1, near.length));
  return at.setX(max);
}
