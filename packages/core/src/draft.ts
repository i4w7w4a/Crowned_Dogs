// The order a player is putting together, one tap at a time, before pressing "Готово".
// Pure functions: the bot and the web client call them on every tap.

import {
  canFlee, canSwap, canThrowGrenade, canUseKnife, grenadeSpec, hitChance, plannedDistance, validateOrder,
  type Action, type Match, type Order, type Side,
} from "./match.ts";
import { DASH_STEPS, MAX_STEPS, RUN_STEPS, STANCE_TURNS, type Zone } from "./rules.ts";

export type DraftKind = "shoot" | "knife" | "grenade" | "swap" | "run" | "dash" | "flee";

export interface Draft {
  kind: DraftKind;
  /** Steps toward the enemy: -1..1 with an attack, ±2 for a run, 3 for a dash. */
  move: number;
  zone: Zone | null;
  defend: Zone[];
}

/** A button press, already decoded from the callback data. */
export type Command =
  | { type: "zone"; zone: Zone }
  | { type: "guard"; zone: Zone }
  | { type: "step"; move: -1 | 0 | 1 }
  | { type: "run"; move: -2 | 2 }
  | { type: "dash" }
  | { type: "knife" }
  | { type: "grenade" }
  | { type: "swap" }
  | { type: "flee" };

export function freshDraft(m: Match, side: Side): Draft {
  // Keep last turn's defence: most players guard the same way twice, it saves taps.
  return { kind: "shoot", move: 0, zone: null, defend: [...m.fighters[side].lastDefend].slice(0, 2) };
}

/** Why the grenade cannot be thrown now, in the words of a toast. */
export function grenadeRefusal(m: Match, side: Side): string {
  if (m.fighters[side].grenades <= 0) return "Гранаты больше нет";
  const g = grenadeSpec(m, side);
  return `Граната летит на ${g.minDistance}–${g.maxDistance} шагов`;
}

export function guardsNeeded(d: Draft): number {
  return d.kind === "dash" ? 1 : 2;
}

/** Applies one press. Returns the new draft, or an error text to show in a toast. */
export function applyCommand(m: Match, side: Side, d: Draft, c: Command): Draft | string {
  const next: Draft = { ...d, defend: [...d.defend] };
  switch (c.type) {
    case "zone":
      next.zone = d.zone === c.zone && (d.kind === "shoot" || d.kind === "knife") ? null : c.zone;
      if (next.kind !== "knife") next.kind = "shoot";
      if (Math.abs(next.move) > 1) next.move = 0;
      break;
    case "guard": {
      const i = next.defend.indexOf(c.zone);
      if (i >= 0) next.defend.splice(i, 1);
      else next.defend.push(c.zone);
      break;
    }
    case "step":
      if (next.kind === "run" || next.kind === "dash" || next.kind === "flee") next.kind = "shoot";
      next.move = c.move;
      if (next.kind === "knife" && !canUseKnife(m, next.move)) return "С такого шага нож не достанет";
      break;
    case "run":
      next.kind = d.kind === "run" && d.move === c.move ? "shoot" : "run";
      next.move = next.kind === "run" ? c.move : 0;
      break;
    case "dash":
      next.kind = d.kind === "dash" ? "shoot" : "dash";
      next.move = next.kind === "dash" ? DASH_STEPS : 0;
      break;
    case "knife":
      if (d.kind === "knife") { next.kind = "shoot"; break; }
      if (Math.abs(next.move) > 1) next.move = 0;
      if (!canUseKnife(m, next.move)) {
        // Try stepping in for the player before giving up.
        if (canUseKnife(m, 1)) next.move = 1;
        else return "Нож бьёт только с 2 шагов";
      }
      next.kind = "knife";
      break;
    case "grenade":
      if (d.kind === "grenade") { next.kind = "shoot"; break; }
      if (!canThrowGrenade(m, side)) return grenadeRefusal(m, side);
      next.kind = "grenade";
      if (Math.abs(next.move) > 1) next.move = 0;
      break;
    case "swap":
      if (d.kind === "swap") { next.kind = "shoot"; break; }
      if (!canSwap(m, side)) return "Второго оружия нет";
      next.kind = "swap";
      if (Math.abs(next.move) > 1) next.move = 0;
      break;
    case "flee":
      if (d.kind === "flee") { next.kind = "shoot"; next.move = 0; break; }
      if (!canFlee(m)) return "Уйти можно только с края карты";
      next.kind = "flee";
      next.move = 0;
      break;
  }
  // Too many guarded zones (after a dash, or one tap too many): drop the oldest.
  while (next.defend.length > guardsNeeded(next)) next.defend.shift();
  return next;
}

export function actionOf(d: Draft): Action | null {
  switch (d.kind) {
    case "shoot":
    case "knife":
      return d.zone ? { type: d.kind, move: d.move as -1 | 0 | 1, zone: d.zone } : null;
    case "grenade":
    case "swap":
      return { type: d.kind, move: d.move as -1 | 0 | 1 };
    case "run":
      return { type: "run", move: d.move === RUN_STEPS ? 2 : -2 };
    case "dash":
      return { type: "dash" };
    case "flee":
      return { type: "flee" };
  }
}

/** What is still missing, in the words of the "Готово" button. Null when the order is complete. */
export function missing(d: Draft): string | null {
  if (!actionOf(d)) return d.kind === "knife" ? "Выбери зону для ножа" : "Выбери зону атаки";
  const need = guardsNeeded(d) - d.defend.length;
  if (need > 0) return need === 2 ? "Выбери 2 зоны защиты" : "Выбери зону защиты";
  return null;
}

export function toOrder(m: Match, side: Side, d: Draft): Order | string {
  const gap = missing(d);
  if (gap) return gap;
  const order: Order = { action: actionOf(d)!, defend: [...d.defend] };
  const err = validateOrder(m, side, order);
  return err ? `Так нельзя: ${err}` : order;
}

/** Hit chance of the planned shot or knife if the enemy stands still. Null when nothing is aimed. */
export function draftChance(m: Match, side: Side, d: Draft): number | null {
  if (d.kind !== "shoot" && d.kind !== "knife") return null;
  const me = m.fighters[side];
  const dist = Math.min(plannedDistance(m, d.move), MAX_STEPS);
  const still = d.move === 0 ? Math.min(me.still + 1, STANCE_TURNS) : 0;
  return hitChance(m, side, d.kind === "knife" ? "knife" : me.weapon, dist, still, false);
}
