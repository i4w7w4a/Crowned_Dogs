import * as THREE from "three";
import { callRust, sceneTune } from "../../../../packages/runtime/src/index.ts";
import type { Quality } from "./quality.ts";
export function tune(material: THREE.Material, quality: Quality, extra?: (shader: THREE.WebGLProgramParametersWithUniforms) => void, extraKey = "x"): void {
  sceneTune({ THREE, callRust }, material, quality, extra, extraKey);
}
