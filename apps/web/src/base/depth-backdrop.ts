import * as THREE from "three";
import { callRust, RustDepthBackdrop } from "../../../../packages/runtime/src/index.ts";
export class DepthBackdrop {
  private readonly controller: RustDepthBackdrop;
  private disposed = false;
  readonly ready: Promise<void>;
  constructor(canvas: HTMLCanvasElement, color: string, depth: string, ratio: number, zoom: number) {
    this.controller = new RustDepthBackdrop({ THREE, callRust }, canvas, color, depth, ratio, zoom);
    this.ready = this.controller.ready;
  }
  draw(x: number, y: number): void { if (!this.disposed) this.controller.draw(x, y); }
  dispose(): void { if (!this.disposed) { this.disposed = true; this.controller.dispose(); this.controller.free(); } }
}
