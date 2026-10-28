import { ITEM_BY_ID, weaponItem, type Loadout, type GearSlot } from "@crowned-dogs/core";
import { itemArt, svg, type WeaponArtId, type Slot } from "../../../../packages/icons/src/index.ts";
import {
  GEAR_SLOT_NAMES, GEAR_SLOTS, compileKit, describeItem, equipItem, equippedItemId,
  itemsForSlot, loadLocalLoadout, saveLocalLoadout,
} from "./loadout-state.ts";
import { PersonaViewer } from "./persona-viewer.ts";
import { WeaponScreen } from "./weapon-screen.ts";
import { liquidGlass } from "../hud/liquid.ts";

export type BaseView = "base" | "fitting" | "weapon";
const escape = (s: string) => s.replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c]!));
const artParts = Object.fromEntries((["pistol", "shotgun", "rifle", "sniper"] as const).map((w) => [w, weaponItem(w).stock])) as Record<WeaponArtId, Record<Slot, string>>;
let artIndex = 0;
function itemIcon(id: string | null): string {
  const shape = itemArt(id ?? "pistol-pustelga", artParts);
  return shape ? svg(shape, `base-${artIndex++}`, 7).replace("<svg ", '<svg aria-hidden="true" focusable="false" ') : "";
}
const chevron = '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true"><path d="m14 5-7 7 7 7"/></svg>';
const arrow = '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true"><path d="M4 12h15m-6-6 6 6-6 6"/></svg>';
const rotateIcon = '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true"><path d="M18 7a8 8 0 1 0 2 9M18 2v6h-6"/></svg>';
const asset = (name: string) => `${import.meta.env.BASE_URL}img/base/${name}`;
const columns: GearSlot[][] = [["head", "body", "weapon1", "knife"], ["suit", "boots", "weapon2", "grenade"]];

/** Owns the local base and fitting room; battle state stays in the existing client. */
export class BaseScreen {
  private loadout = loadLocalLoadout();
  private view: BaseView = "base";
  private active = true;
  private selectedSlot: GearSlot = "body";
  private viewer: PersonaViewer | null = null;
  private workshop: WeaponScreen;
  private loadAttempted = false;
  private modelReady = false;
  private origin: HTMLElement | null = null;
  private observer: ResizeObserver;
  private home: HTMLElement;
  private fitting: HTMLElement;
  private stage: HTMLElement;
  private canvas: HTMLCanvasElement;
  private dialog: HTMLDialogElement;
  private catalog: HTMLElement;
  private status: HTMLElement;
  private root: HTMLElement;
  private onBattle: (loadout: Loadout) => Promise<void>;

  constructor(root: HTMLElement, onBattle: (loadout: Loadout) => Promise<void>) {
    this.root = root;
    this.onBattle = onBattle;
    root.className = "cd-shell";
    root.innerHTML = `
      <section class="cd-home" aria-label="База Соль">
        <img class="cd-backdrop" src="${asset("salt-base.webp")}" alt="Убежище в соляной шахте: Персоны в ложементах, капсула оператора и узел очистки воды" draggable="false">
        <div class="cd-shade" aria-hidden="true"></div>
        <header class="cd-home-header"><div class="cd-wordmark">CROWNED<span>DOGS</span></div><span class="cd-edition">БАЗА ОПЕРАТОРА</span></header>
        <div class="cd-home-title"><p class="cd-overline">СОЛЯНАЯ ШАХТА</p><h1>СОЛЬ</h1><p>Здесь начинается следующий выход.</p></div>
        <div class="cd-home-bottom">
          <div class="cd-operator"><span class="cd-overline">ОПЕРАТОР</span><span class="cd-tag">K-417-093</span></div>
          <div class="cd-current-kit"><span>Основное оружие</span><strong data-kit-name></strong></div>
          <button class="go glass cd-primary cd-enter" type="button"><span>В бой<small>Дуэль с ботом</small></span>${arrow}</button>
          <button class="glass cd-open-fitting" type="button"><span class="cd-menu-icon">${itemIcon("body-plita2")}</span><span>Примерочная<small>Боец и снаряжение</small></span>${arrow}</button>
          <p class="cd-home-message" aria-live="polite"></p>
        </div>
      </section>
      <section class="cd-fitting" aria-label="Примерочная" hidden>
        <img class="cd-backdrop" src="${asset("fitting-room.webp")}" alt="Оборудованная ниша примерочной в соляной шахте" draggable="false">
        <div class="cd-fit-shade" aria-hidden="true"></div>
        <header class="cd-fit-header"><button class="glass cd-back" type="button" aria-label="Вернуться на базу">${chevron}</button><h1>Примерочная</h1><span class="glass cd-3d-badge">3D</span></header>
        <div class="cd-model-stage">
          <canvas class="cd-persona-canvas" tabindex="0" role="img" aria-label="Трёхмерная Персона в снаряжении. Перетаскивайте или используйте стрелки влево и вправо, чтобы повернуть."></canvas>
          <div class="glass cd-model-status" role="status">Загрузка Персоны…</div>
          ${columns.map((slots, i) => `<div class="cd-slot-column cd-slot-column-${i}" aria-label="${i ? "Костюм и снаряжение" : "Защита и оружие"}">${slots.map((s) => `<button class="glass cd-slot" type="button" data-slot="${s}"><span class="cd-slot-art"></span><span class="cd-slot-label">${GEAR_SLOT_NAMES[s]}</span></button>`).join("")}</div>`).join("")}
        </div>
        <footer class="cd-fit-footer">
          <div class="cd-rotation"><button class="glass cd-rotate-left" type="button" aria-label="Повернуть бойца влево">${chevron}</button><span>${rotateIcon} Повернуть</span><button class="glass cd-rotate-right" type="button" aria-label="Повернуть бойца вправо">${chevron}</button></div>
          <div class="cd-stats" aria-label="Защита выбранного комплекта"></div>
          <button class="go glass cd-primary cd-open-catalog" type="button">Каталог снаряжения ${arrow}</button>
          <p class="cd-save-status" aria-live="polite" hidden></p>
        </footer>
      </section>
      <dialog class="sheet glass cd-dialog" aria-labelledby="cd-catalog-title">
        <header class="cd-dialog-header"><div><p class="cd-overline">КАТАЛОГ СНАРЯЖЕНИЯ</p><h2 id="cd-catalog-title"></h2></div><button class="glass cd-dialog-close" type="button" aria-label="Закрыть каталог">×</button></header>
        <div class="cd-dialog-content">
          <label class="cd-slot-picker">Слот<select class="glass">${GEAR_SLOTS.map((s) => `<option value="${s}">${GEAR_SLOT_NAMES[s]}</option>`).join("")}</select></label>
          <p class="cd-catalog-note">Предметы для локальной примерки. Это каталог, а не серверный инвентарь.</p>
          <div class="cd-catalog-items"></div>
        </div>
      </dialog>`;
    this.home = root.querySelector(".cd-home")!;
    this.fitting = root.querySelector(".cd-fitting")!;
    this.stage = root.querySelector(".cd-model-stage")!;
    this.canvas = root.querySelector("canvas")!;
    this.dialog = root.querySelector("dialog")!;
    this.catalog = root.querySelector(".cd-catalog-items")!;
    this.status = root.querySelector(".cd-model-status")!;
    root.querySelector(".cd-open-fitting")!.addEventListener("click", () => this.navigate("fitting"));
    root.querySelector(".cd-back")!.addEventListener("click", () => this.navigate("base"));
    root.querySelector(".cd-enter")!.addEventListener("click", () => { void this.enterBattle(); });
    root.querySelector(".cd-open-catalog")!.addEventListener("click", () => this.openCatalog(this.selectedSlot));
    for (const b of root.querySelectorAll<HTMLButtonElement>("[data-slot]")) b.addEventListener("click", () => this.openCatalog(b.dataset.slot as GearSlot));
    root.querySelector(".cd-dialog-close")!.addEventListener("click", () => this.dialog.close());
    this.dialog.addEventListener("click", (e) => { if (e.target === this.dialog) { const r = this.dialog.getBoundingClientRect(); if (e.clientX < r.left || e.clientX > r.right || e.clientY < r.top || e.clientY > r.bottom) this.dialog.close(); } });
    this.dialog.addEventListener("close", () => { this.origin?.focus(); });
    root.querySelector("select")!.addEventListener("change", (e) => {
      this.selectedSlot = (e.target as HTMLSelectElement).value as GearSlot;
      if (this.selectedSlot === "weapon1") { this.dialog.close(); this.navigate("weapon"); }
      else this.renderCatalog();
    });
    this.catalog.addEventListener("click", (e) => {
      const b = (e.target as HTMLElement).closest<HTMLButtonElement>("[data-item]");
      if (!b) return;
      try {
        this.loadout = equipItem(this.loadout, this.selectedSlot, b.dataset.item || null);
        const saved = saveLocalLoadout(this.loadout);
        this.refresh(); this.dialog.close();
        const saveStatus = root.querySelector<HTMLElement>(".cd-save-status")!;
        saveStatus.hidden = saved;
        saveStatus.textContent = saved ? "" : "Не удалось сохранить комплект. Он будет доступен до закрытия страницы.";
      } catch (error) { root.querySelector(".cd-catalog-note")!.textContent = error instanceof Error ? error.message : "Не удалось выбрать предмет"; }
    });
    root.querySelector(".cd-rotate-left")!.addEventListener("click", () => this.viewer?.rotate(-Math.PI / 6));
    root.querySelector(".cd-rotate-right")!.addEventListener("click", () => this.viewer?.rotate(Math.PI / 6));
    this.canvas.addEventListener("keydown", (e) => { if (e.key === "ArrowLeft" || e.key === "ArrowRight") { e.preventDefault(); this.viewer?.rotate(e.key === "ArrowLeft" ? -.2 : .2); } });
    this.canvas.addEventListener("persona-preview-error", (event) => {
      const detail: unknown = (event as CustomEvent<unknown>).detail;
      this.status.textContent = typeof detail === "string" ? detail : "3D-просмотр недоступен. Каталог снаряжения доступен.";
      this.status.hidden = false;
    });
    this.canvas.addEventListener("webglcontextrestored", () => { if (this.modelReady) this.status.hidden = true; });
    this.observer = new ResizeObserver(() => this.resize()); this.observer.observe(this.stage);
    this.workshop = new WeaponScreen(root, () => {
      this.navigate("fitting");
      this.root.querySelector<HTMLButtonElement>('[data-slot="weapon1"]')!.focus({ preventScroll: true });
    }, loadout => {
      this.loadout = loadout;
      const saved = saveLocalLoadout(loadout);
      this.refresh();
      return saved;
    });
    this.refresh();
    liquidGlass(root);
  }

  private navigate(view: BaseView): void { history.replaceState(null, "", `#${view}`); this.show(view); }

  show(view: BaseView): void {
    if (this.dialog.open) this.dialog.close();
    this.view = view; this.home.hidden = view !== "base"; this.fitting.hidden = view !== "fitting";
    if (view === "weapon") this.workshop.show(this.loadout);
    else this.workshop.hide();
    if (view === "fitting") { this.ensureViewer(); this.resize(); }
    this.viewer?.setActive(this.active && view === "fitting");
  }

  setActive(active: boolean): void {
    this.active = active; this.root.hidden = !active;
    if (!active && this.dialog.open) this.dialog.close();
    if (!active && this.view === "weapon") this.workshop.hide();
    if (active && this.view === "weapon") this.workshop.show(this.loadout);
    this.viewer?.setActive(active && this.view === "fitting");
    if (active) this.resize();
  }

  private resize(): void {
    if (this.stage.clientWidth && this.stage.clientHeight) this.viewer?.resize(this.stage.clientWidth, this.stage.clientHeight);
  }

  private ensureViewer(): void {
    if (this.loadAttempted) return;
    this.loadAttempted = true;
    try {
      this.viewer = new PersonaViewer(this.canvas);
      this.viewer.updateLoadout(this.loadout);
      this.resize();
      void this.viewer.load().then(() => {
        this.modelReady = true; this.status.hidden = true; this.refresh(); this.resize();
        this.viewer?.setActive(this.active && this.view === "fitting");
      }).catch(() => {
        this.status.textContent = "3D-модель не загрузилась. Каталог снаряжения доступен.";
        this.viewer?.setActive(false);
      });
    } catch { this.status.textContent = "На этом устройстве 3D недоступно. Можно выбрать снаряжение в каталоге."; }
  }

  private refresh(): void {
    const mainName = ITEM_BY_ID.get(this.loadout.weapon1.item)?.name ?? "Оружие";
    this.root.querySelector("[data-kit-name]")!.textContent = mainName;
    for (const button of this.root.querySelectorAll<HTMLButtonElement>("[data-slot]")) {
      const slot = button.dataset.slot as GearSlot, id = equippedItemId(this.loadout, slot), item = id ? ITEM_BY_ID.get(id) : null;
      button.querySelector(".cd-slot-art")!.innerHTML = itemIcon(id);
      button.setAttribute("aria-label", `${GEAR_SLOT_NAMES[slot]}: ${item?.name ?? "пусто"}. Выбрать предмет`);
      button.title = item?.name ?? "Не экипировано";
      button.classList.toggle("cd-slot-empty", id === null);
    }
    const kit = compileKit(this.loadout);
    this.root.querySelector(".cd-stats")!.innerHTML = `<span>Голова <b>${kit.armor.head}</b></span><span>Корпус <b>${kit.armor.body}</b></span><span>Ноги <b>${kit.armor.legs}</b></span>`;
    this.viewer?.updateLoadout(this.loadout);
    for (const b of this.root.querySelectorAll<HTMLButtonElement>(".cd-rotation button")) b.disabled = !this.modelReady;
  }

  private openCatalog(slot: GearSlot): void {
    if (slot === "weapon1") { this.navigate("weapon"); return; }
    this.selectedSlot = slot; this.origin = document.activeElement as HTMLElement;
    this.renderCatalog(); this.dialog.showModal();
    this.catalog.parentElement!.scrollTop = 0;
    this.root.querySelector<HTMLButtonElement>(".cd-dialog-close")!.focus({ preventScroll: true });
  }

  private renderCatalog(): void {
    this.root.querySelector("#cd-catalog-title")!.textContent = GEAR_SLOT_NAMES[this.selectedSlot];
    this.root.querySelector("select")!.value = this.selectedSlot;
    const selected = equippedItemId(this.loadout, this.selectedSlot);
    const empty = this.selectedSlot === "weapon2" ? `<button class="glass cd-item-card" type="button" data-item="" aria-pressed="${selected === null}"><span class="cd-item-art" aria-hidden="true">—</span><span class="cd-item-copy"><strong>Без второго оружия</strong><small>Оставить слот пустым</small></span><span class="cd-item-state">${selected === null ? "Выбрано" : "Снять"}</span></button>` : "";
    this.catalog.innerHTML = empty + itemsForSlot(this.selectedSlot).map((item) => {
      const effects = describeItem(item);
      return `<button class="glass cd-item-card" type="button" data-item="${item.id}" aria-pressed="${selected === item.id}"><span class="cd-item-art">${itemIcon(item.id)}</span><span class="cd-item-copy"><strong>${escape(item.name)}</strong><small>${escape(item.note)}</small>${effects.length ? `<span class="cd-effects">${effects.map(escape).join(" · ")}</span>` : ""}</span><span class="cd-item-state">${selected === item.id ? "Выбрано" : "Надеть"}</span></button>`;
    }).join("");
    this.catalog.parentElement!.scrollTop = 0;
    liquidGlass(this.catalog);
  }

  private async enterBattle(): Promise<void> {
    const button = this.root.querySelector<HTMLButtonElement>(".cd-enter")!;
    button.disabled = true;
    try { await this.onBattle(this.loadout); }
    catch { this.setActive(true); this.root.querySelector(".cd-home-message")!.textContent = "Не удалось открыть бой. Попробуйте ещё раз."; }
    finally { button.disabled = false; }
  }

  dispose(): void { this.observer.disconnect(); this.viewer?.dispose(); this.workshop.dispose(); this.root.remove(); }
}
