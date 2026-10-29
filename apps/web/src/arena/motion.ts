// Rust owns clip preparation, action scheduling and mixer event lifetime.
import * as THREE from "three";
import { RustMotion, RustPose } from "../../../../packages/runtime/src/index.ts";
import type { Held } from "./weapons.ts";
export type Stance = "pistol" | "rifle" | "shotgun" | "sniper";
export type Attack = Stance | "knife" | "grenade";
export class Motion extends RustMotion {
  constructor(mixer: THREE.AnimationMixer, clips: THREE.AnimationClip[]) { super(THREE, mixer, clips); }
  declare onHold: ((what: Held | null) => void) | null;
  declare onMark: ((kind: Attack) => void) | null;
}
export class Pose extends RustPose {}
