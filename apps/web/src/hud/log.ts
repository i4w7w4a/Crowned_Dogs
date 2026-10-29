// The fight log: the last turn in one line at the foot of the scene (the ticker),
// and the whole record in a sheet that slides up over it.

import type { Side, TurnReport, WeaponId } from "@crowned-dogs/core";
import { callRust } from "../rust/engine.ts";

export const icon = (id: string, cls = ""): string => callRust("ui.icon", [id, cls]);

const EMPTY = '<li class="empty">Здесь появится каждый ход: кто куда стрелял и сколько снял.</li>';

export class FightLog {
  private count = 0;
  private list: HTMLElement;
  private ticker: HTMLElement;
  private button: HTMLElement;
  private sheet: HTMLElement;

  constructor(els: { list: HTMLElement; ticker: HTMLElement; button: HTMLElement; sheet: HTMLElement }) {
    ({ list: this.list, ticker: this.ticker, button: this.button, sheet: this.sheet } = els);
  }

  get open(): boolean {
    return !this.sheet.hidden;
  }

  setOpen(open: boolean): void {
    this.sheet.hidden = !open;
    this.button.setAttribute("aria-expanded", String(open));
    if (open) this.list.scrollTop = this.list.scrollHeight;
  }

  /** A new fight: an empty record and a line about who stands where. */
  reset(intro: string): void {
    this.count = 0;
    this.list.innerHTML = EMPTY;
    this.ticker.textContent = intro;
  }

  /** A turn has played out. `names` are ours and the enemy's; `inHand` the guns held after the turn. */
  add(r: TurnReport, names: [string, string], inHand: [WeaponId, WeaponId]): void {
    const view = callRust<{entry: string; ticker: string}>("ui.fightLog", [r, names, inHand]);
    if (this.count++ === 0) this.list.innerHTML = "";
    const li = document.createElement("li");
    li.className = "entry";
    li.innerHTML = view.entry;
    this.list.appendChild(li);
    this.list.scrollTop = this.list.scrollHeight;
    this.ticker.innerHTML = view.ticker;
    this.button.classList.remove("fresh");
    void this.button.offsetWidth;
    this.button.classList.add("fresh");
  }

  /** A short note in the ticker, e.g. why a tap was refused. */
  note(text: string): void {
    this.ticker.textContent = text;
  }
}
