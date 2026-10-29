import * as THREE from "three";
import { GLTFLoader } from "three/addons/loaders/GLTFLoader.js";
import { RoomEnvironment } from "three/addons/environments/RoomEnvironment.js";
import { RustMawLoader } from "../../../../packages/runtime/src/index.ts";
import { liquidGlass } from "../hud/liquid.ts";
import type { LoadingPresentation } from "./battle-loading.ts";
import "./maw-loader.css";

const host = { THREE, GLTFLoader, RoomEnvironment, liquidGlass, base: import.meta.env.BASE_URL };
/** Browser libraries are injected; Rust owns the loading presentation. */
export class MawLoader implements LoadingPresentation {
  private readonly controller = new RustMawLoader(host);
  show(onBack: () => void): void { this.controller.show(onBack); }
  ready(): Promise<void> { return this.controller.ready(); }
  failed(): Promise<"retry" | "back"> { return this.controller.failed(); }
  hide(): void { this.controller.hide(); }
  dispose(): void { this.controller.dispose(); }
}
