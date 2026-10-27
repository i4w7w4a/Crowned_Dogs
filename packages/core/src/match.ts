// One duel between two Personas. The server is the only judge: it keeps the
// Match, collects both secret orders, and calls resolveTurn once per turn.
// Everything here is plain data, so a match can be stored or replayed.

import { SMOKE_PENALTY, type GrenadeSpec } from "./items.ts";
import type { CombatKit, KitWeapon } from "./loadout.ts";
import { nextInt, roll, seedRng, type RngState } from "./rng.ts";
import {
  AFK_LIMIT, BASE_STATS, DASH_EVADE, DASH_STEPS, DASH_WINDED, DAY_FAR_ACCURACY, DAY_GUARD_BONUS, DEFAULT_ARMOR,
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
  /** Thermal sight: half the dust storm and smoke penalty. */
  thermal?: boolean;
  /** Suppressor: no muzzle flash at night. */
  suppressor?: boolean;
  /** The shield does not overheat in the midday heat. */
  heatProof?: boolean;
}

export interface FighterSetup {
  id: string;
  name: string;
  weapon: WeaponId;
  armor?: Partial<Record<Zone, number>>;
  gear?: Gear;
  hp?: number;
  /** Items from compileKit. Replaces weapon, armor and gear; without it the fight uses rules.ts as is. */
  kit?: CombatKit;
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
  /** Turns left with the shield down after an EMP grenade. */
  shieldOff: number;
  /** Turns left hidden in own smoke, this one included: harder to hit. */
  smoke: number;
  /** Holds the kit's sidearm instead of the main weapon. */
  sidearm: boolean;
  /** Dashed last turn: this turn's attack is less accurate. */
  winded: boolean;
}

export type StepMove = -1 | 0 | 1;

export type Action =
  | { type: "shoot"; move: StepMove; zone: Zone }
  | { type: "knife"; move: StepMove; zone: Zone }
  | { type: "grenade"; move: StepMove }
  /** Takes the other gun (main or sidearm) instead of attacking. */
  | { type: "swap"; move: StepMove }
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
  /** The hit landed twice (magazine mods); damage is the sum. */
  double?: true;
  /** A grenade that does more than damage: EMP or smoke. */
  effect?: "emp" | "smoke";
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
    const kit = f.kit;
    return {
      id: f.id,
      name: f.name,
      weapon: kit ? (kit.weapon.id as WeaponId) : f.weapon,
      armor: kit ? { ...kit.armor } : { ...DEFAULT_ARMOR, ...f.armor },
      gear: kit
        ? { nvg: kit.nvg, thermal: kit.thermal, heatProof: kit.heatProof, ...f.gear }
        : { ...f.gear },
      hp,
      maxHp: hp,
      still: 1,
      grenades: kit ? kit.grenades : GRENADE.perFight,
      lastDefend: [],
      afkStreak: 0,
      flash: false,
      winded: false,
      facesSun: sunSide === i,
      shieldOff: 0,
      smoke: 0,
      sidearm: false,
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

// ---------- kit ----------

function plain(spec: (typeof WEAPONS)[AttackWeapon]): KitWeapon {
  return { ...spec, doubleShot: 0, nightAcc: 0, tracking: 0, stanceTurns: STANCE_TURNS, noStancePenalty: NO_STANCE_PENALTY, guardBonus: 0, suppressor: false };
}

/** rules.ts weapons as they come from the factory, for fighters without a kit. */
const PLAIN = Object.fromEntries(Object.values(WEAPONS).map((w) => [w.id, plain(w)])) as Record<AttackWeapon, KitWeapon>;

/** The weapon as this Persona carries it: with its mods if it has a kit. */
export function weaponSpec(m: Match, side: Side, weapon: AttackWeapon): KitWeapon {
  const kit = m.config.fighters[side].kit;
  if (kit) {
    if (weapon === "knife") return kit.knife;
    // Two guns of one class: the one in hand wins.
    if (m.fighters[side].sidearm && kit.sidearm?.id === weapon) return kit.sidearm;
    if (weapon === kit.weapon.id) return kit.weapon;
    if (kit.sidearm?.id === weapon) return kit.sidearm;
  }
  return PLAIN[weapon];
}

/** The gun this Persona would hold after a swap, or null without a sidearm. */
export function swapWeapon(m: Match, side: Side): WeaponId | null {
  const kit = m.config.fighters[side].kit;
  if (!kit?.sidearm) return null;
  return (m.fighters[side].sidearm ? kit.weapon.id : kit.sidearm.id) as WeaponId;
}

/** The grenade this Persona carries. */
export function grenadeSpec(m: Match, side: Side): GrenadeSpec {
  return m.config.fighters[side].kit?.grenade ?? { type: "frag", ...GRENADE };
}

/** Damage of this Persona's grenade on the target; half if it stepped, none if it ran. */
export function grenadeHit(m: Match, side: Side, targetSteps: number): number {
  return grenadeDamage(m.fighters[other(side)].armor.body, targetSteps, grenadeSpec(m, side));
}

/** Share of damage that passes the target's shield on a guarded zone, before heat and block breaks. */
export function guardShare(m: Match, side: Side, weapon: AttackWeapon): number {
  if (m.fighters[other(side)].shieldOff > 0) return 1;
  return (weapon === "knife" ? KNIFE_GUARD : GUARD) + weaponSpec(m, side, weapon).guardBonus;
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
    case "swap":
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
  const g = grenadeSpec(m, side);
  return m.fighters[side].grenades > 0 && m.distance >= g.minDistance && m.distance <= g.maxDistance;
}

export function canUseKnife(m: Match, move: number): boolean {
  return plannedDistance(m, move) <= KNIFE_REACH;
}

/** Only a kit with a second gun can swap. */
export function canSwap(m: Match, side: Side): boolean {
  return swapWeapon(m, side) !== null;
}

export type OrderError =
  | "finished"
  | "bad_move"
  | "bad_defence"
  | "knife_out_of_reach"
  | "grenade_unavailable"
  | "swap_unavailable"
  | "flee_unavailable"
  | "bad_action";

export function validateOrder(m: Match, side: Side, order: Order): OrderError | null {
  if (m.result) return "finished";
  const a = order.action;
  switch (a.type) {
    case "shoot":
    case "knife":
    case "grenade":
    case "swap":
      if (![-1, 0, 1].includes(a.move)) return "bad_move";
      if ("zone" in a && !ZONES.includes(a.zone)) return "bad_action";
      if (a.type === "knife" && !canUseKnife(m, a.move)) return "knife_out_of_reach";
      if (a.type === "grenade" && !canThrowGrenade(m, side)) return "grenade_unavailable";
      if (a.type === "swap" && !canSwap(m, side)) return "swap_unavailable";
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
  const w = weaponSpec(m, side, weapon);
  const target = m.config.fighters[other(side)].kit;
  let acc = w.accuracy[Math.min(distance, MAX_STEPS) - 1] ?? 0;
  if (acc <= 0) return 0;
  if (m.config.time === "night") acc += w.nightAcc;
  if (weapon !== "knife") acc *= conditionAccuracy(m, side, distance);
  if (w.needsStance && stillAfter < w.stanceTurns) acc *= w.noStancePenalty;
  let p = acc - BASE_STATS.dodge;
  if (target) p -= target.dodge + (weapon === "knife" ? 0 : target.camo[band(distance)]);
  if (targetDashing) p -= Math.max(0, DASH_EVADE + (target?.dashEvade ?? 0) - w.tracking);
  if (m.fighters[other(side)].flash) p += MUZZLE_FLASH_BONUS;
  if (m.fighters[side].winded) p -= DASH_WINDED;
  if (m.fighters[other(side)].smoke > 0 && weapon !== "knife") p -= m.fighters[side].gear.thermal ? SMOKE_PENALTY / 2 : SMOKE_PENALTY;
  return clamp(p, 0.05, 0.95);
}

/** Crit chance; pass the side to count the kit's mods. */
export function critChance(m: Match, weapon: AttackWeapon, side?: Side): number {
  const w = side === undefined ? WEAPONS[weapon] : weaponSpec(m, side, weapon);
  return clamp(BASE_STATS.critChance + w.crit + WEATHER[m.config.weather].crit, 0, 0.9);
}

/** Expected damage of one attack into a zone, before luck. Used by the AI and the UI. */
export function expectedHit(m: Match, side: Side, weapon: AttackWeapon, zone: Zone, distance: number): number {
  const w = weaponSpec(m, side, weapon);
  const target = m.fighters[other(side)];
  const cc = critChance(m, weapon, side);
  const crit = 1 + cc * (BASE_STATS.critMult + w.critBonus - 1);
  const armor = armorFactor(target.armor[zone], BASE_STATS.armorPen, w.flatPen, BASE_STATS.antiPen);
  return baseDamage(w, distance) * ZONE_MULT[zone] * crit * armor * (1 + w.doubleShot);
}

/** One bullet (or blade) that found its zone: crit, armour and shield. */
function strike(
  m: Match, side: Side, weapon: AttackWeapon, zone: Zone, distance: number, targetDefend: Zone[],
): { damage: number; crit: boolean; blocked: boolean; shieldBroken: boolean } {
  const rng = m.rng;
  const w = weaponSpec(m, side, weapon);
  const target = m.fighters[other(side)];
  let dmg = baseDamage(w, distance) * ZONE_MULT[zone];
  const crit = roll(rng, critChance(m, weapon, side));
  if (crit) dmg *= BASE_STATS.critMult + w.critBonus;
  dmg *= armorFactor(target.armor[zone], BASE_STATS.armorPen, w.flatPen, BASE_STATS.antiPen);
  const blocked = targetDefend.includes(zone) && !(target.shieldOff > 0);
  let shieldBroken = false;
  if (blocked) {
    let guard = guardShare(m, side, weapon);
    if (m.config.time === "day" && !target.gear.heatProof) guard += DAY_GUARD_BONUS;
    if (roll(rng, BASE_STATS.blockBreakChance)) {
      guard += (1 - GUARD) * BASE_STATS.blockBreakPower;
      shieldBroken = true;
    }
    dmg *= guard;
  }
  return { damage: Math.floor(dmg), crit, blocked, shieldBroken };
}

function rollAttack(
  m: Match, side: Side, action: Action, distance: number, stillAfter: number,
  targetDefend: Zone[], targetDashing: boolean, targetMove: number,
): AttackReport | null {
  const rng = m.rng;
  if (action.type === "grenade") {
    const g = grenadeSpec(m, side);
    const damage = g.base > 0 ? grenadeHit(m, side, targetMove) : 0;
    const report: AttackReport = { weapon: "grenade", zone: null, chance: 1, hit: damage > 0, damage, crit: false, blocked: false, shieldBroken: false };
    // Smoke lands at the thrower's feet; an EMP needs the target to stay inside the blast.
    if (g.type === "smoke") return { ...report, hit: true, effect: "smoke" };
    if (g.type === "emp" && Math.abs(targetMove) < 2) return { ...report, hit: true, effect: "emp" };
    return report;
  }
  if (action.type !== "shoot" && action.type !== "knife") return null;
  const weapon: AttackWeapon = action.type === "knife" ? "knife" : m.fighters[side].weapon;
  const zone = action.zone;
  const chance = hitChance(m, side, weapon, distance, stillAfter, targetDashing);
  const miss: AttackReport = { weapon, zone, chance, hit: false, damage: 0, crit: false, blocked: false, shieldBroken: false };
  if (chance <= 0 || !roll(rng, chance)) return miss;

  const first = strike(m, side, weapon, zone, distance, targetDefend);
  const report: AttackReport = { weapon, zone, chance, hit: true, ...first };
  const twice = weaponSpec(m, side, weapon).doubleShot;
  if (twice > 0 && roll(rng, twice)) {
    const second = strike(m, side, weapon, zone, distance, targetDefend);
    report.damage += second.damage;
    report.crit ||= second.crit;
    report.shieldBroken ||= second.shieldBroken;
    report.double = true;
  }
  return report;
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

  // 2. Smoke goes down at once and hides the thrower this turn and the next.
  for (const s of sides) {
    if (final[s]!.action.type === "grenade" && grenadeSpec(m, s).type === "smoke") m.fighters[s].smoke = 2;
  }

  // 3. Both attacks fly at the new distance at the same moment.
  const attacks = sides.map((s) => {
    const t = other(s);
    return rollAttack(m, s, final[s]!.action, after, stillAfter[s]!, final[t]!.defend, dashing[t]!, moves[t]);
  }) as [AttackReport | null, AttackReport | null];

  // 4. Apply everything. Smoke and a downed shield last through the next turn.
  m.distance = after;
  for (const s of sides) {
    const f = m.fighters[s];
    const incoming = attacks[other(s)];
    if (incoming) f.hp = Math.max(0, f.hp - incoming.damage);
    f.shieldOff = incoming?.effect === "emp" ? 1 : f.shieldOff > 0 ? f.shieldOff - 1 : 0;
    f.smoke = f.smoke > 0 ? f.smoke - 1 : 0;
    f.still = stillAfter[s]!;
    if (final[s]!.action.type === "grenade") f.grenades -= 1;
    if (final[s]!.action.type === "swap") {
      f.weapon = swapWeapon(m, s)!;
      f.sidearm = !f.sidearm;
    }
    f.lastDefend = [...final[s]!.defend];
    f.afkStreak = afk[s] ? f.afkStreak + 1 : 0;
    const silenced = f.gear.suppressor || weaponSpec(m, s, f.weapon).suppressor;
    f.flash = m.config.time === "night" && final[s]!.action.type === "shoot" && !silenced;
    f.winded = final[s]!.action.type === "dash";
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
