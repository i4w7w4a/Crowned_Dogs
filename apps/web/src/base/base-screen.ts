import { ITEM_BY_ID, type Loadout, type GearSlot } from "@crowned-dogs/core";
import { equipmentIcon, equipmentItemIcon, fittingRoomIcon } from "./equipment-icons.ts";
import {
  GEAR_SLOT_NAMES, GEAR_SLOTS, compileKit, describeItem, equipItem, equippedItemId,
  itemsForSlot, loadLocalLoadout, saveLocalLoadout,
} from "./loadout-state.ts";
import { PersonaViewer } from "./persona-viewer.ts";
import { WeaponScreen } from "./weapon-screen.ts";
import { liquidGlass } from "../hud/liquid.ts";
import { GlassScroll } from "./glass-scroll.ts";

export type BaseView = "base" | "fitting" | "weapon";
const escape = (s: string) => s.replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c]!));
const equippedCheck = '<span class="cd-equipped-check" aria-hidden="true"><svg viewBox="0 0 24 24"><path d="m5 12 4 4L19 6"/></svg></span>';
const asset = (name: string) => `${import.meta.env.BASE_URL}img/base/${name}`;
const columns: GearSlot[][] = [["head", "body", "weapon1", "knife"], ["suit", "boots", "weapon2", "grenade"]];

/** Owns the local base and fitting room; battle state stays in the existing client. */
export class BaseScreen {
  private loadout = loadLocalLoadout();
  private view: BaseView = "base";
  private active = true;
  private selectedSlot: GearSlot = "body";
  private weaponReturn: { slot: GearSlot; scroll: number } | null = null;
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
  private catalogScroll: GlassScroll;
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
          <button class="go glass cd-primary cd-enter" type="button"><span>В бой<small>Дуэль с ботом</small></span></button>
          <button class="glass cd-open-fitting" type="button"><span class="cd-menu-icon">${fittingRoomIcon()}</span><span>Примерочная<small>Боец и снаряжение</small></span></button>
          <p class="cd-home-message" aria-live="polite"></p>
        </div>
      </section>
      <section class="cd-fitting" aria-label="Примерочная" hidden>
        <img class="cd-backdrop" src="${asset("fitting-room-v2.png")}" alt="Оборудованная ниша примерочной в соляной шахте" draggable="false">
        <div class="cd-fit-shade" aria-hidden="true"></div>
        <header class="cd-fit-header"><button class="glass cd-back" type="button" aria-label="Вернуться на базу">Назад</button><h1>Примерочная</h1><span class="glass cd-3d-badge">3D</span></header>
        <div class="cd-model-stage">
          <canvas class="cd-persona-canvas" tabindex="0" role="img" aria-label="Персона на вращающемся постаменте. Перетаскивайте или используйте клавиши влево и вправо, чтобы повернуть."></canvas>
          <div class="glass cd-model-status" role="status">Загрузка Персоны…</div>
          ${columns.map((slots, i) => `<div class="cd-slot-column cd-slot-column-${i}" aria-label="${i ? "Костюм и снаряжение" : "Защита и оружие"}">${slots.map((s) => `<button class="glass cd-slot" type="button" data-slot="${s}"><span class="cd-slot-art"></span><span class="cd-slot-label">${GEAR_SLOT_NAMES[s]}</span></button>`).join("")}</div>`).join("")}
        </div>
        <footer class="cd-fit-footer">
          <p class="cd-rotation">Проведи, чтобы повернуть</p>
          <div class="cd-stats" aria-label="Защита выбранного комплекта"></div>
          <button class="go glass cd-primary cd-open-catalog" type="button">Каталог снаряжения</button>
          <p class="cd-save-status" aria-live="polite" hidden></p>
        </footer>
      </section>
      <dialog class="sheet glass cd-dialog" aria-labelledby="cd-catalog-title">
        <header class="cd-dialog-header"><div><p class="cd-overline">КАТАЛОГ СНАРЯЖЕНИЯ</p><h2 id="cd-catalog-title"></h2></div><button class="glass cd-dialog-close" type="button" aria-label="Закрыть каталог">×</button></header>
        <div class="cd-catalog-toolbar">
          <div class="cd-slot-picker"><span class="cd-picker-label">Слот</span><button class="glass cd-select-trigger" type="button" aria-label="Категория снаряжения" aria-haspopup="listbox" aria-controls="cd-slot-options" aria-expanded="false"></button>
            <div id="cd-slot-options" class="sheet glass cd-select-options" role="listbox" aria-label="Слоты снаряжения" hidden>${GEAR_SLOTS.map((s) => `<button class="glass" type="button" role="option" data-category="${s}" aria-selected="false">${GEAR_SLOT_NAMES[s]}</button>`).join("")}</div>
          </div>
        </div>
        <div class="cd-dialog-content">
          <p class="cd-catalog-error" role="alert" hidden></p>
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
    this.catalogScroll = new GlassScroll(root.querySelector<HTMLElement>(".cd-dialog-content")!);
    root.querySelector(".cd-open-fitting")!.addEventListener("click", () => this.navigate("fitting"));
    root.querySelector(".cd-back")!.addEventListener("click", () => this.navigate("base"));
    root.querySelector(".cd-enter")!.addEventListener("click", () => { void this.enterBattle(); });
    root.querySelector(".cd-open-catalog")!.addEventListener("click", () => this.openCatalog(this.selectedSlot));
    for (const b of root.querySelectorAll<HTMLButtonElement>("[data-slot]")) b.addEventListener("click", () => this.openCatalog(b.dataset.slot as GearSlot));
    root.querySelector(".cd-dialog-close")!.addEventListener("click", () => this.dialog.close());
    this.dialog.addEventListener("click", (e) => { if (e.target === this.dialog) { const r = this.dialog.getBoundingClientRect(); if (e.clientX < r.left || e.clientX > r.right || e.clientY < r.top || e.clientY > r.bottom) this.dialog.close(); } });
    this.dialog.addEventListener("close", () => { this.setPickerOpen(false); if (this.view === "fitting") this.origin?.focus(); });
    const picker = root.querySelector<HTMLElement>(".cd-slot-picker")!;
    root.querySelector(".cd-select-trigger")!.addEventListener("click", () => this.setPickerOpen(root.querySelector<HTMLElement>(".cd-select-options")!.hidden));
    picker.addEventListener("click", e => {
      const option = (e.target as Element).closest<HTMLElement>("[data-category]");
      if (!option) return;
      this.setPickerOpen(false);
      const slot = option.dataset.category as GearSlot;
      if (slot === "weapon1") {
        this.weaponReturn = { slot: this.selectedSlot, scroll: this.catalog.parentElement!.scrollTop };
        this.dialog.close(); this.navigate("weapon");
      } else {
        this.selectedSlot = slot; this.renderCatalog();
        root.querySelector<HTMLButtonElement>(".cd-select-trigger")!.focus();
      }
    });
    picker.addEventListener("keydown", e => this.pickerKey(e));
    this.dialog.addEventListener("click", e => { if (!picker.contains(e.target as Node)) this.setPickerOpen(false); });
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
      } catch (error) { const message = root.querySelector<HTMLElement>(".cd-catalog-error")!; message.hidden = false; message.textContent = error instanceof Error ? error.message : "Не удалось выбрать предмет"; }
    });
    this.canvas.addEventListener("keydown", (e) => { if (e.key === "ArrowLeft" || e.key === "ArrowRight") { e.preventDefault(); this.viewer?.rotate(e.key === "ArrowLeft" ? -.2 : .2); } });
    this.canvas.addEventListener("persona-preview-error", (event) => {
      const detail: unknown = (event as CustomEvent<unknown>).detail;
      this.status.textContent = typeof detail === "string" ? detail : "3D-просмотр недоступен. Каталог снаряжения доступен.";
      this.status.hidden = false;
    });
    this.canvas.addEventListener("webglcontextrestored", () => { if (this.modelReady) this.status.hidden = true; });
    this.observer = new ResizeObserver(() => this.resize()); this.observer.observe(this.stage);
    this.workshop = new WeaponScreen(root, () => {
      const returnTo = this.weaponReturn; this.weaponReturn = null;
      this.navigate("fitting");
      if (returnTo) { this.root.querySelector<HTMLButtonElement>('.cd-open-catalog')!.focus(); this.openCatalog(returnTo.slot); this.catalog.parentElement!.scrollTop = returnTo.scroll; }
      else this.root.querySelector<HTMLButtonElement>('[data-slot="weapon1"]')!.focus({ preventScroll: true });
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
      button.querySelector(".cd-slot-art")!.innerHTML = equipmentIcon(slot, id);
      button.setAttribute("aria-label", `${GEAR_SLOT_NAMES[slot]}: ${item?.name ?? "пусто"}. Выбрать предмет`);
      button.title = item?.name ?? "Не экипировано";
      button.classList.toggle("cd-slot-empty", id === null);
    }
    const kit = compileKit(this.loadout);
    this.root.querySelector(".cd-stats")!.innerHTML = `<span>Голова <b>${kit.armor.head}</b></span><span>Корпус <b>${kit.armor.body}</b></span><span>Ноги <b>${kit.armor.legs}</b></span>`;
    this.viewer?.updateLoadout(this.loadout);
  }

  private openCatalog(slot: GearSlot): void {
    if (slot === "weapon1") { this.weaponReturn = null; this.navigate("weapon"); return; }
    this.selectedSlot = slot; this.origin = document.activeElement as HTMLElement;
    this.renderCatalog(); this.dialog.showModal();
    this.catalog.parentElement!.scrollTop = 0;
    this.catalogScroll.refresh();
    this.root.querySelector<HTMLButtonElement>(".cd-dialog-close")!.focus({ preventScroll: true });
  }

  private renderCatalog(): void {
    this.root.querySelector("#cd-catalog-title")!.textContent = GEAR_SLOT_NAMES[this.selectedSlot];
    this.root.querySelector(".cd-select-trigger")!.textContent = GEAR_SLOT_NAMES[this.selectedSlot];
    this.root.querySelectorAll<HTMLElement>("[data-category]").forEach(option => {
      option.setAttribute("aria-selected", String(option.dataset.category === this.selectedSlot));
      option.classList.toggle("on", option.dataset.category === this.selectedSlot);
      option.tabIndex = -1;
    });
    this.root.querySelector<HTMLElement>(".cd-catalog-error")!.hidden = true;
    const selected = equippedItemId(this.loadout, this.selectedSlot);
    const empty = this.selectedSlot === "weapon2" ? `<button class="glass cd-item-card" type="button" data-item="" aria-pressed="${selected === null}"><span class="cd-item-art" aria-hidden="true">—</span><span class="cd-item-copy"><strong>Без второго оружия</strong></span>${selected === null ? equippedCheck : ""}</button>` : "";
    this.catalog.innerHTML = empty + itemsForSlot(this.selectedSlot).map((item) => {
      const effects = describeItem(item);
      return `<button class="glass cd-item-card" type="button" data-item="${item.id}" aria-pressed="${selected === item.id}"><span class="cd-item-art">${equipmentItemIcon(item.id)}</span><span class="cd-item-copy"><strong>${escape(item.name)}</strong><small>${escape(item.note)}</small>${effects.length ? `<span class="cd-effects">${effects.map(escape).join(" · ")}</span>` : ""}</span>${selected === item.id ? equippedCheck : ""}</button>`;
    }).join("");
    this.catalog.parentElement!.scrollTop = 0;
    liquidGlass(this.catalog);
  }

  private setPickerOpen(open: boolean): void {
    const options = this.root.querySelector<HTMLElement>(".cd-select-options")!;
    options.hidden = !open;
    this.root.querySelector(".cd-select-trigger")!.setAttribute("aria-expanded", String(open));
    if (open) {
      liquidGlass(this.dialog);
      options.querySelector<HTMLElement>(`[data-category="${this.selectedSlot}"]`)?.focus();
    }
  }

  private pickerKey(event: KeyboardEvent): void {
    const options = this.root.querySelector<HTMLElement>(".cd-select-options")!;
    if (event.key === "Escape" && !options.hidden) {
      event.preventDefault(); event.stopPropagation(); this.setPickerOpen(false);
      this.root.querySelector<HTMLButtonElement>(".cd-select-trigger")!.focus(); return;
    }
    if (event.key === "Tab" && !options.hidden) { this.root.querySelector<HTMLButtonElement>(".cd-select-trigger")!.focus(); this.setPickerOpen(false); return; }
    if (!["ArrowDown", "ArrowUp", "Home", "End"].includes(event.key)) return;
    event.preventDefault();
    if (options.hidden) { this.setPickerOpen(true); return; }
    const items = [...options.querySelectorAll<HTMLButtonElement>("[data-category]")];
    const index = items.indexOf(document.activeElement as HTMLButtonElement);
    const next = event.key === "Home" ? 0 : event.key === "End" ? items.length - 1 : (index + (event.key === "ArrowDown" ? 1 : -1) + items.length) % items.length;
    items[next]?.focus();
  }

  private async enterBattle(): Promise<void> {
    const button = this.root.querySelector<HTMLButtonElement>(".cd-enter")!;
    button.disabled = true;
    try { await this.onBattle(this.loadout); }
    catch { this.setActive(true); this.root.querySelector(".cd-home-message")!.textContent = "Не удалось открыть бой. Попробуйте ещё раз."; }
    finally { button.disabled = false; }
  }

  dispose(): void { this.observer.disconnect(); this.catalogScroll.dispose(); this.viewer?.dispose(); this.workshop.dispose(); this.root.remove(); }
}
