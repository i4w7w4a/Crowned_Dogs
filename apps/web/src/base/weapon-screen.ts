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
export class WeaponScreen extends BrowserWeapon {
  constructor(parent: HTMLElement, onBack: () => void, onChange: (loadout: Loadout) => boolean) {
    super(parent, onBack, onChange, {
      baseUrl: import.meta.env.BASE_URL, backArrow, liquidGlass, ItemViewer,
      bbox, composeWeapon, itemArt, markup, svg, WEAPON_ART, equipmentItemIcon,
    });
  }
  declare readonly element: HTMLElement;
  override show(loadout: Loadout): void { super.show(loadout); }
}
