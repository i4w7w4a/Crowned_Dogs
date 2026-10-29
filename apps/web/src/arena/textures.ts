import * as THREE from "three";
import { RustTextureTiers } from "../../../../packages/runtime/src/index.ts";

/** SDK boundary only; Rust owns tier requests, atomic swaps and disposal. */
export class TextureTiers {
  private readonly engine: RustTextureTiers;
  constructor(roots: THREE.Object3D[], model: string) { this.engine = new RustTextureTiers({ THREE }, roots, model); }
  set(size: number, anisotropy: number): Promise<void> { return this.engine.set(size, anisotropy) as Promise<void>; }
  get current(): number { return this.engine.current; }
  dispose(): void { this.engine.dispose(); }
}
