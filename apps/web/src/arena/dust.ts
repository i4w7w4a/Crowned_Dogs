import * as THREE from "three";
import { RustDust } from "../../../../packages/runtime/src/index.ts";
export class Dust extends RustDust {
  constructor(three: object = THREE) { super(three); }
}
