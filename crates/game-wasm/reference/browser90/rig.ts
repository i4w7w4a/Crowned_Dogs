// The few bones the arena moves by hand (hit flinches, the fall, where the rings and
// the sync connector sit), found by name in either rig we use: the Mixamo placeholder
// and the UE Mannequin skeleton of our Personas.

import * as THREE from "three";
import { motionCompute } from "../../../../packages/runtime/src/index.ts";

/** The bones moved by hand: Mixamo names (the placeholder X Bot; three drops the colon) and the UE Mannequin names of our Personas. */
export const RIG = {
  hips: ["mixamorigHips", "pelvis"], spine: ["mixamorigSpine", "spine_01"], spine1: ["mixamorigSpine1", "spine_02"],
  spine2: ["mixamorigSpine2", "spine_03"], neck: ["mixamorigNeck", "neck_01"], head: ["mixamorigHead", "head"],
  rUpLeg: ["mixamorigRightUpLeg", "thigh_r"], rLeg: ["mixamorigRightLeg", "calf_r"],
  lUpLeg: ["mixamorigLeftUpLeg", "thigh_l"], lLeg: ["mixamorigLeftLeg", "calf_l"],
} as const;

export interface Bones {
  head: THREE.Object3D; top: THREE.Object3D; neck: THREE.Object3D;
  spine: THREE.Object3D; spine1: THREE.Object3D; spine2: THREE.Object3D; hips: THREE.Object3D;
  rUpLeg: THREE.Object3D; rLeg: THREE.Object3D; lUpLeg: THREE.Object3D; lLeg: THREE.Object3D;
}

/** Finds the bones the arena moves by hand, in a Mixamo or a UE Mannequin rig. `box` is the model's bounds at rest. */
export function findBones(model: THREE.Object3D, box: THREE.Box3): Bones {
  const q = new THREE.Quaternion();
  const find = (key: keyof typeof RIG): THREE.Object3D => {
    const b = RIG[key].map((n) => model.getObjectByName(n)).find((o) => o);
    if (!b) throw new Error(`the model has no bone ${RIG[key].join(" / ")}`);
    // the figure's own left-right and front-back lines in this bone's space, whichever way the rig rolls its bones
    b.getWorldQuaternion(q).invert();
    AXES.set(b, { pitch: new THREE.Vector3(1, 0, 0).applyQuaternion(q), roll: new THREE.Vector3(0, 0, 1).applyQuaternion(q) });
    return b;
  };
  const head = find("head");
  let top = model.getObjectByName("mixamorigHeadTop_End");
  if (!top) {
    // UE rigs end at the head bone: mark the crown, the highest point of the model, and let the head carry it
    top = new THREE.Object3D();
    const p = head.getWorldPosition(new THREE.Vector3());
    top.position.set(p.x, box.max.y, p.z);
    head.attach(top);
  }
  return {
    head, top, neck: find("neck"), spine: find("spine"), spine1: find("spine1"), spine2: find("spine2"), hips: find("hips"),
    rUpLeg: find("rUpLeg"), rLeg: find("rLeg"), lUpLeg: find("lUpLeg"), lLeg: find("lLeg"),
  };
}

const AXES = new WeakMap<THREE.Object3D, { pitch: THREE.Vector3; roll: THREE.Vector3 }>();
/** Bends a bone forward (or back, for a negative angle) about the figure's left-right line. */
export const pitch = (b: THREE.Object3D, a: number) => b.rotateOnAxis(AXES.get(b)!.pitch, a);
/** Tilts a bone sideways about the figure's front-back line. */
export const roll = (b: THREE.Object3D, a: number) => b.rotateOnAxis(AXES.get(b)!.roll, a);

/** Arm bones: the upper arm down to the fingertips (UE Mannequin and Mixamo names). */
const ARM = /^(upperarm|lowerarm|hand|thumb|index|middle|ring|pinky)_|^mixamorig(Left|Right)(Arm|ForeArm|Hand)/;

/** First person: a switch that shows only a figure's arms and hands. Seen from inside its own head the rest
 *  of the body (the neck, the collar, the shoulders) would fill the lens. A triangle stays when all its
 *  corners follow arm bones the most; a mesh with no such triangles (the head) is hidden. Call it once the figure
 *  is dressed: what is hidden then stays hidden. */
export function armsOnly(model: THREE.Object3D): (on: boolean) => void {
  const swaps: [THREE.SkinnedMesh, THREE.BufferAttribute | null, THREE.BufferAttribute | null, boolean][] = [];
  model.traverse((o) => {
    if (!(o instanceof THREE.SkinnedMesh)) return;
    const g = o.geometry, si = g.getAttribute("skinIndex"), sw = g.getAttribute("skinWeight");
    const arm = o.skeleton.bones.map((b) => ARM.test(b.name));
    const all = g.index, n = all ? all.count : g.getAttribute("position").count;
    const count = si.count, weightsAt = 3 + count * 4, armsAt = weightsAt + count * 4, indicesAt = armsAt + arm.length;
    const input = new Float64Array(indicesAt + n);
    input.set([count, arm.length, n]);
    for (let v = 0; v < count; v++) for (let k = 0; k < 4; k++) {
      input[3 + v * 4 + k] = si.getComponent(v, k); input[weightsAt + v * 4 + k] = sw.getComponent(v, k);
    }
    input.set(arm.map(Number), armsAt);
    for (let i = 0; i < n; i++) input[indicesAt + i] = all ? all.getX(i) : i;
    const kept = motionCompute("armTriangles", input);
    swaps.push([o, all, kept.length ? new THREE.BufferAttribute(new Uint32Array(kept), 1) : null, o.visible]);
  });
  return (on) => {
    for (const [m, all, arms, shown] of swaps) { m.visible = shown && (!on || arms !== null); m.geometry.setIndex(on && arms ? arms : all); }
  };
}
