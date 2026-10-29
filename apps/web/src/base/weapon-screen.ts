import type { Item, Loadout, ModSlot, WeaponItem } from "@crowned-dogs/core";
import { callRust } from "../../../../packages/runtime/src/index.ts";
import { bbox, composeWeapon, itemArt, markup, svg, WEAPON_ART, type Slot } from "../../../../packages/icons/src/index.ts";
import { equipmentItemIcon } from "./equipment-icons.ts";
import { liquidGlass } from "../hud/liquid.ts";
import { GlassScroll } from "./glass-scroll.ts";
import { backArrow } from "./navigation-icons.ts";
import { ItemViewer } from "./item-viewer.ts";
import { MOD_SLOTS, equipItem, equipWeaponMod } from "./loadout-state.ts";
import "./weapon-screen.css";

interface WorkshopModel { weapon: WeaponItem; parts: Record<Slot, string>; className: string; displayName: string; model: string | null; sockets: { slot: ModSlot; name: string; title: string; modified: boolean }[] }
interface Choice { item: Item; selected: boolean }
let iconSerial = 0;
function partIcon(id: string): string {
  const art = itemArt(id);
  return art ? svg(art, `workshop-part-${iconSerial++}`, 6).replace('<svg ', '<svg aria-hidden="true" ') : '<span aria-hidden="true">—</span>';
}

/** A separate, quiet weapon workbench. The catalogue and numbers stay behind taps. */
export class WeaponScreen {
  readonly element: HTMLElement;
  private loadout: Loadout | null = null;
  private selectedSlot: ModSlot | null = null;
  private sheet: HTMLDialogElement;
  private sheetBody: HTMLElement;
  private sheetScroll: GlassScroll;
  private origin: HTMLElement | null = null;
  private onChange: (loadout: Loadout) => boolean;
  private viewer: ItemViewer | null = null;

  constructor(parent: HTMLElement, onBack: () => void, onChange: (loadout: Loadout) => boolean) {
    this.onChange = onChange;
    this.element = document.createElement("section");
    this.element.className = "cw-screen";
    this.element.hidden = true;
    this.element.setAttribute("aria-label", "Оружие");
    this.element.innerHTML = callRust("client.workshop.initial", [import.meta.env.BASE_URL, backArrow]);
    parent.append(this.element);
    liquidGlass(this.element);
    this.sheet = this.element.querySelector("dialog")!;
    this.sheetBody = this.element.querySelector(".cw-sheet-body")!;
    this.sheetScroll = new GlassScroll(this.sheetBody);
    this.element.querySelector(".cw-back")!.addEventListener("click", onBack);
    this.element.querySelector(".cw-info")!.addEventListener("click", () => this.showStats());
    this.element.querySelector(".cw-change")!.addEventListener("click", () => this.showWeapons());
    this.element.querySelector(".cw-close")!.addEventListener("click", () => this.sheet.close());
    this.element.querySelector(".cw-sockets")!.addEventListener("click", e => {
      const button = (e.target as Element).closest<HTMLElement>("[data-socket]");
      if (button) this.showMods(button.dataset.socket as ModSlot);
    });
    this.element.querySelector(".cw-stage")!.addEventListener("click", e => {
      const socket = (e.target as Element).closest<SVGElement>("[data-anchor]");
      if (socket) {
        const slot = socket.dataset.anchor as ModSlot;
        this.element.querySelector<HTMLButtonElement>(`[data-socket="${slot}"]`)!.focus();
        this.showMods(slot);
      }
    });
    this.sheet.addEventListener("click", e => {
      if (e.target !== this.sheet) return;
      const r = this.sheet.getBoundingClientRect();
      if (e.clientX < r.left || e.clientX > r.right || e.clientY < r.top || e.clientY > r.bottom) this.sheet.close();
    });
    this.sheet.addEventListener("close", () => {
      this.selectedSlot = null;
      this.element.querySelectorAll("[data-socket]").forEach(b => b.setAttribute("aria-expanded", "false"));
      if (!this.element.hidden) this.origin?.focus({ preventScroll: true });
    });
    this.sheetBody.addEventListener("click", e => {
      const button = (e.target as Element).closest<HTMLButtonElement>("[data-mod], [data-weapon]");
      if (!button || !this.loadout) return;
      try {
        const next = button.dataset.weapon
          ? equipItem(this.loadout, "weapon1", button.dataset.weapon)
          : equipWeaponMod(this.loadout, this.selectedSlot!, button.dataset.mod!);
        this.loadout = next;
        const saved = this.onChange(next);
        this.render();
        this.sheet.close();
        const error = this.element.querySelector<HTMLElement>(".cw-error")!;
        error.textContent = saved ? "" : "Не удалось сохранить выбор. Он доступен до закрытия страницы.";
        error.hidden = saved;
      } catch (error) {
        this.sheetBody.querySelector(".cw-sheet-error")?.remove();
        const message = document.createElement("p"); message.className = "cw-sheet-error"; message.setAttribute("role", "alert");
        message.textContent = error instanceof Error ? error.message : "Не удалось установить деталь.";
        this.sheetBody.prepend(message);
      }
    });
  }

  show(loadout: Loadout): void {
    this.loadout = loadout;
    this.element.hidden = false;
    this.render();
    this.element.querySelector<HTMLButtonElement>(".cw-back")!.focus({ preventScroll: true });
  }

  hide(): void { this.element.hidden = true; this.viewer?.setActive(false); if (this.sheet.open) this.sheet.close(); }
  dispose(): void { this.hide(); this.viewer?.dispose(); this.sheetScroll.dispose(); this.element.remove(); }

  private slotName(slot: ModSlot): string {
    return callRust("client.workshop.slotName", [this.loadout, slot]);
  }

  private render(): void {
    if (!this.loadout) return;
    const view = callRust<WorkshopModel>("client.workshop.model", [this.loadout]);
    const { weapon, parts } = view;
    this.element.querySelector(".cw-class")!.textContent = view.className;
    this.element.querySelector(".cw-name")!.textContent = view.displayName;
    const shape = composeWeapon(weapon.weapon, parts);
    this.element.querySelector(".cw-gun")!.innerHTML = callRust("client.workshop.fallback", [bbox(shape), WEAPON_ART[weapon.weapon].anchors, markup(shape, "cw-weapon-" + iconSerial++)]);
    const model = this.element.querySelector<HTMLElement>(".cw-model")!;
    const real = view.model !== null;
    this.element.querySelector(".cw-stage")!.classList.toggle("cw-stage-3d", real);
    this.element.querySelector(".cw-gun")!.toggleAttribute("hidden", real);
    this.element.querySelector(".cw-links")!.toggleAttribute("hidden", !real);
    model.hidden = !real;
    if (real) {
      this.viewer ??= new ItemViewer(model, { workbench: true, onSockets: points => this.drawSocketLinks(points) });
      void this.viewer.show(weapon.id, weapon.name, this.loadout);
    } else this.viewer?.hide();
    for (const socket of view.sockets) {
      const button = this.element.querySelector<HTMLButtonElement>('[data-socket="' + socket.slot + '"]')!;
      button.querySelector(".cw-socket-name")!.textContent = socket.name;
      button.setAttribute("aria-label", socket.name + ": " + socket.title);
      button.setAttribute("aria-haspopup", "dialog");
      button.setAttribute("aria-expanded", String(this.sheet.open && this.selectedSlot === socket.slot));
      button.classList.toggle("cw-modified", socket.modified);
      button.title = socket.title;
    }
  }

  private drawSocketLinks(points: Partial<Record<ModSlot, [number, number]>>): void {
    const stage = this.element.querySelector<HTMLElement>(".cw-stage")!;
    const links = this.element.querySelector<SVGSVGElement>(".cw-links")!;
    const w = stage.clientWidth, h = stage.clientHeight;
    links.setAttribute("viewBox", "0 0 " + w + " " + h);
    links.innerHTML = callRust("client.workshop.links", [points, w, h]);
  }

  private openSheet(title: string): void {
    this.origin = document.activeElement as HTMLElement;
    this.element.querySelector("#cw-sheet-title")!.textContent = title;
    this.sheet.showModal();
    liquidGlass(this.sheet);
    this.sheetBody.scrollTop = 0;
    this.sheetScroll.refresh();
    this.element.querySelector<HTMLButtonElement>(".cw-close")!.focus({ preventScroll: true });
  }

  private showMods(slot: ModSlot): void {
    if (!this.loadout) return;
    this.selectedSlot = slot;
    const choices = callRust<Choice[]>("client.workshop.choices", [this.loadout, slot]);
    this.sheetBody.innerHTML = callRust("client.workshop.options", [choices.map(c => ({ ...c, icon: partIcon(c.item.id) })), "mod"]);
    this.openSheet(this.slotName(slot));
    this.element.querySelector('[data-socket="' + slot + '"]')!.setAttribute("aria-expanded", "true");
  }

  private showWeapons(): void {
    if (!this.loadout) return;
    const choices = callRust<Choice[]>("client.workshop.choices", [this.loadout, "weapon1"]);
    this.sheetBody.innerHTML = callRust("client.workshop.options", [choices.map(c => ({ ...c, icon: equipmentItemIcon(c.item.id) })), "weapon"]);
    this.openSheet("Сменить оружие");
  }

  private showStats(): void {
    if (!this.loadout) return;
    this.sheetBody.innerHTML = callRust("client.workshop.stats", [this.loadout]);
    this.openSheet("Характеристики");
  }
}
