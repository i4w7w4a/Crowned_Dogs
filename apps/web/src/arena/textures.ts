import * as THREE from "three";
import { RustTextureTiers } from "../../../../packages/runtime/src/index.ts";

/** SDK boundary only; Rust owns tier requests, atomic swaps and disposal. */
export class TextureTiers extends RustTextureTiers {
  constructor(roots: THREE.Object3D[], model: string) { super({ THREE }, roots, model); }
  override set(size: number, anisotropy: number): Promise<void> { return super.set(size, anisotropy); }
}
