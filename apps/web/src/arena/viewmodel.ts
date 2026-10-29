import * as THREE from "three";
import { RustViewmodel } from "../../../../packages/runtime/src/index.ts";
import { gripOf, type Hand } from "./weapons.ts";
export class Viewmodel extends RustViewmodel {
  constructor(model: THREE.Object3D, hand: Hand) { super(THREE, model, hand, gripOf(model, "l")); }
}
