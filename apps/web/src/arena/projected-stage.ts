import * as THREE from "three";
import { RustProjectedStage } from "../../../../packages/runtime/src/index.ts";

/** Rust owns projection, render passes and GPU lifetimes; Three.js is the renderer. */
export class ProjectedStage extends RustProjectedStage {
  constructor(renderer: THREE.WebGLRenderer) { super({ THREE }, renderer); }
}
