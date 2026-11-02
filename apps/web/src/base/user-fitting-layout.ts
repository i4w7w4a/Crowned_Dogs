import * as THREE from "three";
import { sceneAuthoredWeaponFit } from "../../../../packages/runtime/src/index.ts";
import layout from "./user-fitting-layout.json" with { type: "json" };
export function authoredWeaponFit(mount: THREE.Object3D): THREE.Group { return sceneAuthoredWeaponFit({ THREE }, mount, layout); }
