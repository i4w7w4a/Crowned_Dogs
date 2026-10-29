import * as THREE from "three";
import { RoomEnvironment } from "three/addons/environments/RoomEnvironment.js";
import { getItem, type Loadout } from "@crowned-dogs/core";
import { RustPersonaViewer } from "../../../../packages/runtime/src/index.ts";
import { loadModel } from "../arena/load-model.ts";
import { tuneFittingMaterial } from "./turntable-material.ts";
import { QUALITY, startQuality } from "../arena/quality.ts";
import { TextureTiers } from "../arena/textures.ts";
import { Hand } from "../arena/weapons.ts";
import { fitVaryag, makeFittingStage, makeSling, PLATFORM_TOP, prepareVaryag } from "./fitting-stage.ts";
import { authoredWeaponFit } from "./user-fitting-layout.ts";

export interface PersonaPreviewCoverage { exact: false; description: string; missingItemIds: string[] }
const host = { THREE, RoomEnvironment, getItem, loadModel, tuneFittingMaterial, QUALITY, startQuality,
  TextureTiers, Hand, fitVaryag, makeFittingStage, makeSling, PLATFORM_TOP, prepareVaryag,
  authoredWeaponFit, base: import.meta.env.BASE_URL };

/** Keeps the authored model intact while Rust owns the preview lifecycle. */
export class PersonaViewer {
  private readonly controller: RustPersonaViewer;
  constructor(canvas: HTMLCanvasElement) { this.controller = new RustPersonaViewer(host, canvas); }
  get previewDescription(): string { return this.controller.previewDescription; }
  load(): Promise<void> { return this.controller.load(); }
  resize(width: number, height: number): void { this.controller.resize(width, height); }
  setActive(active: boolean): void { this.controller.setActive(active); }
  shift(x: number, y: number): void { this.controller.shift(x, y); }
  rotate(delta: number): void { this.controller.rotate(delta); }
  updateLoadout(loadout: Loadout): PersonaPreviewCoverage { return this.controller.updateLoadout(loadout); }
  dispose(): void { this.controller.dispose(); }
}
