/** Browser rendering dependencies; navigation, catalogue and lifecycle execute in Rust. */
import { ITEM_BY_ID, type Loadout } from "@crowned-dogs/core";
import { BrowserBase } from "../../../../packages/runtime/src/index.ts";
import { equipmentIcon, equipmentItemIcon } from "./equipment-icons.ts";
import { GEAR_SLOT_NAMES, GEAR_SLOTS, compileKit, loadLocalLoadout, saveLocalLoadout } from "./loadout-state.ts";
import { PersonaViewer } from "./persona-viewer.ts";
import { DepthBackdrop } from "./depth-backdrop.ts";
import { WeaponScreen } from "./weapon-screen.ts";
import { liquidGlass } from "../hud/liquid.ts";
import { gameAudio } from "../audio/arena-audio.ts";
import { backArrow } from "./navigation-icons.ts";
import { ItemViewer, ITEM_MODELS } from "./item-viewer.ts";
import { availableInFitting } from "./model-catalog.ts";
export type BaseView = "base" | "fitting" | "weapon";
export class BaseScreen {
  private readonly controller: BrowserBase;
  constructor(root: HTMLElement, onBattle: (loadout: Loadout) => Promise<void>) {
    this.controller = new BrowserBase(root, onBattle, {
      baseUrl: import.meta.env.BASE_URL, ITEM_BY_ID, GEAR_SLOT_NAMES, GEAR_SLOTS,
      compileKit, loadLocalLoadout, saveLocalLoadout, equipmentIcon, equipmentItemIcon,
      PersonaViewer, DepthBackdrop, WeaponScreen, ItemViewer, ITEM_MODELS,
      availableInFitting, liquidGlass, gameAudio, backArrow,
    });
  }
  show(view: BaseView): void { this.controller.show(view); }
  setActive(active: boolean): void { this.controller.setActive(active); }
  dispose(): void { this.controller.dispose(); }
}
