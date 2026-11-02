import * as THREE from "three";
import { RustHomeStage } from "../../../../packages/runtime/src/index.ts";
/** Rendering dependencies only; intact-photo depth motion and lifetime live in Rust. */
export class HomeStage extends RustHomeStage {
  constructor(canvas: HTMLCanvasElement, url: string) { super({ THREE }, canvas, url); }
}
