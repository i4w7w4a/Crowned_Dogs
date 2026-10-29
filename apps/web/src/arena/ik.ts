// Typed Three SDK boundary; Rust owns sampling, constraints and scene mutation.
import * as THREE from "three";
import { RustGrip, RustFeet, rig_set_world_quat, rig_turn, rig_reach, rig_gait_of } from "../../../../packages/runtime/src/index.ts";
export type Gait = { speed: number; passing: number };
export const setWorldQuat = (bone: THREE.Object3D, rotation: THREE.Quaternion): void => rig_set_world_quat(THREE, bone, rotation);
export const turn = (bone: THREE.Object3D, from: THREE.Vector3, to: THREE.Vector3): void => rig_turn(THREE, bone, from, to);
export const reach = (a: THREE.Object3D, b: THREE.Object3D, c: THREE.Object3D, target: THREE.Vector3, pole: THREE.Vector3): void => rig_reach(THREE, a, b, c, target, pole);
export const gaitOf = (model: THREE.Object3D, clip: THREE.AnimationClip): Gait | null => rig_gait_of(THREE, model, clip);
export class Grip extends RustGrip {
  constructor(model: THREE.Object3D, side: "l" | "r") { super(THREE, model, side); }
}
export class Feet extends RustFeet {
  constructor(model: THREE.Object3D, clips: THREE.AnimationClip[], stances: string[]) { super(THREE, model, clips, stances); }
}
