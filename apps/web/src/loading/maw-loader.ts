import * as THREE from "three";
import { GLTFLoader } from "three/addons/loaders/GLTFLoader.js";
import { RoomEnvironment } from "three/addons/environments/RoomEnvironment.js";
import { RustMawLoader } from "../../../../packages/runtime/src/index.ts";
import { liquidGlass } from "../hud/liquid.ts";
import type { LoadingPresentation } from "./battle-loading.ts";
import "./maw-loader.css";

const host = { THREE, GLTFLoader, RoomEnvironment, liquidGlass, base: import.meta.env.BASE_URL };
/** Browser libraries are injected; Rust owns the loading presentation. */
export class MawLoader extends RustMawLoader implements LoadingPresentation {
  constructor() { super(host); }
  override failed(): Promise<"retry" | "back"> { return super.failed(); }
}
