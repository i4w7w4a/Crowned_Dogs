import { BASE_STATS, ITEM_BY_ID, WEAPONS, type Loadout, type ModSlot, type WeaponItem } from "@crowned-dogs/core";
import { bbox, composeWeapon, itemArt, markup, svg, WEAPON_ART, type Slot } from "../../../../packages/icons/src/index.ts";
import { equipmentItemIcon } from "./equipment-icons.ts";
import { liquidGlass } from "../hud/liquid.ts";
import { GlassScroll } from "./glass-scroll.ts";
import { backArrow } from "./navigation-icons.ts";
import { ItemViewer } from "./item-viewer.ts";
import { availableInFitting } from "./model-catalog.ts";
import {
  MOD_SLOTS, MOD_SLOT_NAMES, compileKit, describeItem, effectiveWeaponModId,
  equipItem, equipWeaponMod, itemsForSlot, weaponModsForSlot,
} from "./loadout-state.ts";
import "./weapon-screen.css";

const escape = (s: string) => s.replace(/[&<>"']/g, c => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c]!));
const equippedCheck = '<span class="cd-equipped-check" aria-hidden="true"><svg viewBox="0 0 24 24"><path d="m5 12 4 4L19 6"/></svg></span>';
const socketNames: Record<ModSlot, string> = { ...MOD_SLOT_NAMES, muzzle: "Дуло", rail: "Планка", stock: "Приклад" };
// Labels stay far enough apart for a finger; the leaders end at the real art sockets.
const labelPoints: Record<ModSlot, [number, number]> = {
  optic: [76, 53], muzzle: [286, 53], rail: [292, 256], mag: [184, 310], stock: [56, 264],
};
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
    this.element.innerHTML = `
      <img class="cw-room" src="${import.meta.env.BASE_URL}img/base/fitting-room.webp" alt="" draggable="false">
      <header class="cw-header">
        <button class="cw-back glass" type="button" aria-label="Назад к снаряжению">${backArrow}<span>Назад</span></button>
        <span>Оружие</span>
        <button class="cw-info cw-icon glass" type="button" aria-label="Характеристики оружия"><span aria-hidden="true">i</span></button>
      </header>
      <div class="cw-title"><p class="cw-class"></p><h1 class="cw-name"></h1></div>
      <div class="cw-stage">
        <div class="cw-model" hidden></div>
        <svg class="cw-gun" viewBox="0 0 360 360" role="img" aria-label="Оружие с установленными модификациями"></svg>
        <svg class="cw-links" hidden aria-hidden="true"></svg>
        <div class="cw-sockets">${MOD_SLOTS.map(slot => `<button class="cw-socket glass" type="button" data-socket="${slot}" style="left:${labelPoints[slot][0] / 3.6}%;top:${labelPoints[slot][1] / 3.6}%"><span class="cw-socket-dot" aria-hidden="true"></span><span class="cw-socket-name">${socketNames[slot]}</span></button>`).join("")}</div>
      </div>
      <footer class="cw-footer"><button class="cw-change glass" type="button">Сменить оружие</button><p class="cw-error" role="status" hidden></p></footer>
      <dialog class="cw-sheet sheet glass" aria-labelledby="cw-sheet-title">
        <header class="cw-sheet-header"><h2 id="cw-sheet-title"></h2><button class="cw-close cw-icon glass" type="button" aria-label="Закрыть панель">×</button></header>
        <div class="cw-sheet-body"></div>
      </dialog>`;
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

  private weapon(): WeaponItem {
    const item = ITEM_BY_ID.get(this.loadout!.weapon1.item);
    if (!item || item.kind !== "weapon") throw new Error("Неизвестное оружие");
    return item;
  }

  private slotName(slot: ModSlot): string {
    return slot === "stock" && this.weapon().weapon === "pistol" ? "Рукоять" : socketNames[slot];
  }

  private render(): void {
    if (!this.loadout) return;
    const weapon = this.weapon();
    this.element.querySelector(".cw-class")!.textContent = WEAPONS[weapon.weapon].name;
    this.element.querySelector(".cw-name")!.textContent = weapon.name.match(/«(.*?)»/)?.[1] ?? weapon.name;
    const parts = Object.fromEntries(MOD_SLOTS.map(slot => [slot, effectiveWeaponModId(this.loadout!, slot)])) as Record<Slot, string>;
    const shape = composeWeapon(weapon.weapon, parts);
    const [x, y, w, h] = bbox(shape);
    const angle = -Math.PI / 6, cos = Math.cos(angle), sin = Math.sin(angle);
    const scale = Math.min(300 / (w * cos + h * -sin), 260 / (w * -sin + h * cos));
    const cx = x + w / 2, cy = y + h / 2;
    const project = ([ax, ay]: [number, number]): [number, number] => [180 + ((ax - cx) * cos - (ay - cy) * sin) * scale, 180 + ((ax - cx) * sin + (ay - cy) * cos) * scale];
    const lines = MOD_SLOTS.map(slot => {
      const [ax, ay] = project(WEAPON_ART[weapon.weapon].anchors[slot]);
      const [lx, ly] = labelPoints[slot];
      return `<g class="cw-anchor" data-anchor="${slot}" aria-hidden="true"><path d="M${lx} ${ly} L${ax} ${ay}"/><circle class="cw-anchor-hit" cx="${ax}" cy="${ay}" r="15"/><circle class="cw-anchor-pin" cx="${ax}" cy="${ay}" r="4"/></g>`;
    }).join("");
    this.element.querySelector(".cw-gun")!.innerHTML = `<g class="cw-gun-body" transform="translate(180 180) rotate(-30) scale(${scale}) translate(${-cx} ${-cy})">${markup(shape, `cw-weapon-${iconSerial++}`)}</g>${lines}`;
    const model = this.element.querySelector<HTMLElement>(".cw-model")!;
    const real = weapon.id === "rifle-varyag";
    this.element.querySelector(".cw-stage")!.classList.toggle("cw-stage-3d", real);
    this.element.querySelector(".cw-gun")!.toggleAttribute("hidden", real);
    this.element.querySelector(".cw-links")!.toggleAttribute("hidden", !real);
    model.hidden = !real;
    if (real) {
      this.viewer ??= new ItemViewer(model, { workbench: true, onSockets: points => this.drawSocketLinks(points) });
      void this.viewer.show(weapon.id, weapon.name, this.loadout);
    } else this.viewer?.hide();
    for (const slot of MOD_SLOTS) {
      const button = this.element.querySelector<HTMLButtonElement>(`[data-socket="${slot}"]`)!;
      const item = ITEM_BY_ID.get(parts[slot]);
      button.querySelector(".cw-socket-name")!.textContent = this.slotName(slot);
      button.setAttribute("aria-label", `${this.slotName(slot)}: ${item?.name ?? "штатная деталь"}`);
      button.setAttribute("aria-haspopup", "dialog");
      button.setAttribute("aria-expanded", String(this.sheet.open && this.selectedSlot === slot));
      button.classList.toggle("cw-modified", parts[slot] !== weapon.stock[slot]);
      button.title = item?.name ?? "";
    }
  }

  private drawSocketLinks(points: Partial<Record<ModSlot, [number, number]>>): void {
    const stage = this.element.querySelector<HTMLElement>(".cw-stage")!;
    const links = this.element.querySelector<SVGSVGElement>(".cw-links")!;
    const w = stage.clientWidth, h = stage.clientHeight;
    links.setAttribute("viewBox", `0 0 ${w} ${h}`);
    links.innerHTML = MOD_SLOTS.map(slot => {
      const p = points[slot]; if (!p) return "";
      const [lx, ly] = labelPoints[slot];
      return `<g class="cw-anchor" data-anchor="${slot}"><path d="M${lx / 360 * w} ${ly / 360 * h} L${p[0]} ${p[1]}"/><circle class="cw-anchor-hit" cx="${p[0]}" cy="${p[1]}" r="18"/><circle class="cw-anchor-pin" cx="${p[0]}" cy="${p[1]}" r="4"/></g>`;
    }).join("");
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
    const selected = effectiveWeaponModId(this.loadout, slot);
    this.sheetBody.innerHTML = `<div class="cw-options">${weaponModsForSlot(this.loadout, slot).filter(availableInFitting).map(item => `<button class="cw-option glass" type="button" data-mod="${item.id}" aria-pressed="${item.id === selected}"><span class="cw-part-icon">${partIcon(item.id)}</span><strong>${escape(item.name)}</strong>${item.id === selected ? equippedCheck : ""}</button>`).join("")}</div>`;
    this.openSheet(this.slotName(slot));
    this.element.querySelector(`[data-socket="${slot}"]`)!.setAttribute("aria-expanded", "true");
  }

  private showWeapons(): void {
    if (!this.loadout) return;
    const weapons = itemsForSlot("weapon1").filter(availableInFitting).filter((item): item is WeaponItem => item.kind === "weapon");
    this.sheetBody.innerHTML = `<div class="cw-options">${weapons.map(item => `<button class="cw-option glass" type="button" data-weapon="${item.id}" aria-pressed="${item.id === this.loadout!.weapon1.item}"><span class="cw-part-icon">${equipmentItemIcon(item.id)}</span><strong>${escape(item.name)}</strong>${item.id === this.loadout!.weapon1.item ? equippedCheck : ""}</button>`).join("")}</div>`;
    this.openSheet("Сменить оружие");
  }

  private showStats(): void {
    if (!this.loadout) return;
    const kit = compileKit(this.loadout).weapon;
    const damage = (BASE_STATS.personal + kit.power * BASE_STATS.mastery) * kit.falloff[6]!;
    const stats = [
      ["Базовый урон в корпус", damage.toLocaleString("ru-RU", { maximumFractionDigits: 1 })],
      [kit.needsStance ? "Точность из стойки" : "Точность оружия", `${Math.round(kit.accuracy[6]! * 100)}%`],
      ["Пробитие брони", String(kit.flatPen)],
      ["Шанс критического попадания", `${Math.round((BASE_STATS.critChance + kit.crit) * 100)}%`],
    ];
    const mods = MOD_SLOTS.map(slot => ITEM_BY_ID.get(effectiveWeaponModId(this.loadout!, slot))!).filter(item => describeItem(item).length);
    this.sheetBody.innerHTML = `<p class="cw-stat-context">7 шагов. Обычное попадание без брони, щита и влияния погоды.</p><dl class="cw-stat-list">${stats.map(([name, value]) => `<div><dt>${name}</dt><dd>${value}</dd></div>`).join("")}</dl>${mods.length ? `<div class="cw-effects"><h3>Установленные модификации</h3>${mods.map(item => `<p><strong>${escape(item.name)}</strong><span>${describeItem(item).map(escape).join(" · ")}</span></p>`).join("")}</div>` : ""}`;
    this.openSheet("Характеристики");
  }
}
