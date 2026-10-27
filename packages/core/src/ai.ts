// A simple opponent for fights against the bot and for balance runs.
// It looks one turn ahead: what it can deal, what it risks taking, and where
// the next turn would leave it. Then it picks with a bit of randomness so it
// is not fully predictable. It never sees the other side's secret order.

import {
  canFlee, canThrowGrenade, canUseKnife, defendCount, distanceLimit, expectedHit, hitChance, other,
  plannedDistance, validateOrder, type Action, type Match, type Order, type Side,
} from "./match.ts";
import { nextFloat, seedRng, type RngState } from "./rng.ts";
import {
  GUARD, KNIFE_GUARD, MAX_STEPS, STANCE_TURNS, ZONES, grenadeDamage, type AttackWeapon, type Zone,
} from "./rules.ts";

/** How the bot plays. Lower temperature = more predictable, closer to its best guess. */
export interface BotStyle {
  /** How much it cares about where the next turn leaves it, compared with damage now. */
  foresight: number;
  /** Fear of incoming damage when ahead / when behind on health. */
  caution: number;
  cautionBehind: number;
  temperature: number;
}

export const DEFAULT_BOT: BotStyle = { foresight: 1.2, caution: 0.85, cautionBehind: 1.2, temperature: 3 };

/** How players tend to aim before we know anything about them. */
const AIM_PRIOR: Record<Zone, number> = { head: 3, body: 3, groin: 2, legs: 2 };

/** Random numbers for the bot, separate from the match dice so replays stay exact. */
export function botRng(m: Match, side: Side): RngState {
  return seedRng(`${m.config.seed}:bot:${side}:${m.turn}`);
}

/** Share of damage left when the target guards `count` of 4 zones and we aim blindly. */
function guardFactor(count: number, guard: number = GUARD): number {
  return 1 - (count / ZONES.length) * (1 - guard);
}

/** How often the enemy guarded each zone so far (0..1, starts at one half). */
function guessDefence(m: Match, enemy: Side): Record<Zone, number> {
  const n = m.history.length;
  const out = {} as Record<Zone, number>;
  for (const z of ZONES) {
    const seen = m.history.filter((t) => t.defend[enemy].includes(z)).length;
    out[z] = (seen + 1) / (n + 2);
  }
  return out;
}

/** Where the enemy is likely to aim, from its past attacks. */
function guessAim(m: Match, enemy: Side): Record<Zone, number> {
  const w = { ...AIM_PRIOR };
  for (const t of m.history) {
    const a = t.actions[enemy];
    if ((a.type === "shoot" || a.type === "knife") && ZONES.includes(a.zone)) w[a.zone] += 2;
  }
  return w;
}

/** The enemy's likely step this turn, from its last two moves. */
function guessMove(m: Match, enemy: Side): number {
  const recent = m.history.slice(-2).map((t) => t.moves[enemy]);
  if (recent.length === 0) return 0;
  const avg = recent.reduce((a, b) => a + b, 0) / recent.length;
  return Math.max(-1, Math.min(1, Math.round(avg)));
}

/** Value of one attack into a zone, given how likely that zone is guarded. */
function zoneValue(m: Match, side: Side, weapon: AttackWeapon, zone: Zone, d: number, pDef: number): number {
  const guard = weapon === "knife" ? KNIFE_GUARD : GUARD;
  return expectedHit(m, side, weapon, zone, d) * (1 - pDef * (1 - guard));
}

function bestZoneValue(m: Match, side: Side, weapon: AttackWeapon, d: number, defence: Record<Zone, number>): number {
  return Math.max(...ZONES.map((z) => zoneValue(m, side, weapon, z, d, defence[z])));
}

/** Expected damage of a plain shot at distance d, guarded blindly. */
function shotThreat(m: Match, side: Side, d: number, stillAfter: number, targetDashing: boolean, targetGuards: number): number {
  const weapon = m.fighters[side].weapon;
  const p = hitChance(m, side, weapon, d, stillAfter, targetDashing);
  if (p <= 0) return 0;
  let total = 0, weight = 0;
  for (const z of ZONES) {
    total += AIM_PRIOR[z] * expectedHit(m, side, weapon, z, d);
    weight += AIM_PRIOR[z];
  }
  let threat = p * (total / weight) * guardFactor(targetGuards);
  if (d <= 2) {
    const knife = hitChance(m, side, "knife", d, stillAfter, targetDashing) *
      expectedHit(m, side, "knife", "body", d) * guardFactor(targetGuards, KNIFE_GUARD);
    threat = Math.max(threat, knife);
  }
  return threat;
}

/** Who wins the exchange at distance d if both stand and shoot. */
function advantage(m: Match, side: Side, d: number): number {
  const me = m.fighters[side], foe = m.fighters[other(side)];
  const dd = Math.max(1, Math.min(d, MAX_STEPS));
  return shotThreat(m, side, dd, Math.min(me.still + 1, STANCE_TURNS), false, 2) -
    shotThreat(m, other(side), dd, Math.min(foe.still + 1, STANCE_TURNS), false, 2);
}

interface Candidate {
  action: Action;
  score: number;
}

function candidates(m: Match, side: Side, style: BotStyle): Candidate[] {
  const me = m.fighters[side], foe = m.fighters[other(side)];
  const enemy = other(side);
  const defence = guessDefence(m, enemy);
  const foeStill = Math.min(foe.still + 1, STANCE_TURNS);
  // When behind on health, play safer.
  const caution = me.hp / me.maxHp < foe.hp / foe.maxHp ? style.cautionBehind : style.caution;
  // How often the enemy stood still: grenades only hurt a target that stays.
  const stood = m.history.filter((t) => t.moves[enemy] === 0).length;
  const pStand = (stood + 1) / (m.history.length + 2);
  const foeMove = guessMove(m, enemy);
  const limit = distanceLimit(m);

  const list: Action[] = [];
  for (const move of [-1, 0, 1] as const) {
    list.push({ type: "shoot", move, zone: "body" });
    if (canUseKnife(m, move)) list.push({ type: "knife", move, zone: "body" });
    if (canThrowGrenade(m, side)) list.push({ type: "grenade", move });
  }
  list.push({ type: "run", move: 2 }, { type: "run", move: -2 }, { type: "dash" });

  return list.filter((a) => a.type !== "dash" || m.distance > 1).map((action) => {
    const move = action.type === "dash" ? 3 : "move" in action ? action.move : 0;
    const d = Math.max(1, Math.min(limit, m.distance - move - foeMove));
    const stillAfter = move === 0 ? Math.min(me.still + 1, STANCE_TURNS) : 0;
    let deal = 0;
    if (action.type === "shoot") {
      deal = hitChance(m, side, me.weapon, d, stillAfter, false) * bestZoneValue(m, side, me.weapon, d, defence);
    } else if (action.type === "knife") {
      deal = hitChance(m, side, "knife", d, stillAfter, false) * bestZoneValue(m, side, "knife", d, defence);
    } else if (action.type === "grenade") {
      deal = grenadeDamage(foe.armor.body, 0) * pStand;
    }
    const dashing = action.type === "dash";
    const take = shotThreat(m, enemy, d, foeStill, dashing, defendCount(action));
    const future = Math.max(advantage(m, side, d - 1), advantage(m, side, d), advantage(m, side, d + 1));
    return { action, score: deal - caution * take + style.foresight * future };
  });
}

/** Picks with a soft preference for the best score. */
function softPick<T extends { score: number }>(rng: RngState, items: T[], temperature: number): T {
  const top = Math.max(...items.map((c) => c.score));
  const weights = items.map((c) => Math.exp((c.score - top) / temperature));
  const total = weights.reduce((a, b) => a + b, 0);
  let r = nextFloat(rng) * total;
  for (let i = 0; i < items.length; i++) {
    r -= weights[i]!;
    if (r <= 0) return items[i]!;
  }
  return items[items.length - 1]!;
}

function pickZone(rng: RngState, m: Match, side: Side, weapon: AttackWeapon, d: number): Zone {
  const defence = guessDefence(m, other(side));
  const items = ZONES.map((zone) => ({ zone, score: zoneValue(m, side, weapon, zone, d, defence[zone]) }));
  return softPick(rng, items, 4).zone;
}

function pickDefence(rng: RngState, m: Match, side: Side, count: number): Zone[] {
  const aim = guessAim(m, other(side));
  const me = m.fighters[side];
  const items = ZONES.map((zone) => ({
    zone,
    // Guard where the enemy aims and where a hit hurts the most.
    weight: Math.pow(aim[zone] * expectedHit(m, other(side), m.fighters[other(side)].weapon, zone, m.distance) / (me.armor[zone] + 20), 1.5),
  }));
  const out: Zone[] = [];
  while (out.length < count) {
    const left = items.filter((i) => !out.includes(i.zone));
    const total = left.reduce((a, b) => a + b.weight, 0);
    let r = nextFloat(rng) * total;
    let chosen = left[left.length - 1]!.zone;
    for (const i of left) {
      r -= i.weight;
      if (r <= 0) { chosen = i.zone; break; }
    }
    out.push(chosen);
  }
  return out;
}

/** The bot's order for this turn. */
export function botOrder(m: Match, side: Side, rng: RngState = botRng(m, side), style: BotStyle = DEFAULT_BOT): Order {
  const me = m.fighters[side], foe = m.fighters[other(side)];

  // Losing badly at the edge of the map: sometimes it is wiser to leave.
  if (canFlee(m) && me.hp / me.maxHp < 0.2 && foe.hp / foe.maxHp > 0.6 && nextFloat(rng) < 0.5) {
    return { action: { type: "flee" }, defend: pickDefence(rng, m, side, 2) };
  }

  let action = softPick(rng, candidates(m, side, style), style.temperature).action;
  if (action.type === "shoot" || action.type === "knife") {
    const weapon: AttackWeapon = action.type === "knife" ? "knife" : me.weapon;
    const d = Math.max(1, Math.min(distanceLimit(m), plannedDistance(m, action.move) - guessMove(m, other(side))));
    action = { ...action, zone: pickZone(rng, m, side, weapon, d) };
  }
  const order: Order = { action, defend: pickDefence(rng, m, side, defendCount(action)) };
  // The bot plays by the same rules as everyone; fall back to a plain shot if anything is off.
  if (validateOrder(m, side, order)) {
    return { action: { type: "shoot", move: 0, zone: "body" }, defend: pickDefence(rng, m, side, 2) };
  }
  return order;
}
