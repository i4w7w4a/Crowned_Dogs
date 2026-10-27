// Turns a resolved turn into a short sound script. The rules resolve both
// attacks at once; on screen and in the ears our attack lands first, the
// enemy's a beat later, so the player can tell them apart.

import {
  DEFAULT_ARMOR, other,
  type Action, type AttackReport, type Side, type TurnReport, type WeaponId, type Zone,
} from "@crowned-dogs/core";
import { MIX, type SoundId } from "./bank.ts";

export interface Cue {
  sound: SoundId;
  /** Seconds after the turn starts playing. */
  at: number;
  /** Extra level in dB (distance, layering). */
  gainDb: number;
  /** Stereo position: our Persona is on the left of the over-the-shoulder view. */
  pan: number;
}

export interface TurnSoundContext {
  /** The side whose ears we are in. */
  viewer: Side;
  /** Armour per zone; a zone with armour above 0 rings like a plate when hit. */
  armor?: [Partial<Record<Zone, number>>, Partial<Record<Zone, number>>];
}

/** When each part of the turn plays, in seconds (the arena uses the same beats). */
export const TIMING = {
  move: 0,
  ourAttack: 0.85,
  enemyAttack: 1.45,
  stepGap: 0.27,
  dashStepGap: 0.19,
  knifeLands: 0.18,
  grenadeBounce: 0.45,
  grenadeBlast: 0.9,
  fallAfterLastAttack: 0.45,
} as const;

const PAN = { ours: -0.3, theirs: 0.15 } as const;

/** Everything a fight between these weapons can play, to preload when it starts. */
export function fightSounds(ours: WeaponId, theirs: WeaponId): SoundId[] {
  return [...new Set<SoundId>([
    `shot.${ours}.near`, `shot.${theirs}.far`, "casing",
    "knife.swing", "knife.hit", "knife.blocked",
    "hit.armor", "hit.suit", "hit.head", "hit.crit", "shield.block", "shield.break", "miss.near", "miss.far",
    "grenade.pin", "grenade.throw", "grenade.bounce", "grenade.blast",
    "step", "dash.whoosh", "gear.rustle", "body.fall", "persona.powerdown", "sync.lost",
  ])];
}

export function cuesForTurn(report: TurnReport, ctx: TurnSoundContext): Cue[] {
  const cues: Cue[] = [];
  const me = ctx.viewer;
  const them = other(me);
  // the enemy's own sounds fade with distance; ours stay close
  const far = -Math.min(MIX.distanceDbMax, Math.max(0, report.distanceAfter - 1) * MIX.distanceDbPerStep);
  const side = (s: Side) => (s === me ? { gainDb: 0, pan: PAN.ours } : { gainDb: far, pan: PAN.theirs });
  const add = (sound: SoundId, at: number, s: Side, extraDb = 0) => {
    const p = side(s);
    cues.push({ sound, at: round(at), gainDb: round(p.gainDb + extraDb), pan: p.pan });
  };

  for (const s of [me, them]) movement(report.actions[s], s, add);

  let lastHit: number = TIMING.ourAttack;
  for (const s of [me, them]) {
    const attack = report.attacks[s];
    if (!attack) continue;
    const t = s === me ? TIMING.ourAttack : TIMING.enemyAttack;
    lastHit = Math.max(lastHit, attackCues(attack, report.actions[s], s, t, ctx, add));
  }

  for (const s of [me, them]) {
    if (report.hp[s] > 0) continue;
    const t = lastHit + TIMING.fallAfterLastAttack;
    add("body.fall", t, s);
    add("persona.powerdown", t + 0.25, s);
    if (s === me) add("sync.lost", t + 0.6, s);
  }

  return cues.sort((a, b) => a.at - b.at);
}

type Add = (sound: SoundId, at: number, s: Side, extraDb?: number) => void;

function movement(action: Action, s: Side, add: Add) {
  const steps = (n: number, gap: number) => {
    for (let i = 0; i < n; i++) add("step", TIMING.move + i * gap, s);
    add("gear.rustle", TIMING.move, s);
  };
  switch (action.type) {
    case "run":
      steps(2, TIMING.stepGap);
      break;
    case "dash":
      add("dash.whoosh", TIMING.move, s);
      steps(3, TIMING.dashStepGap);
      break;
    case "flee":
      add("dash.whoosh", TIMING.move, s);
      steps(3, TIMING.stepGap);
      break;
    case "shoot":
    case "knife":
    case "grenade":
      if (action.move !== 0) steps(1, 0);
      break;
  }
}

/** Adds the attack and what it hit; returns when the last of it lands. */
function attackCues(attack: AttackReport, action: Action, s: Side, t: number, ctx: TurnSoundContext, add: Add): number {
  const ours = s === ctx.viewer;
  const target = other(s);

  if (attack.weapon === "grenade") {
    add("grenade.pin", t - 0.3, s, -2);
    add("grenade.throw", t, s);
    add("grenade.bounce", t + TIMING.grenadeBounce, target);
    add("grenade.blast", t + TIMING.grenadeBlast, target);
    if (attack.hit) add("hit.suit", t + TIMING.grenadeBlast + 0.05, target, -4);
    return t + TIMING.grenadeBlast;
  }

  if (attack.weapon === "knife") {
    add("knife.swing", t, s);
    const at = t + TIMING.knifeLands;
    if (!attack.hit) return t;
    add(attack.blocked ? "knife.blocked" : "knife.hit", at, target);
    if (attack.crit) add("hit.crit", at, target);
    return at;
  }

  const weapon = attack.weapon;
  add(ours ? `shot.${weapon}.near` : `shot.${weapon}.far`, t, s);
  if (ours && weapon !== "shotgun") add("casing", t + 0.55, s);
  // our shot lands across the hall a moment later; the enemy's lands on us at once
  const at = t + (ours ? 0.06 : 0.02);
  if (!attack.hit) {
    add(ours ? "miss.far" : "miss.near", at + 0.04, target);
    return at;
  }
  hitCues(attack, target, at, ctx, add);
  return at;
}

function hitCues(attack: AttackReport, target: Side, at: number, ctx: TurnSoundContext, add: Add) {
  const zone = attack.zone ?? "body";
  if (attack.blocked) {
    add(attack.shieldBroken ? "shield.break" : "shield.block", at, target);
    if (!attack.shieldBroken) return;
  }
  const armor = (ctx.armor?.[target] ?? DEFAULT_ARMOR)[zone] ?? DEFAULT_ARMOR[zone];
  const layer = attack.blocked ? -6 : 0;
  add(armor > 0 ? "hit.armor" : "hit.suit", at, target, layer);
  if (zone === "head") add("hit.head", at, target, layer);
  if (attack.crit) add("hit.crit", at + 0.02, target);
}

const round = (v: number) => Math.round(v * 1000) / 1000;
