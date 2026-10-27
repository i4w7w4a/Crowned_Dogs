// One duel between two Personas. The server is the only judge: it keeps the
// Match, collects both secret orders, and calls resolveTurn once per turn.
// Everything here is plain data, so a match can be stored or replayed.

import { nextInt, roll, seedRng, type RngState } from "./rng.ts";
import {
  AFK_LIMIT, BASE_STATS, DASH_EVADE, DASH_STEPS, DAY_FAR_ACCURACY, DAY_GUARD_BONUS, DEFAULT_ARMOR,
  GRENADE, GUARD, KNIFE_GUARD, KNIFE_REACH, MAPS, MAX_STEPS, MIN_DISTANCE_LIMIT, MUZZLE_FLASH_BONUS,
  NIGHT_ACCURACY, NO_STANCE_PENALTY, RUN_STEPS, SHRINK_AFTER_TURN, STANCE_TURNS, START_DISTANCE,
  SUN_ACCURACY, WEAPONS, WEATHER, ZONES, ZONE_MULT, armorFactor, band, baseDamage, grenadeDamage,
  type AttackWeapon, type MapSpec, type TimeOfDay, type Weather, type WeaponId, type Zone,
} from "./rules.ts";

export type Side = 0 | 1;
export const other = (side: Side): Side => (side === 0 ? 1 : 0);

export interface Gear {
  /** Night vision: no night penalty. */
  nvg?: boolean;
  /** Thermal sight: half the dust storm penalty. */
  thermal?: boolean;
  /** Suppressor: no muzzle flash at night. */
  suppressor?: boolean;
}

export interface FighterSetup {
  id: string;
  name: string;
  weapon: WeaponId;
  armor?: Partial<Record<Zone, number>>;
  gear?: Gear;
  hp?: number;
}

export interface FighterState {
  id: string;
  name: string;
  weapon: WeaponId;
  armor: Record<Zone, number>;
  gear: Gear;
  hp: number;
  maxHp: number;
  /** Turns in a row without moving, capped at 2. The start counts as one. */
  still: number;
  grenades: number;
  /** Defence of the last turn: kept if the player misses the timer. */
  lastDefend: Zone[];
  afkStreak: number;
  /** Fired at night without a suppressor last turn: easier to hit now. */
  flash: boolean;
  /** Evening: this Persona faces the low sun. */
  facesSun: boolean;
}

export type StepMove = -1 | 0 | 1;

export type Action =
  | { type: "shoot"; move: StepMove; zone: Zone }
  | { type: "knife"; move: StepMove; zone: Zone }
  | { type: "grenade"; move: StepMove }
  | { type: "run"; move: -2 | 2 }
  | { type: "dash" }
  | { type: "flee" }
  /** Missed the timer: stands still and does not attack. */
  | { type: "wait" };

export interface Order {
  action: Action;
  defend: Zone[];
}

export interface MatchConfig {
  seed: string;
  mapId: string;
  weather: Weather;
  time: TimeOfDay;
  fighters: [FighterSetup, FighterSetup];
  startDistance?: number;
}

export type EndReason = "ko" | "double_ko" | "fled" | "turn_limit" | "afk";

export interface MatchResult {
  /** null is a draw. */
  winner: Side | null;
  reason: EndReason;
}

export interface AttackReport {
  weapon: AttackWeapon | "grenade";
  zone: Zone | null;
  chance: number;
  hit: boolean;
  damage: number;
  crit: boolean;
  /** The zone was defended, so the shield took most of the hit. */
  blocked: boolean;
  /** The shot pushed through the shield (block break). */
  shieldBroken: boolean;
}

export interface TurnReport {
  turn: number;
  distanceBefore: number;
  distanceAfter: number;
  limit: number;
  actions: [Action, Action];
  defend: [Zone[], Zone[]];
  moves: [number, number];
  attacks: [AttackReport | null, AttackReport | null];
  hp: [number, number];
  afk: [boolean, boolean];
}

export interface Match {
  version: 1;
  config: MatchConfig;
  map: MapSpec;
  turn: number;
  distance: number;
  fighters: [FighterState, FighterState];
  rng: RngState;
  result: MatchResult | null;
  history: TurnReport[];
}

// ---------- setup ----------

export function createMatch(config: MatchConfig): Match {
  const map = MAPS[config.mapId];
  if (!map) throw new Error(`Unknown map: ${config.mapId}`);
  const rng = seedRng(config.seed);
  const sunSide = config.time === "evening" ? nextInt(rng, 2) : -1;
  const fighters = config.fighters.map((f, i): FighterState => {
    const hp = f.hp ?? BASE_STATS.hp;
    return {
      id: f.id,
      name: f.name,
      weapon: f.weapon,
      armor: { ...DEFAULT_ARMOR, ...f.armor },
      gear: { ...f.gear },
      hp,
      maxHp: hp,
      still: 1,
      grenades: GRENADE.perFight,
      lastDefend: [],
      afkStreak: 0,
      flash: false,
      facesSun: sunSide === i,
    };
  }) as [FighterState, FighterState];
  return {
    version: 1,
    config,
    map,
    turn: 1,
    distance: Math.min(config.startDistance ?? START_DISTANCE, map.maxDistance),
    fighters,
    rng,
    result: null,
    history: [],
  };
}

// ---------- rules helpers ----------

export function turnLimit(m: Match): number {
  return WEATHER[m.config.weather].turnLimit;
}

export function turnSeconds(m: Match): number {
  return WEATHER[m.config.weather].turnSeconds;
}

/** How far apart the Personas can be on this turn (the field shrinks after turn 6). */
export function distanceLimit(m: Match, turn: number = m.turn): number {
  const shrink = Math.max(0, turn - SHRINK_AFTER_TURN);
  return Math.max(Math.min(MIN_DISTANCE_LIMIT, m.map.maxDistance), m.map.maxDistance - shrink);
}

/** Steps toward the enemy (negative = away). */
export function moveOf(action: Action): number {
  switch (action.type) {
    case "shoot":
    case "knife":
    case "grenade":
    case "run":
      return action.move;
    case "dash":
      return DASH_STEPS;
    default:
      return 0;
  }
}

export function defendCount(action: Action): number {
  return action.type === "dash" ? 1 : 2;
}

/** Distance after this move if the enemy stands still. */
export function plannedDistance(m: Match, move: number): number {
  return clamp(m.distance - move, 1, distanceLimit(m));
}

/** "Leave the fight" appears only at the edge of the map. */
export function canFlee(m: Match): boolean {
  return m.distance >= distanceLimit(m);
}

export function canThrowGrenade(m: Match, side: Side): boolean {
  return m.fighters[side].grenades > 0 && m.distance >= GRENADE.minDistance && m.distance <= GRENADE.maxDistance;
}

export function canUseKnife(m: Match, move: number): boolean {
  return plannedDistance(m, move) <= KNIFE_REACH;
}

export type OrderError =
  | "finished"
  | "bad_move"
  | "bad_defence"
  | "knife_out_of_reach"
  | "grenade_unavailable"
  | "flee_unavailable"
  | "bad_action";

export function validateOrder(m: Match, side: Side, order: Order): OrderError | null {
  if (m.result) return "finished";
  const a = order.action;
  switch (a.type) {
    case "shoot":
    case "knife":
    case "grenade":
      if (![-1, 0, 1].includes(a.move)) return "bad_move";
      if ("zone" in a && !ZONES.includes(a.zone)) return "bad_action";
      if (a.type === "knife" && !canUseKnife(m, a.move)) return "knife_out_of_reach";
      if (a.type === "grenade" && !canThrowGrenade(m, side)) return "grenade_unavailable";
      break;
    case "run":
      if (Math.abs(a.move) !== RUN_STEPS) return "bad_move";
      break;
    case "dash":
      break;
    case "flee":
      if (!canFlee(m)) return "flee_unavailable";
      break;
    default:
      return "bad_action";
  }
  const zones = new Set(order.defend);
  if (zones.size !== order.defend.length || order.defend.some((z) => !ZONES.includes(z))) return "bad_defence";
  if (order.defend.length !== defendCount(a)) return "bad_defence";
  return null;
}

// ---------- hit chance and damage ----------

/** Accuracy multiplier from weather, time of day and gear. */
export function conditionAccuracy(m: Match, side: Side, distance: number): number {
  const me = m.fighters[side];
  const b = band(distance);
  let mult = WEATHER[m.config.weather].accuracy[b];
  if (m.config.weather === "dust" && me.gear.thermal) mult = 1 - (1 - mult) / 2;
  switch (m.config.time) {
    case "day":
      if (b === "far") mult *= DAY_FAR_ACCURACY;
      break;
    case "evening":
      if (me.facesSun) mult *= SUN_ACCURACY;
      break;
    case "night":
      if (distance > 4 && !me.gear.nvg) mult *= NIGHT_ACCURACY;
      break;
  }
  return mult;
}

/**
 * Chance to hit. `stillAfter` is the attacker's standing streak after this
 * turn's move (a sniper needs 2 for the stance).
 */
export function hitChance(
  m: Match, side: Side, weapon: AttackWeapon, distance: number, stillAfter: number, targetDashing: boolean,
): number {
  const w = WEAPONS[weapon];
  let acc = w.accuracy[Math.min(distance, MAX_STEPS) - 1] ?? 0;
  if (acc <= 0) return 0;
  if (weapon !== "knife") acc *= conditionAccuracy(m, side, distance);
  if (w.needsStance && stillAfter < STANCE_TURNS) acc *= NO_STANCE_PENALTY;
  let p = acc - BASE_STATS.dodge;
  if (targetDashing) p -= DASH_EVADE;
  if (m.fighters[other(side)].flash) p += MUZZLE_FLASH_BONUS;
  return clamp(p, 0.05, 0.95);
}

export function critChance(m: Match, weapon: AttackWeapon): number {
  return clamp(BASE_STATS.critChance + WEAPONS[weapon].crit + WEATHER[m.config.weather].crit, 0, 0.9);
}

/** Expected damage of one attack into a zone, before luck. Used by the AI and the UI. */
export function expectedHit(m: Match, side: Side, weapon: AttackWeapon, zone: Zone, distance: number): number {
  const w = WEAPONS[weapon];
  const target = m.fighters[other(side)];
  const cc = critChance(m, weapon);
  const crit = 1 + cc * (BASE_STATS.critMult + w.critBonus - 1);
  const armor = armorFactor(target.armor[zone], BASE_STATS.armorPen, w.flatPen, BASE_STATS.antiPen);
  return baseDamage(w, distance) * ZONE_MULT[zone] * crit * armor;
}

function rollAttack(
  m: Match, side: Side, action: Action, distance: number, stillAfter: number,
  targetDefend: Zone[], targetDashing: boolean, targetMove: number,
): AttackReport | null {
  const rng = m.rng;
  const target = m.fighters[other(side)];
  if (action.type === "grenade") {
    const damage = grenadeDamage(target.armor.body, targetMove);
    return { weapon: "grenade", zone: null, chance: 1, hit: damage > 0, damage, crit: false, blocked: false, shieldBroken: false };
  }
  if (action.type !== "shoot" && action.type !== "knife") return null;
  const weapon: AttackWeapon = action.type === "knife" ? "knife" : m.fighters[side].weapon;
  const w = WEAPONS[weapon];
  const zone = action.zone;
  const chance = hitChance(m, side, weapon, distance, stillAfter, targetDashing);
  const miss: AttackReport = { weapon, zone, chance, hit: false, damage: 0, crit: false, blocked: false, shieldBroken: false };
  if (chance <= 0 || !roll(rng, chance)) return miss;

  let dmg = baseDamage(w, distance) * ZONE_MULT[zone];
  const crit = roll(rng, critChance(m, weapon));
  if (crit) dmg *= BASE_STATS.critMult + w.critBonus;
  dmg *= armorFactor(target.armor[zone], BASE_STATS.armorPen, w.flatPen, BASE_STATS.antiPen);
  const blocked = targetDefend.includes(zone);
  let shieldBroken = false;
  if (blocked) {
    let guard = weapon === "knife" ? KNIFE_GUARD : GUARD;
    if (m.config.time === "day") guard += DAY_GUARD_BONUS;
    if (roll(rng, BASE_STATS.blockBreakChance)) {
      guard += (1 - GUARD) * BASE_STATS.blockBreakPower;
      shieldBroken = true;
    }
    dmg *= guard;
  }
  return { weapon, zone, chance, hit: true, damage: Math.floor(dmg), crit, blocked, shieldBroken };
}

// ---------- the turn ----------

const WAIT: Action = { type: "wait" };

/**
 * Plays one turn. Both orders are secret until now; a missing order (null)
 * means the player ran out of time. Returns a new Match; the input is untouched.
 */
export function resolveTurn(match: Match, orders: [Order | null, Order | null]): { match: Match; report: TurnReport } {
  if (match.result) throw new Error("The match is already over");
  const m: Match = structuredClone(match);
  const sides: Side[] = [0, 1];

  const afk: [boolean, boolean] = [orders[0] === null, orders[1] === null];
  const final = sides.map((s) => {
    const o = orders[s];
    if (!o) return { action: WAIT, defend: [...m.fighters[s].lastDefend] };
    const err = validateOrder(m, s, o);
    if (err) throw new Error(`Invalid order for side ${s}: ${err}`);
    return { action: o.action, defend: [...o.defend] };
  }) as [Order, Order];

  // 1. Both move at once. Nobody walks through the enemy, and nobody leaves the field by walking.
  const moves: [number, number] = [moveOf(final[0].action), moveOf(final[1].action)];
  const before = m.distance;
  const limit = distanceLimit(m);
  const after = clamp(before - moves[0] - moves[1], 1, limit);
  const stillAfter = moves.map((mv, s) => (mv === 0 ? Math.min(m.fighters[s as Side].still + 1, STANCE_TURNS) : 0));
  const dashing = final.map((o) => o.action.type === "dash");

  // 2. Both attacks fly at the new distance at the same moment.
  const attacks = sides.map((s) => {
    const t = other(s);
    return rollAttack(m, s, final[s]!.action, after, stillAfter[s]!, final[t]!.defend, dashing[t]!, moves[t]);
  }) as [AttackReport | null, AttackReport | null];

  // 3. Apply everything.
  m.distance = after;
  for (const s of sides) {
    const f = m.fighters[s];
    const incoming = attacks[other(s)];
    if (incoming) f.hp = Math.max(0, f.hp - incoming.damage);
    f.still = stillAfter[s]!;
    if (final[s]!.action.type === "grenade") f.grenades -= 1;
    f.lastDefend = [...final[s]!.defend];
    f.afkStreak = afk[s] ? f.afkStreak + 1 : 0;
    f.flash = m.config.time === "night" && final[s]!.action.type === "shoot" && !f.gear.suppressor;
  }

  const report: TurnReport = {
    turn: m.turn,
    distanceBefore: before,
    distanceAfter: after,
    limit,
    actions: [final[0].action, final[1].action],
    defend: [final[0].defend, final[1].defend],
    moves,
    attacks,
    hp: [m.fighters[0].hp, m.fighters[1].hp],
    afk,
  };
  m.history.push(report);
  m.result = judge(m, final);
  if (!m.result) m.turn += 1;
  return { match: m, report };
}

function judge(m: Match, final: [Order, Order]): MatchResult | null {
  const [a, b] = m.fighters;
  const downA = a.hp <= 0, downB = b.hp <= 0;
  if (downA && downB) return { winner: null, reason: "double_ko" };
  if (downA) return { winner: 1, reason: "ko" };
  if (downB) return { winner: 0, reason: "ko" };

  const fledA = final[0].action.type === "flee", fledB = final[1].action.type === "flee";
  if (fledA || fledB) return { winner: fledA && fledB ? null : fledA ? 1 : 0, reason: "fled" };

  const afkA = a.afkStreak >= AFK_LIMIT, afkB = b.afkStreak >= AFK_LIMIT;
  if (afkA || afkB) return { winner: afkA && afkB ? null : afkA ? 1 : 0, reason: "afk" };

  if (m.turn >= turnLimit(m)) {
    const pa = a.hp / a.maxHp, pb = b.hp / b.maxHp;
    return { winner: pa === pb ? null : pa > pb ? 0 : 1, reason: "turn_limit" };
  }
  return null;
}

function clamp(v: number, lo: number, hi: number): number {
  return Math.max(lo, Math.min(hi, v));
}
