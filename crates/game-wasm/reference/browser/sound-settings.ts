// The fight menu's sound settings on the base: a round button in the top right corner opens
// a small sheet with the sound switch and the two volume sliders. Same audio as the fight.

import { gameAudio, type VolumeKind } from "../audio/arena-audio.ts";
import { liquidGlass } from "../hud/liquid.ts";
import { QUALITY, QUALITY_IDS, keepQuality, type QualityId } from "../arena/quality.ts";
import { store } from "../store.ts";

const CAMERAS: [string, string][] = [["fp", "От первого лица"], ["back", "Из-за плеча"]];

const gear = `<svg class="cd-set-ico" viewBox="0 0 24 24" aria-hidden="true"><path d="M4 7h10M18 7h2M4 17h4M12 17h8"/><circle cx="16" cy="7" r="2.2"/><circle cx="10" cy="17" r="2.2"/></svg>`;

export function mountSoundSettings(): void {
  const btn = document.createElement("button");
  btn.type = "button";
  btn.className = "glass cd-settings";
  btn.setAttribute("aria-label", "Настройки звука");
  btn.innerHTML = gear;
  const dlg = document.createElement("dialog");
  dlg.className = "cd-dialog sheet glass cd-sound";
  dlg.innerHTML = `
    <header class="cd-dialog-header"><div><p class="cd-overline">НАСТРОЙКИ</p><h2>Звук</h2></div><button class="glass cd-dialog-close" type="button" aria-label="Закрыть">×</button></header>
    <div class="cd-sound-body">
      <button type="button" class="glass cd-sound-toggle"><span>Звук и музыка</span><b></b></button>
      <label class="vrow"><span>Звуки</span><input type="range" min="0" max="100" step="1" data-k="fx"><output></output></label>
      <label class="vrow"><span>Музыка в меню</span><input type="range" min="0" max="100" step="1" data-k="music"><output></output></label>
      <label class="vrow"><span>Музыка боя</span><input type="range" min="0" max="100" step="1" data-k="fight"><output></output></label>
      <p class="cd-set-label">Камера</p><div class="cd-set-seg" data-g="cam"></div>
      <p class="cd-set-label">Графика</p><div class="cd-set-seg" data-g="gfx"></div>
    </div>`;
  document.body.append(btn, dlg);
  const segs = (g: string, list: [string, string][], now: string, pick: (v: string) => void) => {
    const box = dlg.querySelector<HTMLElement>(`[data-g=${g}]`)!;
    box.replaceChildren(...list.map(([v, name]) => {
      const b = document.createElement("button");
      b.type = "button"; b.className = "glass"; b.textContent = name;
      b.setAttribute("aria-pressed", String(v === now));
      b.addEventListener("click", () => { pick(v); draw(); });
      return b;
    }));
  };
  const toggle = dlg.querySelector<HTMLButtonElement>(".cd-sound-toggle")!;
  const draw = () => {
    toggle.querySelector("b")!.textContent = gameAudio.on ? "Вкл" : "Выкл";
    toggle.setAttribute("aria-pressed", String(gameAudio.on));
    for (const input of dlg.querySelectorAll<HTMLInputElement>("input[data-k]")) {
      const v = Math.round(gameAudio.volume[input.dataset.k as VolumeKind] * 100);
      input.value = String(v); input.style.setProperty("--v", v + "%");
      input.nextElementSibling!.textContent = String(v);
    }
    segs("cam", CAMERAS, store.get("cd-camera") === "back" ? "back" : "fp", (v) => store.set("cd-camera", v));
    segs("gfx", QUALITY_IDS.map((id) => [id, QUALITY[id].name]), store.get("cd-quality") ?? "", (v) => keepQuality(v as QualityId));
  };
  for (const input of dlg.querySelectorAll<HTMLInputElement>("input[data-k]")) {
    input.addEventListener("input", () => { gameAudio.unlock(); gameAudio.setVolume(input.dataset.k as VolumeKind, Number(input.value) / 100); draw(); });
  }
  toggle.addEventListener("click", () => { gameAudio.toggle(); draw(); });
  btn.addEventListener("click", () => { gameAudio.unlock(); draw(); dlg.showModal(); liquidGlass(dlg); });
  dlg.querySelector(".cd-dialog-close")!.addEventListener("click", () => dlg.close());
  dlg.addEventListener("click", (e) => { if (e.target === dlg) dlg.close(); });
}

/** Only on the base screens: the fight has its own menu. */
export function showSoundSettings(on: boolean): void {
  const btn = document.querySelector<HTMLElement>(".cd-settings");
  if (!btn) return;
  btn.hidden = !on;
  // the icon lives in the game's frame, not in the browser's corner
  const home = document.querySelector(".cd-home");
  if (on && home && btn.parentElement !== home) home.append(btn);
}
