import * as THREE from "three";
import { sceneLoadGuns, sceneGripOf, RustSceneHand } from "../../../../packages/runtime/src/index.ts";
import { loadModel } from "./load-model.ts";
export type Held = "pistol" | "rifle" | "shotgun" | "sniper" | "knife" | "grenade";
export type Guns = Partial<Record<Held, THREE.Object3D>>;
export const MODELLED_GUNS: readonly string[] = ["rifle-varyag"];
export const MOD_MESHES: Record<string, string> = { "optic-sokol": "optic_sokol", "muzzle-tishina": "muzzle_tishina" };
const host = { THREE, loadModel };
export function loadGuns(): Promise<Guns> { return sceneLoadGuns(host); }
export function gripOf(scene: THREE.Object3D, side: "l" | "r" = "r"): THREE.Matrix4 | null { return sceneGripOf(host, scene, side); }
export class Hand {
  private readonly controller: RustSceneHand;
  private disposed = false;
  readonly mount: THREE.Object3D;
  constructor(model: THREE.Object3D, grip: THREE.Matrix4, material: THREE.Material, options: { layer?: number; guns?: Guns; look?: (m: THREE.Material) => THREE.Material } = {}) {
    this.controller = new RustSceneHand(host, model, grip, material, options);
    this.mount = this.controller.mount;
  }
  fit(mods: Partial<Record<string, string>> = {}): void { if (!this.disposed) this.controller.fit(mods); }
  hold(what: Held | null): void { if (!this.disposed) this.controller.hold(what); }
  muzzle(out: THREE.Vector3): boolean { return !this.disposed && this.controller.muzzle(out); }
  dispose(): void { if (!this.disposed) { this.disposed = true; this.controller.free(); } }
}
