// The fight log: the last turn in one line at the foot of the scene (the ticker),
// and the whole record in a sheet that slides up over it.

import type { Side, TurnReport, WeaponId } from "@crowned-dogs/core";
import { turnLine, type Line } from "./words.ts";

export const icon = (id: string, cls = "") =>
  `<svg class="ico${cls ? " " + cls : ""}" aria-hidden="true"><use href="#${id}"/></svg>`;

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
    const lines = ([0, 1] as Side[]).map((s) => turnLine(r, s, inHand[s]));
    if (this.count++ === 0) this.list.innerHTML = "";
    const li = document.createElement("li");
    li.className = "entry";
    const dist = r.distanceBefore === r.distanceAfter ? `${r.distanceAfter}` : `${r.distanceBefore} → ${r.distanceAfter}`;
    li.innerHTML = `<div class="eh"><span>Ход ${r.turn}</span><span>дистанция ${dist}</span></div>`
      + lineHTML("me", "Ты", lines[0]!) + lineHTML("en", names[1], lines[1]!);
    this.list.appendChild(li);
    this.list.scrollTop = this.list.scrollHeight;
    this.ticker.innerHTML = `<b>Ход ${r.turn}</b>Ты: ${shortResult(lines[0]!, true)} · ${names[1]}: ${shortResult(lines[1]!, false)}`;
    this.button.classList.remove("fresh");
    void this.button.offsetWidth;
    this.button.classList.add("fresh");
  }

  /** A short note in the ticker, e.g. why a tap was refused. */
  note(text: string): void {
    this.ticker.textContent = text;
  }
}

function lineHTML(side: "me" | "en", who: string, l: Line): string {
  const ic = l.icon === "move" ? icon(l.dash ? "i-mv-3" : "i-mv-2", l.back ? "flip" : "")
    : icon(l.icon === "shot" ? "i-shot" : "i-" + l.icon);
  const o = l.outcome;
  const res = o.kind === "none" ? `<span class="r nil">${o.text}</span>`
    : o.kind === "miss" ? '<span class="r nil">мимо</span>'
    : o.kind === "blocked" ? `<span class="r blk">${icon("i-zont")}−${o.damage}</span>`
    : `<span class="r dmg">${o.crit ? icon("i-crit") : ""}−${o.damage}</span>`;
  return `<div class="ln ${side}">${ic}<span class="w">${who}</span><span class="a">${l.act || "на месте"}</span>${res}</div>`;
}

/** What one side did, short: for the line at the foot of the scene. */
function shortResult(l: Line, mine: boolean): string {
  const o = l.outcome;
  if (o.kind === "none") return l.icon === "swap" ? "смена оружия" : l.act || "на месте";
  if (o.kind === "miss") return `<em class="nil">${l.icon === "grenade" ? "граната мимо" : "промах"}</em>`;
  if (o.kind === "blocked") return `<em class="blk">Зонт −${o.damage}</em>`;
  return `<em class="${mine ? "us" : "them"}">${l.target} −${o.damage}${o.crit ? icon("i-crit") : ""}</em>`;
}
