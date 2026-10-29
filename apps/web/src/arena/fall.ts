import * as THREE from "three";
import { RustCollapse } from "../../../../packages/runtime/src/index.ts";
export const FALL_TIME = 1.25;
export class Collapse extends RustCollapse {
  constructor(model: THREE.Object3D) { super(THREE, model); }
}
