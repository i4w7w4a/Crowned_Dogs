import * as THREE from "three";
import { RoomEnvironment } from "three/addons/environments/RoomEnvironment.js";
import { getItem, type Loadout } from "@crowned-dogs/core";
import { RustPersonaViewer } from "../../../../packages/runtime/src/index.ts";
import { loadModel } from "../arena/load-model.ts";
import { tuneFittingMaterial } from "./turntable-material.ts";
import { QUALITY, startQuality } from "../arena/quality.ts";
import { TextureTiers } from "../arena/textures.ts";
import { Hand } from "../arena/weapons.ts";
import { fitVaryag, makeCarry, makeFittingStage, makeSling, PLATFORM_TOP, prepareVaryag } from "./fitting-stage.ts";
import { authoredWeaponFit } from "./user-fitting-layout.ts";

export interface PersonaPreviewCoverage { exact: false; description: string; missingItemIds: string[] }
const host = { THREE, RoomEnvironment, getItem, loadModel, tuneFittingMaterial, QUALITY, startQuality,
  TextureTiers, Hand, fitVaryag, makeCarry, makeFittingStage, makeSling, PLATFORM_TOP, prepareVaryag,
  authoredWeaponFit, base: import.meta.env.BASE_URL };

/** Keeps the authored model intact while Rust owns the preview lifecycle. */
export class PersonaViewer extends RustPersonaViewer {
  constructor(canvas: HTMLCanvasElement) { super(host, canvas); }
  override updateLoadout(loadout: Loadout): PersonaPreviewCoverage { return super.updateLoadout(loadout); }
}
