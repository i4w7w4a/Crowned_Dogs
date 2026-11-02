import * as THREE from "three";
import { Line2 } from "three/addons/lines/Line2.js";
import { LineGeometry } from "three/addons/lines/LineGeometry.js";
import { LineMaterial } from "three/addons/lines/LineMaterial.js";
import { RustRegionMap } from "../../../../packages/runtime/src/index.ts";

/** Rendering dependencies only; the oblast map lives in Rust. */
export class RegionMap extends RustRegionMap {
  constructor(section: HTMLElement) {
    super({ THREE, Line2, LineGeometry, LineMaterial }, section);
  }
}
