// All balance numbers in one place. They come from the duel simulator
// (about 20 thousand fights) described in the concept doc, sections
// "Правила хода", "Оружие и дистанции" and "Погода, время суток и события".

export const ZONES = ["head", "body", "groin", "legs"] as const;
export type Zone = (typeof ZONES)[number];

export const ZONE_NAMES: Record<Zone, string> = {
  head: "голова",
  body: "корпус",
  groin: "пах",
  legs: "ноги",
};

/** Damage multiplier per zone (from 4i7). */
export const ZONE_MULT: Record<Zone, number> = { head: 1.27, body: 1.0, groin: 1.11, legs: 0.74 };

/** A level-1 Persona with no stat points spent (4i7 numbers, health set to 100). */
export const BASE_STATS = {
  hp: 100,
  personal: 27,
  mastery: 0.74,
  critChance: 0.07,
  critMult: 1.47,
  dodge: 0.07,
  blockBreakChance: 0.07,
  blockBreakPower: 0.27,
  armorPen: 0.04,
  antiPen: 0.04,
} as const;

/** Starting body armour: plate carrier 20 on the body, 10 on the legs. */
export const DEFAULT_ARMOR: Record<Zone, number> = { head: 0, body: 20, groin: 0, legs: 10 };

/** Share of damage that passes the shield on a defended zone. */
export const GUARD = 0.26;
/** A slow blade gets through the shield more easily. */
export const KNIFE_GUARD = 0.5;
/** In the midday heat the shield overheats and lets a bit more through. */
export const DAY_GUARD_BONUS = 0.04;

export const START_DISTANCE = 7;
/** After this turn the battlefield shrinks by one step per turn... */
export const SHRINK_AFTER_TURN = 6;
/** ...down to this limit. */
export const MIN_DISTANCE_LIMIT = 4;
export const AFK_LIMIT = 3;
/** Being hit while dashing is 10 points harder. */
export const DASH_EVADE = 0.1;
/** At night a muzzle flash gives the enemy +10 points next turn. */
export const MUZZLE_FLASH_BONUS = 0.1;

export const KNIFE_REACH = 2;
export const GRENADE = { base: 35, armorPen: 0.24, minDistance: 2, maxDistance: 8, perFight: 1 } as const;

export const RUN_STEPS = 2;
export const DASH_STEPS = 3;

// ---------- weapons ----------

export const WEAPON_IDS = ["pistol", "shotgun", "rifle", "sniper"] as const;
export type WeaponId = (typeof WEAPON_IDS)[number];
export type AttackWeapon = WeaponId | "knife";

export interface WeaponSpec {
  id: AttackWeapon;
  name: string;
  hands: 1 | 2;
  /** Weapon power W in the 4i7 formula: damage = personal + W * mastery. */
  power: number;
  /** Flat armour penetration. */
  flatPen: number;
  /** Added to the base crit chance. */
  crit: number;
  /** Added to the crit multiplier. */
  critBonus: number;
  /** Accuracy by distance, index = steps - 1. Zero means out of reach. */
  accuracy: readonly number[];
  /** Damage multiplier by distance, index = steps - 1. */
  falloff: readonly number[];
  /** Sniper: full accuracy only after standing still this turn and the last one. */
  needsStance: boolean;
}

export const MAX_STEPS = 16;

/** Expands "up to N steps -> value" pairs into a lookup by distance. */
function byDistance(spec: ReadonlyArray<readonly [number, number]>): number[] {
  const out: number[] = [];
  for (let d = 1; d <= MAX_STEPS; d++) {
    const hit = spec.find(([until]) => d <= until);
    out.push(hit ? hit[1] : 0);
  }
  return out;
}

const FLAT = byDistance([[99, 1]]);

export const WEAPONS: Record<AttackWeapon, WeaponSpec> = {
  pistol: {
    id: "pistol", name: "Пистолет", hands: 1, power: 12, flatPen: 6, crit: 0.15, critBonus: 0,
    accuracy: byDistance([[2, 0.9], [4, 0.85], [5, 0.7], [6, 0.6], [8, 0.42], [10, 0.28], [12, 0.18], [99, 0.1]]),
    falloff: FLAT, needsStance: false,
  },
  shotgun: {
    id: "shotgun", name: "Дробовик", hands: 2, power: 20, flatPen: 2, crit: -0.05, critBonus: 0,
    accuracy: byDistance([[2, 0.95], [3, 0.9], [4, 0.8], [5, 0.55], [6, 0.35], [7, 0.2], [8, 0.1], [99, 0]]),
    falloff: byDistance([[2, 1.4], [4, 1.25], [6, 0.85], [99, 0.6]]), needsStance: false,
  },
  rifle: {
    id: "rifle", name: "Автомат", hands: 2, power: 14, flatPen: 8, crit: 0.03, critBonus: 0,
    accuracy: byDistance([[1, 0.5], [2, 0.55], [3, 0.65], [4, 0.72], [8, 0.8], [9, 0.68], [10, 0.58], [11, 0.48], [12, 0.38], [13, 0.3], [99, 0.2]]),
    falloff: FLAT, needsStance: false,
  },
  sniper: {
    id: "sniper", name: "Снайперка", hands: 2, power: 52, flatPen: 12, crit: 0.25, critBonus: 0.5,
    accuracy: byDistance([[2, 0.25], [4, 0.4], [5, 0.65], [6, 0.75], [8, 0.88], [99, 0.95]]),
    falloff: FLAT, needsStance: true,
  },
  knife: {
    id: "knife", name: "Нож", hands: 1, power: 8, flatPen: 4, crit: 0, critBonus: 0,
    accuracy: byDistance([[KNIFE_REACH, 0.95], [99, 0]]),
    falloff: FLAT, needsStance: false,
  },
};

/** Sniper accuracy without a stance is halved. */
export const NO_STANCE_PENALTY = 0.5;
/** Turns of standing still needed for a stance; the start counts as one. */
export const STANCE_TURNS = 2;

// ---------- conditions ----------

export const WEATHERS = ["clear", "dust", "storm", "fog"] as const;
export type Weather = (typeof WEATHERS)[number];
export const TIMES = ["morning", "day", "evening", "night"] as const;
export type TimeOfDay = (typeof TIMES)[number];
export type Band = "near" | "mid" | "far";

export function band(distance: number): Band {
  return distance <= 4 ? "near" : distance <= 8 ? "mid" : "far";
}

export interface WeatherSpec {
  name: string;
  accuracy: Record<Band, number>;
  crit: number;
  turnLimit: number;
  turnSeconds: number;
}

export const WEATHER: Record<Weather, WeatherSpec> = {
  clear: { name: "ясно", accuracy: { near: 1, mid: 1, far: 1 }, crit: 0, turnLimit: 15, turnSeconds: 60 },
  dust: { name: "пыльная буря", accuracy: { near: 1, mid: 0.9, far: 0.7 }, crit: -0.05, turnLimit: 15, turnSeconds: 45 },
  storm: { name: "гроза", accuracy: { near: 0.9, mid: 0.9, far: 0.9 }, crit: 0, turnLimit: 10, turnSeconds: 60 },
  fog: { name: "туман", accuracy: { near: 1, mid: 1, far: 0.8 }, crit: 0.05, turnLimit: 15, turnSeconds: 60 },
};

export const TIME_NAMES: Record<TimeOfDay, string> = {
  morning: "утро",
  day: "день",
  evening: "вечер",
  night: "ночь",
};

/** Midday heat haze: far shots are 10% worse. */
export const DAY_FAR_ACCURACY = 0.9;
/** Low sun: whoever faces the sunset shoots 10% worse. */
export const SUN_ACCURACY = 0.9;
/** At night without NVG, shots beyond 4 steps are 25% worse. */
export const NIGHT_ACCURACY = 0.75;

/** Region events shift the weather odds (clear, dust, storm, fog). */
export const REGION_EVENTS = {
  calm: { name: "Затишье", odds: { clear: 40, dust: 20, storm: 20, fog: 20 } },
  storms: { name: "Сезон гроз", odds: { clear: 20, dust: 10, storm: 55, fog: 15 } },
  dryWind: { name: "Суховей", odds: { clear: 35, dust: 50, storm: 5, fog: 10 } },
  chemical: { name: "Химический выброс", odds: { clear: 15, dust: 15, storm: 10, fog: 60 } },
} as const satisfies Record<string, { name: string; odds: Record<Weather, number> }>;
export type RegionEvent = keyof typeof REGION_EVENTS;

// ---------- maps ----------

export interface MapSpec {
  id: string;
  name: string;
  region: string;
  /** How far apart the Personas can get. */
  maxDistance: number;
}

export const MAPS: Record<string, MapSpec> = {
  saltHall: { id: "saltHall", name: "Соляной цех", region: "Белый Яр", maxDistance: 11 },
  blocks: { id: "blocks", name: "Пятиэтажки", region: "Белый Яр", maxDistance: 12 },
  dam: { id: "dam", name: "Плотина", region: "Каскад", maxDistance: 14 },
  dryBed: { id: "dryBed", name: "Сухое дно", region: "Каскад", maxDistance: 13 },
};

export const REGIONS: Record<string, { name: string; maps: string[] }> = {
  belyYar: { name: "Белый Яр", maps: ["saltHall", "blocks"] },
  kaskad: { name: "Каскад", maps: ["dam", "dryBed"] },
};

// ---------- formulas ----------

/** 4i7 armour curve: share of damage left after armour. */
export function armorFactor(armor: number, attackerPen: number, flatPen: number, defenderAntiPen: number): number {
  const p = Math.min(Math.max(attackerPen - defenderAntiPen, 0), 0.74);
  const a = Math.max(0, armor * (1 - p) - flatPen);
  return 1 - (0.74 * a) / (47 + a);
}

/** Base damage of one hit before zone, crit, armour and shield. */
export function baseDamage(weapon: WeaponSpec, distance: number): number {
  const fall = weapon.falloff[Math.min(distance, MAX_STEPS) - 1] ?? 1;
  return (BASE_STATS.personal + weapon.power * BASE_STATS.mastery) * fall;
}

/** Grenade damage through the vest; half if the target stepped, none if it ran. */
export function grenadeDamage(
  bodyArmor: number, targetSteps: number, grenade: { base: number; armorPen: number } = GRENADE,
): number {
  const full = grenade.base * armorFactor(bodyArmor, grenade.armorPen, 0, BASE_STATS.antiPen);
  const moved = Math.abs(targetSteps);
  return Math.floor(moved === 0 ? full : moved === 1 ? full / 2 : 0);
}
