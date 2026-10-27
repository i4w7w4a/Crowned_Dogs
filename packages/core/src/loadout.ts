// A Persona's kit: what is in each slot and on each weapon. compileKit turns
// it into the numbers the fight uses (see CombatKit in match.ts). Plain data,
// so a kit can be stored with the player and sent to the client or the bot.

import {
  GEAR_SLOTS, ITEM_BY_ID, KIND_FOR_SLOT, MOD_SLOTS, SMOKE_PENALTY, getItem, weaponItem,
  type Effects, type GearSlot, type GrenadeSpec, type Item, type ModSlot, type WeaponItem,
} from "./items.ts";
import type { FighterSetup } from "./match.ts";
import {
  BASE_STATS, NO_STANCE_PENALTY, STANCE_TURNS, WEAPONS, ZONES, ZONE_NAMES, band,
  type Band, type WeaponId, type WeaponSpec, type Zone,
} from "./rules.ts";

export interface EquippedWeapon {
  item: string;
  /** Missing slots keep the factory part. */
  mods?: Partial<Record<ModSlot, string>>;
  skin?: string;
}

export interface Loadout {
  weapon1: EquippedWeapon;
  /** One-handed only; empty for now is fine. */
  weapon2: EquippedWeapon | null;
  head: string;
  body: string;
  suit: string;
  boots: string;
  knife: string;
  grenade: string;
}

/** A weapon with every mod applied. */
export interface KitWeapon extends WeaponSpec {
  doubleShot: number;
  nightAcc: number;
  tracking: number;
  stanceTurns: number;
  noStancePenalty: number;
  /** Added to the share of damage that passes the shield on a guarded zone. */
  guardBonus: number;
}

/** Everything the fight needs to know about a Persona's kit. */
export interface CombatKit {
  weapon: KitWeapon;
  sidearm: KitWeapon | null;
  knife: KitWeapon;
  armor: Record<Zone, number>;
  nvg: boolean;
  thermal: boolean;
  suppressor: boolean;
  heatProof: boolean;
  grenade: GrenadeSpec;
  grenades: number;
  /** Added to the base dodge. */
  dodge: number;
  /** Added to the dodge while dashing. */
  dashEvade: number;
  /** Taken off the enemy's accuracy, by band. */
  camo: Record<Band, number>;
}

/** A fighter for createMatch wearing this kit. */
export function kitFighter(id: string, name: string, l: Loadout): FighterSetup {
  const kit = compileKit(l);
  return { id, name, weapon: kit.weapon.id as WeaponId, kit };
}

// ---------- starter kits ----------

export const STARTER: Omit<Loadout, "weapon1"> = {
  weapon2: null,
  head: "head-cap",
  body: "body-plita2",
  suit: "suit-field",
  boots: "boots-bertsy",
  knife: "knife-std",
  grenade: "grenade-rg1",
};

/** Factory weapon plus starter gear: plays exactly like the concept doc's simulator. */
export function starterLoadout(weapon: WeaponId): Loadout {
  return { ...STARTER, weapon1: { item: weaponItem(weapon).id } };
}

/**
 * Ready kits a veteran might wear, one per class. Used for balance runs and
 * as bot opponents; not "the best" kit, just a sensible one.
 */
export const VETERAN_KITS: Record<WeaponId, Loadout> = {
  pistol: {
    weapon1: { item: "pistol-pustelga", mods: { optic: "optic-kp1", rail: "rail-lcu", mag: "mag-pistol-long", stock: "stock-pistol-hvat" } },
    weapon2: null,
    head: "head-k6", body: "body-light", suit: "suit-ryvok", boots: "boots-hodok", knife: "knife-zhalo", grenade: "grenade-rg1",
  },
  shotgun: {
    weapon1: { item: "shotgun-taran", mods: { optic: "optic-blik", muzzle: "muzzle-choke", rail: "rail-kulak", mag: "mag-shotgun-box", stock: "stock-shotgun-otdacha" } },
    weapon2: null,
    head: "head-k6", body: "body-plita4", suit: "suit-ryvok", boots: "boots-hodok", knife: "knife-zhalo", grenade: "grenade-rg1",
  },
  rifle: {
    weapon1: { item: "rifle-varyag", mods: { optic: "optic-gran", muzzle: "muzzle-klyk", rail: "rail-kulak", mag: "mag-rifle-twin", stock: "stock-rifle-opora" } },
    weapon2: null,
    head: "head-sfera", body: "body-plita4", suit: "suit-solonchak", boots: "boots-hodok", knife: "knife-std", grenade: "grenade-rg1",
  },
  sniper: {
    weapon1: { item: "sniper-berkut", mods: { optic: "optic-oryol", rail: "rail-kulak", mag: "mag-sniper-10", stock: "stock-sniper-izgotovka" } },
    weapon2: { item: "pistol-pustelga", mods: { optic: "optic-kp1" } },
    head: "head-sych", body: "body-plita2", suit: "suit-ten", boots: "boots-bertsy", knife: "knife-iskra", grenade: "grenade-grad",
  },
};

// ---------- checks ----------

export type LoadoutError =
  | { slot: GearSlot; error: "unknown_item" | "wrong_slot" | "two_handed" }
  | { slot: GearSlot; mod: ModSlot; error: "unknown_item" | "wrong_slot" | "does_not_fit" }
  | { slot: GearSlot; error: "bad_skin" };

function checkWeapon(slot: "weapon1" | "weapon2", w: EquippedWeapon): LoadoutError | null {
  const item = ITEM_BY_ID.get(w.item);
  if (!item) return { slot, error: "unknown_item" };
  if (item.kind !== "weapon") return { slot, error: "wrong_slot" };
  if (slot === "weapon2" && WEAPONS[item.weapon].hands !== 1) return { slot, error: "two_handed" };
  for (const [mod, id] of Object.entries(w.mods ?? {}) as [ModSlot, string][]) {
    const m = ITEM_BY_ID.get(id);
    if (!m) return { slot, mod, error: "unknown_item" };
    if (m.kind !== "mod" || m.slot !== mod) return { slot, mod, error: "wrong_slot" };
    if (!m.fits.includes(item.weapon)) return { slot, mod, error: "does_not_fit" };
  }
  if (w.skin !== undefined && ITEM_BY_ID.get(w.skin)?.kind !== "skin") return { slot, error: "bad_skin" };
  return null;
}

/** null when the kit can be worn. */
export function validateLoadout(l: Loadout): LoadoutError | null {
  const first = checkWeapon("weapon1", l.weapon1);
  if (first) return first;
  if (l.weapon2) {
    const second = checkWeapon("weapon2", l.weapon2);
    if (second) return second;
  }
  for (const slot of GEAR_SLOTS) {
    if (slot === "weapon1" || slot === "weapon2") continue;
    const item = ITEM_BY_ID.get(l[slot]);
    if (!item) return { slot, error: "unknown_item" };
    if (item.kind !== KIND_FOR_SLOT[slot]) return { slot, error: "wrong_slot" };
  }
  return null;
}

// ---------- compiling ----------

/** The mod item in each slot of an equipped weapon, factory parts filled in. */
export function weaponParts(w: EquippedWeapon): Record<ModSlot, string> {
  const base = getItem(w.item) as WeaponItem;
  const out = { ...base.stock };
  for (const slot of MOD_SLOTS) {
    const id = w.mods?.[slot];
    if (id) out[slot] = id;
  }
  return out;
}

function sum(list: Effects[], pick: (e: Effects) => number | undefined): number {
  return list.reduce((acc, e) => acc + (pick(e) ?? 0), 0);
}

function bandSum(list: Effects[], pick: (e: Effects) => Partial<Record<Band, number>> | undefined): Record<Band, number> {
  return {
    near: sum(list, (e) => pick(e)?.near),
    mid: sum(list, (e) => pick(e)?.mid),
    far: sum(list, (e) => pick(e)?.far),
  };
}

const round = (v: number) => Math.round(v * 1000) / 1000;

/** Applies weapon-side effects to a base spec. Accuracy changes only where the weapon reaches. */
export function applyWeaponEffects(spec: WeaponSpec, list: Effects[]): KitWeapon {
  const acc = bandSum(list, (e) => e.acc);
  const fall = bandSum(list, (e) => e.falloff);
  const stance = list.some((e) => e.stanceTurns === 1) ? 1 : STANCE_TURNS;
  return {
    ...spec,
    power: spec.power + sum(list, (e) => e.power),
    flatPen: spec.flatPen + sum(list, (e) => e.pen),
    crit: round(spec.crit + sum(list, (e) => e.crit)),
    critBonus: round(spec.critBonus + sum(list, (e) => e.critMult)),
    accuracy: spec.accuracy.map((a, i) => (a > 0 ? round(Math.min(0.98, Math.max(0.02, a + acc[band(i + 1)]))) : 0)),
    falloff: spec.falloff.map((f, i) => round(Math.max(0.1, f + fall[band(i + 1)]))),
    doubleShot: round(sum(list, (e) => e.doubleShot)),
    nightAcc: round(sum(list, (e) => e.nightAcc)),
    tracking: round(sum(list, (e) => e.tracking)),
    stanceTurns: stance,
    noStancePenalty: round(Math.max(0, NO_STANCE_PENALTY + sum(list, (e) => e.noStance))),
    guardBonus: round(sum(list, (e) => e.guard)),
  };
}

function weaponEffects(w: EquippedWeapon): Effects[] {
  return [getItem(w.item).effects, ...Object.values(weaponParts(w)).map((id) => getItem(id).effects)];
}

/** Turns a valid kit into fight numbers. Throws on an invalid kit. */
export function compileKit(l: Loadout): CombatKit {
  const err = validateLoadout(l);
  if (err) throw new Error(`Invalid loadout: ${JSON.stringify(err)}`);
  const main = getItem(l.weapon1.item) as WeaponItem;
  const mainFx = weaponEffects(l.weapon1);
  const side = l.weapon2 ? (getItem(l.weapon2.item) as WeaponItem) : null;
  const sideFx = l.weapon2 ? weaponEffects(l.weapon2) : [];
  const knife = getItem(l.knife);
  const grenade = getItem(l.grenade);
  if (grenade.kind !== "grenade") throw new Error("Not a grenade");
  const wear = [l.head, l.body, l.suit, l.boots].map((id) => getItem(id).effects);
  // Mods can carry Persona-wide effects too (an underbarrel launcher adds a grenade).
  const persona = [...wear, ...mainFx, ...sideFx];

  const armor = {} as Record<Zone, number>;
  for (const z of ZONES) armor[z] = sum(persona, (e) => e.armor?.[z]);

  return {
    weapon: applyWeaponEffects(WEAPONS[main.weapon], mainFx),
    sidearm: side ? applyWeaponEffects(WEAPONS[side.weapon], sideFx) : null,
    knife: applyWeaponEffects(WEAPONS.knife, [knife.effects]),
    armor,
    nvg: persona.some((e) => e.nvg),
    thermal: persona.some((e) => e.thermal),
    // Only the gun in your hands can flash.
    suppressor: mainFx.some((e) => e.suppressor),
    heatProof: persona.some((e) => e.heatProof),
    grenade: { ...grenade.grenade },
    grenades: 1 + sum(persona, (e) => e.grenades),
    dodge: round(sum(persona, (e) => e.dodge)),
    dashEvade: round(sum(persona, (e) => e.dashEvade)),
    camo: bandSum(persona, (e) => e.camo),
  };
}

// ---------- words for the gear screen and the bot ----------

const num = (v: number) => String(Math.round(v * 100) / 100).replace(".", ",");
const pct = (v: number) => `${v > 0 ? "+" : "−"}${Math.round(Math.abs(v) * 100)}%`;
const BAND_WORDS: Record<Band, string> = { near: "вблизи (1–4 шага)", mid: "на средней (5–8)", far: "вдали (9+)" };

/** One line about an effect; `good` tells the gear screen how to colour it. */
export interface EffectLine {
  text: string;
  good: boolean;
}

/** Plain Russian lines, one per effect. Empty for stock parts. */
export function effectLines(e: Effects): EffectLine[] {
  const out: EffectLine[] = [];
  const add = (text: string, good: boolean) => out.push({ text, good });
  for (const b of ["near", "mid", "far"] as const) {
    if (e.acc?.[b]) add(`${pct(e.acc[b]!)} к точности ${BAND_WORDS[b]}`, e.acc[b]! > 0);
  }
  if (e.nightAcc) add(`${pct(e.nightAcc)} к точности ночью`, e.nightAcc > 0);
  for (const b of ["near", "mid", "far"] as const) {
    if (e.falloff?.[b]) add(`урон ${BAND_WORDS[b]} ${pct(e.falloff[b]!)}`, e.falloff[b]! > 0);
  }
  if (e.power) {
    const dmg = e.power * BASE_STATS.mastery;
    add(`урон ${dmg > 0 ? "+" : "−"}${num(Math.round(Math.abs(dmg) * 10) / 10)}`, dmg > 0);
  }
  if (e.pen) add(`${e.pen > 0 ? "+" : "−"}${Math.abs(e.pen)} к пробитию брони`, e.pen > 0);
  if (e.crit) add(`${pct(e.crit)} к шансу крита`, e.crit > 0);
  if (e.critMult) add(`${e.critMult > 0 ? "+" : "−"}${num(Math.abs(e.critMult))} к силе крита`, e.critMult > 0);
  if (e.doubleShot) add(`${Math.round(e.doubleShot * 100)}% шанс попасть дважды`, true);
  if (e.tracking) add(`${pct(e.tracking)} к точности по цели в рывке`, e.tracking > 0);
  if (e.stanceTurns === 1) add("стойка готова за 1 ход вместо 2", true);
  if (e.noStance) add(`без стойки точность ×${num(NO_STANCE_PENALTY + e.noStance)} вместо ×${num(NO_STANCE_PENALTY)}`, e.noStance > 0);
  if (e.guard) add(`сквозь щит проходит ${Math.round((0.5 + e.guard) * 100)}% вместо 50%`, e.guard > 0);
  if (e.suppressor) add("глушитель: ночью нет вспышки", true);
  for (const z of ZONES) {
    if (e.armor?.[z]) add(`броня: ${ZONE_NAMES[z]} ${e.armor[z]}`, true);
  }
  if (e.dodge) add(`${pct(e.dodge)} к уклонению`, e.dodge > 0);
  if (e.dashEvade) add(`${pct(e.dashEvade)} к уклонению в рывке`, e.dashEvade > 0);
  for (const b of ["near", "mid", "far"] as const) {
    if (e.camo?.[b]) add(`по тебе ${BAND_WORDS[b]} ${pct(-e.camo[b]!)} к точности`, e.camo[b]! > 0);
  }
  if (e.grenades) add(`+${e.grenades} граната за бой`, true);
  if (e.nvg) add("ПНВ: нет ночного штрафа", true);
  if (e.thermal) add("тепловизор: пыль и дым мешают вдвое меньше", true);
  if (e.heatProof) add("щит не перегревается днём", true);
  return out;
}

/** Plain Russian lines, one per effect. */
export function describeEffects(e: Effects): string[] {
  return effectLines(e).map((l) => l.text);
}

/** Plain Russian lines for a grenade. */
export function describeGrenade(g: GrenadeSpec): string[] {
  const range = `${g.minDistance}–${g.maxDistance} шагов`;
  switch (g.type) {
    case "frag":
      return [`осколки: ${g.base} до брони`, range];
    case "emp":
      return [`щит врага выключен на следующий ход`, `урон ${g.base} до брони`, range];
    case "smoke":
      return [`дым под ноги: в этот и следующий ход по тебе −${Math.round(SMOKE_PENALTY * 100)}% к точности`, "с любой дистанции"];
  }
}

/** What an item does, in words. */
export function describeItem(item: Item): string[] {
  return item.kind === "grenade" ? describeGrenade(item.grenade) : describeEffects(item.effects);
}
