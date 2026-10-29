import * as THREE from "three";
import { RustHoldOut, RustFlash } from "../../../../packages/runtime/src/index.ts";
export class HoldOut extends RustHoldOut {
  constructor(model: THREE.Object3D) { super(THREE, model); }
}
export class Flash extends RustFlash {
  constructor(map: THREE.Texture, layer: number) { super(THREE, map, layer); }
}
