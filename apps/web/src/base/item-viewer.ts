import * as THREE from "three";
import { RoomEnvironment } from "three/addons/environments/RoomEnvironment.js";
import { MOD_SLOTS, type Loadout, type ModSlot } from "@crowned-dogs/core";
import { RustItemViewer } from "../../../../packages/runtime/src/index.ts";
import { loadModel } from "../arena/load-model.ts";
import { QUALITY, startQuality } from "../arena/quality.ts";
import { TextureTiers } from "../arena/textures.ts";
import { tune } from "../arena/materials.ts";
import { fitVaryag, prepareVaryag } from "./fitting-stage.ts";
import { ITEM_MODELS } from "./model-catalog.ts";
import "./item-viewer.css";
export { ITEM_MODELS } from "./model-catalog.ts";

type ViewerOptions = { workbench?: boolean; onSockets?: (points: Partial<Record<ModSlot, [number, number]>>) => void };
const host = { THREE, RoomEnvironment, MOD_SLOTS, loadModel, QUALITY, startQuality, TextureTiers,
  tune, fitVaryag, prepareVaryag, ITEM_MODELS, base: import.meta.env.BASE_URL };

/** Platform imports only; Rust owns requests, input, rendering and disposal. */
export class ItemViewer extends RustItemViewer {
  constructor(element: HTMLElement, options: ViewerOptions = {}) { super(host, element, options); }
  override show(id: string, name: string, loadout?: Loadout): Promise<void> { return super.show(id, name, loadout); }
}
