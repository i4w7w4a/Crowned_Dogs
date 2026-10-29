/** Three.js/art adapters. The workshop controller and DOM interaction execute in Rust. */
import type { Loadout } from "@crowned-dogs/core";
import { BrowserWeapon } from "../../../../packages/runtime/src/index.ts";
import { bbox, composeWeapon, itemArt, markup, svg, WEAPON_ART } from "../../../../packages/icons/src/index.ts";
import { equipmentItemIcon } from "./equipment-icons.ts";
import { liquidGlass } from "../hud/liquid.ts";
import { backArrow } from "./navigation-icons.ts";
import { ItemViewer } from "./item-viewer.ts";
import "./weapon-screen.css";
import "./glass-scroll.css";
export class WeaponScreen {
  private readonly controller: BrowserWeapon;
  readonly element: HTMLElement;
  constructor(parent: HTMLElement, onBack: () => void, onChange: (loadout: Loadout) => boolean) {
    this.controller = new BrowserWeapon(parent, onBack, onChange, {
      baseUrl: import.meta.env.BASE_URL, backArrow, liquidGlass, ItemViewer,
      bbox, composeWeapon, itemArt, markup, svg, WEAPON_ART, equipmentItemIcon,
    });
    this.element = this.controller.element as HTMLElement;
  }
  show(loadout: Loadout): void { this.controller.show(loadout); }
  hide(): void { this.controller.hide(); }
  dispose(): void { this.controller.dispose(); }
}
