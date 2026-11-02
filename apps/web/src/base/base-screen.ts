/** Browser rendering dependencies; navigation, catalogue and lifecycle execute in Rust. */
import { ITEM_BY_ID, type Loadout } from "@crowned-dogs/core";
import { BrowserBase } from "../../../../packages/runtime/src/index.ts";
import { equipmentIcon, equipmentItemIcon } from "./equipment-icons.ts";
import { GEAR_SLOT_NAMES, GEAR_SLOTS, compileKit, loadLocalLoadout, saveLocalLoadout } from "./loadout-state.ts";
import { PersonaViewer } from "./persona-viewer.ts";
import { DepthBackdrop } from "./depth-backdrop.ts";
import { HomeStage } from "./home-stage.ts";
import { RegionMap } from "./region-map.ts";
import { WeaponScreen } from "./weapon-screen.ts";
import { liquidGlass } from "../hud/liquid.ts";
import { gameAudio } from "../audio/arena-audio.ts";
import { backArrow } from "./navigation-icons.ts";
import { ItemViewer, ITEM_MODELS } from "./item-viewer.ts";
import { availableInFitting } from "./model-catalog.ts";
export type BaseView = "base" | "fitting" | "weapon" | "persona" | "region";
export class BaseScreen extends BrowserBase {
  constructor(root: HTMLElement, onBattle: (loadout: Loadout) => Promise<void>) {
    super(root, onBattle, {
      baseUrl: import.meta.env.BASE_URL, ITEM_BY_ID, GEAR_SLOT_NAMES, GEAR_SLOTS,
      compileKit, loadLocalLoadout, saveLocalLoadout, equipmentIcon, equipmentItemIcon,
      PersonaViewer, DepthBackdrop, HomeStage, RegionMap, WeaponScreen, ItemViewer, ITEM_MODELS,
      availableInFitting, liquidGlass, gameAudio, backArrow,
    });
  }
  override show(view: BaseView): void { super.show(view); }
}
