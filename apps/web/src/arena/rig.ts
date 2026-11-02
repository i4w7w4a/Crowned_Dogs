import * as THREE from "three";
import { RustArmsOnly, rig_names, rig_find_bones, rig_axes_pitch, rig_axes_roll } from "../../../../packages/runtime/src/index.ts";
export const RIG = rig_names() as Readonly<Record<"hips" | "spine" | "spine1" | "spine2" | "neck" | "head" | "rUpLeg" | "rLeg" | "lUpLeg" | "lLeg", readonly string[]>>;
export interface Bones {
  head: THREE.Object3D; top: THREE.Object3D; neck: THREE.Object3D;
  spine: THREE.Object3D; spine1: THREE.Object3D; spine2: THREE.Object3D; hips: THREE.Object3D;
  rUpLeg: THREE.Object3D; rLeg: THREE.Object3D; lUpLeg: THREE.Object3D; lLeg: THREE.Object3D;
}
export const findBones = (model: THREE.Object3D, box: THREE.Box3): Bones => rig_find_bones(THREE, model, box);
export const pitch = (bone: THREE.Object3D, angle: number): THREE.Object3D => rig_axes_pitch(bone, angle);
export const roll = (bone: THREE.Object3D, angle: number): THREE.Object3D => rig_axes_roll(bone, angle);
export function armsOnly(model: THREE.Object3D): ((on: boolean) => void) & { dispose(): void } {
  const mask = new RustArmsOnly(THREE, model);
  return Object.assign((on: boolean) => mask.set(on), { dispose: () => mask.dispose() });
}
