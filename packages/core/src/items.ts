// The item catalogue: weapons with five mod slots each, and the gear a
// Persona wears. Every item except the stock and starter ones changes a
// number in the fight, and every one of them is visible on the model.
// The starter kit adds nothing on top of rules.ts, so a fight in starter gear
// plays exactly like the duel simulator from the concept doc.

import type { Band, WeaponId, Zone } from "./rules.ts";

export type Tier = 1 | 2 | 3;
export const TIER_NAMES: Record<Tier, string> = { 1: "серийное", 2: "армейское", 3: "штучное" };

/** What an item changes. Numbers add up over the whole kit; points are shares (0.04 = 4%). */
export interface Effects {
  // ----- the weapon the mod is on (or the knife) -----
  /** Accuracy by distance band, only where the weapon reaches at all. */
  acc?: Partial<Record<Band, number>>;
  /** Accuracy at night, any distance. */
  nightAcc?: number;
  /** Damage multiplier by distance band (the shotgun's spread). */
  falloff?: Partial<Record<Band, number>>;
  /** Weapon power W: damage = personal + W * mastery, so +1 is about +0.74 damage. */
  power?: number;
  /** Flat armour penetration. */
  pen?: number;
  crit?: number;
  critMult?: number;
  /** Chance that a hit lands twice. */
  doubleShot?: number;
  /** Cuts the dodge bonus of a target that dashes. */
  tracking?: number;
  /** Sniper: turns of standing still for a full-accuracy stance (default 2). */
  stanceTurns?: 1;
  /** Sniper: added to the no-stance accuracy multiplier (negative = worse off-stance). */
  noStance?: number;
  /** Knife: added to the share of damage that passes the shield. */
  guard?: number;
  suppressor?: true;
  // ----- the Persona -----
  armor?: Partial<Record<Zone, number>>;
  dodge?: number;
  /** Added dodge while dashing. */
  dashEvade?: number;
  /** Taken off the enemy's accuracy against you, by distance band. */
  camo?: Partial<Record<Band, number>>;
  /** Extra grenades per fight. */
  grenades?: number;
  nvg?: true;
  thermal?: true;
  /** The shield does not overheat in the midday heat. */
  heatProof?: true;
}

export const GEAR_SLOTS = ["weapon1", "weapon2", "head", "body", "suit", "boots", "knife", "grenade"] as const;
export type GearSlot = (typeof GEAR_SLOTS)[number];

export const GEAR_SLOT_NAMES: Record<GearSlot, string> = {
  weapon1: "Оружие 1",
  weapon2: "Оружие 2",
  head: "Голова",
  body: "Корпус",
  suit: "Костюм",
  boots: "Ботинки",
  knife: "Нож",
  grenade: "Граната",
};

export const MOD_SLOTS = ["optic", "muzzle", "rail", "mag", "stock"] as const;
export type ModSlot = (typeof MOD_SLOTS)[number];

export const MOD_SLOT_NAMES: Record<ModSlot, string> = {
  optic: "Прицел",
  muzzle: "Дульное устройство",
  rail: "Цевьё и модуль",
  mag: "Магазин",
  stock: "Приклад или рукоять",
};

interface ItemBase {
  id: string;
  name: string;
  tier: Tier;
  /** One plain sentence for the gear screen and the bot. */
  note: string;
  effects: Effects;
}

export interface WeaponItem extends ItemBase {
  kind: "weapon";
  /** Combat numbers live in rules.ts WEAPONS[weapon]. */
  weapon: WeaponId;
  /** The parts it leaves the factory with, one per mod slot. */
  stock: Record<ModSlot, string>;
}

export interface ModItem extends ItemBase {
  kind: "mod";
  slot: ModSlot;
  fits: readonly WeaponId[];
  /** Factory part: no effects, swapped out by any other mod. */
  isStock?: true;
}

/** Paint for any weapon. Looks only. */
export interface SkinItem extends ItemBase {
  kind: "skin";
}

export interface WearItem extends ItemBase {
  kind: "head" | "body" | "suit" | "boots" | "knife";
}

export type GrenadeType = "frag" | "emp" | "smoke";

export interface GrenadeSpec {
  type: GrenadeType;
  /** Damage before the vest. */
  base: number;
  armorPen: number;
  minDistance: number;
  maxDistance: number;
}

/** Smoke: accuracy taken off shots at the thrower this turn and the next (thermal halves it). */
export const SMOKE_PENALTY = 0.35;

export interface GrenadeItem extends ItemBase {
  kind: "grenade";
  grenade: GrenadeSpec;
}

export type Item = WeaponItem | ModItem | SkinItem | WearItem | GrenadeItem;

// ---------- weapons ----------

const WEAPON_LIST: WeaponItem[] = [
  {
    id: "pistol-pustelga", kind: "weapon", weapon: "pistol", name: "Пистолет «Пустельга»", tier: 1,
    note: "Служебный пистолет ЧВК. Часто критует, носится запасным к любому стволу.",
    effects: {},
    stock: { optic: "pistol-optic-stock", muzzle: "pistol-muzzle-stock", rail: "pistol-rail-stock", mag: "pistol-mag-stock", stock: "pistol-stock-stock" },
  },
  {
    id: "shotgun-taran", kind: "weapon", weapon: "shotgun", name: "Дробовик «Таран»", tier: 1,
    note: "Полуавтомат 12-го калибра. Страшен вплотную, дальше 8 шагов не достаёт.",
    effects: {},
    stock: { optic: "shotgun-optic-stock", muzzle: "shotgun-muzzle-stock", rail: "shotgun-rail-stock", mag: "shotgun-mag-stock", stock: "shotgun-stock-stock" },
  },
  {
    id: "rifle-varyag", kind: "weapon", weapon: "rifle", name: "Автомат «Варяг»", tier: 1,
    note: "Армейский автомат. Ровный на любой дистанции, лучший выбор для новичка.",
    effects: {},
    stock: { optic: "rifle-optic-stock", muzzle: "rifle-muzzle-stock", rail: "rifle-rail-stock", mag: "rifle-mag-stock", stock: "rifle-stock-stock" },
  },
  {
    id: "sniper-berkut", kind: "weapon", weapon: "sniper", name: "Винтовка «Беркут»", tier: 1,
    note: "Снайперская винтовка. Бьёт сильнее всех и часто критует, но стреляет точно только из стойки.",
    effects: {},
    stock: { optic: "sniper-optic-stock", muzzle: "sniper-muzzle-stock", rail: "sniper-rail-stock", mag: "sniper-mag-stock", stock: "sniper-stock-stock" },
  },
];

// ---------- weapon mods ----------

const P = ["pistol"] as const, S = ["shotgun"] as const, R = ["rifle"] as const, N = ["sniper"] as const;

function stockMod(weapon: WeaponId, slot: ModSlot, name: string, note: string): ModItem {
  return { id: `${weapon}-${slot}-stock`, kind: "mod", slot, fits: [weapon], name, tier: 1, note, effects: {}, isStock: true };
}

const MOD_LIST: ModItem[] = [
  // Factory parts. They are on the base model and change nothing.
  stockMod("pistol", "optic", "Открытый прицел", "Заводская мушка и целик."),
  stockMod("pistol", "muzzle", "Без насадки", "Голый ствол."),
  stockMod("pistol", "rail", "Пустая планка", "Планка под фонарь или лазер."),
  stockMod("pistol", "mag", "Магазин на 15", "Заводской магазин."),
  stockMod("pistol", "stock", "Заводская рукоять", "Гладкие пластиковые щёчки."),
  stockMod("shotgun", "optic", "Мушка", "Планка с мушкой."),
  stockMod("shotgun", "muzzle", "Цилиндр", "Ствол без сужения, дробь разлетается широко."),
  stockMod("shotgun", "rail", "Заводское цевьё", "Гладкое цевьё."),
  stockMod("shotgun", "mag", "Подствольный магазин на 5", "Трубчатый магазин под стволом."),
  stockMod("shotgun", "stock", "Заводской приклад", "Жёсткий пластиковый приклад."),
  stockMod("rifle", "optic", "Открытый прицел", "Заводская мушка и целик."),
  stockMod("rifle", "muzzle", "Пламегаситель", "Заводская насадка."),
  stockMod("rifle", "rail", "Заводское цевьё", "Цевьё с планками."),
  stockMod("rifle", "mag", "Магазин на 30", "Заводской магазин."),
  stockMod("rifle", "stock", "Складной приклад", "Заводской складной приклад."),
  stockMod("sniper", "optic", "Прицел ПСО-6", "Заводская оптика."),
  stockMod("sniper", "muzzle", "Дульный тормоз", "Заводской тормоз."),
  stockMod("sniper", "rail", "Заводское ложе", "Ложе без модулей."),
  stockMod("sniper", "mag", "Магазин на 5", "Заводской магазин."),
  stockMod("sniper", "stock", "Заводской приклад", "Деревянный приклад."),

  // Optics: move the sweet spot closer or farther.
  {
    id: "optic-tritium", kind: "mod", slot: "optic", fits: P, name: "Тритиевые целики", tier: 1,
    note: "Светящиеся точки на мушке видны в темноте.",
    effects: { nightAcc: 0.05 },
  },
  {
    id: "optic-kp1", kind: "mod", slot: "optic", fits: P, name: "Мини-коллиматор КП-1", tier: 2,
    note: "Маленькая красная точка на затворе. Пистолет держит среднюю дистанцию.",
    effects: { acc: { near: 0.02, mid: 0.03 } },
  },
  {
    id: "optic-blik", kind: "mod", slot: "optic", fits: [...S, ...R], name: "Коллиматор «Блик»", tier: 1,
    note: "Красная точка без увеличения: быстро вблизи, хуже вдали.",
    effects: { acc: { near: 0.04, mid: 0.02, far: -0.02 } },
  },
  {
    id: "optic-gran", kind: "mod", slot: "optic", fits: [...S, ...R], name: "Призма 2× «Грань»", tier: 2,
    note: "Небольшое увеличение для средней дистанции.",
    effects: { acc: { near: -0.02, mid: 0.05 } },
  },
  {
    id: "optic-sokol", kind: "mod", slot: "optic", fits: R, name: "Оптика 4× «Сокол»", tier: 2,
    note: "Делает из автомата полуснайперку. Вплотную в неё неудобно целиться.",
    effects: { acc: { near: -0.05, mid: 0.02, far: 0.07 } },
  },
  {
    id: "optic-znoy", kind: "mod", slot: "optic", fits: [...R, ...N], name: "Тепловизионный прицел «Зной»", tier: 3,
    note: "Видит тепло сквозь пыль, дым и темноту. Тяжёлый, вплотную медленный.",
    effects: { thermal: true, nightAcc: 0.04, acc: { near: -0.03 } },
  },
  {
    id: "optic-oryol", kind: "mod", slot: "optic", fits: N, name: "Оптика 12× «Орёл»", tier: 2,
    note: "Сильное увеличение: точный выстрел в уязвимое место, но ближе 9 шагов цель не помещается.",
    effects: { acc: { near: -0.03, far: 0.04 }, crit: 0.05 },
  },
  {
    id: "optic-blik-b", kind: "mod", slot: "optic", fits: N, name: "Боковой коллиматор «Блик-Б»", tier: 2,
    note: "Красная точка сбоку от оптики на случай, если враг подбежал.",
    effects: { acc: { near: 0.06, far: -0.02 } },
  },

  // Muzzle devices.
  {
    id: "muzzle-pb9", kind: "mod", slot: "muzzle", fits: P, name: "Глушитель ПБ-9", tier: 2,
    note: "Ночью нет вспышки. Пуля чуть слабее.",
    effects: { suppressor: true, power: -2 },
  },
  {
    id: "muzzle-klyk", kind: "mod", slot: "muzzle", fits: [...P, ...R], name: "Компенсатор «Клык»", tier: 2,
    note: "Гасит подброс ствола. Ночью вспышка слепит самого стрелка.",
    effects: { acc: { near: 0.02, mid: 0.04 }, nightAcc: -0.04 },
  },
  {
    id: "muzzle-choke", kind: "mod", slot: "muzzle", fits: S, name: "Чок «Кучность»", tier: 2,
    note: "Сужение ствола: дробь летит кучнее и достаёт дальше, но вплотную легче промахнуться.",
    effects: { acc: { near: -0.02, mid: 0.08 } },
  },
  {
    id: "muzzle-vata", kind: "mod", slot: "muzzle", fits: S, name: "Глушитель «Вата»", tier: 3,
    note: "Редкий глушитель для дробовика. Ночью нет вспышки.",
    effects: { suppressor: true, power: -1 },
  },
  {
    id: "muzzle-tishina", kind: "mod", slot: "muzzle", fits: R, name: "Глушитель «Тишина»", tier: 2,
    note: "Ночью нет вспышки. Дозвуковой патрон хуже летит далеко.",
    effects: { suppressor: true, acc: { far: -0.03 } },
  },
  {
    id: "muzzle-tishina-s", kind: "mod", slot: "muzzle", fits: N, name: "Глушитель «Тишина-С»", tier: 2,
    note: "Ночью нет вспышки. Пуля слабее.",
    effects: { suppressor: true, power: -3 },
  },
  {
    id: "muzzle-molot", kind: "mod", slot: "muzzle", fits: N, name: "Тормоз-компенсатор «Молот»", tier: 3,
    note: "Винтовка почти не прыгает. Ночью сильная вспышка.",
    effects: { acc: { mid: 0.03 }, crit: 0.03, nightAcc: -0.04 },
  },

  // Handguard and modules.
  {
    id: "rail-luch", kind: "mod", slot: "rail", fits: [...P, ...S, ...R], name: "Фонарь «Луч»", tier: 1,
    note: "Тактический фонарь: ночью видно, куда стреляешь.",
    effects: { nightAcc: 0.05 },
  },
  {
    id: "rail-lcu", kind: "mod", slot: "rail", fits: P, name: "Лазер ЛЦУ-2", tier: 2,
    note: "Лазерная точка помогает вести бегущую цель.",
    effects: { acc: { near: 0.02 }, tracking: 0.04 },
  },
  {
    id: "rail-kulak", kind: "mod", slot: "rail", fits: [...S, ...R, ...N], name: "Рукоять «Кулак»", tier: 2,
    note: "Вертикальная рукоять: легче довести ствол за бегущим врагом.",
    effects: { tracking: 0.06 },
  },
  {
    id: "rail-koster", kind: "mod", slot: "rail", fits: R, name: "Подствольник «Костёр»", tier: 3,
    note: "Подствольный гранатомёт: ещё одна граната за бой. Тяжёлый.",
    effects: { grenades: 1, dodge: -0.01 },
  },
  {
    id: "rail-upor", kind: "mod", slot: "rail", fits: N, name: "Сошки «Упор»", tier: 2,
    note: "Из стойки винтовка лежит как влитая, а без стойки с ними ещё неудобнее.",
    effects: { acc: { mid: 0.03, far: 0.05 }, noStance: -0.03 },
  },

  // Magazines and ammo.
  {
    id: "mag-pistol-long", kind: "mod", slot: "mag", fits: P, name: "Удлинённый магазин", tier: 2,
    note: "Больше патронов: иногда успеваешь выстрелить дважды.",
    effects: { doubleShot: 0.04 },
  },
  {
    id: "mag-pistol-hp", kind: "mod", slot: "mag", fits: P, name: "Экспансивные патроны", tier: 2,
    note: "Сильнее по открытым местам, хуже по броне.",
    effects: { power: 2, pen: -4 },
  },
  {
    id: "mag-pistol-ap", kind: "mod", slot: "mag", fits: P, name: "Бронебойные патроны", tier: 2,
    note: "Прошивают бронежилет, но реже дают крит.",
    effects: { pen: 6, crit: -0.03 },
  },
  {
    id: "mag-shotgun-box", kind: "mod", slot: "mag", fits: S, name: "Коробчатый магазин на 8", tier: 2,
    note: "Быстрая перезарядка: иногда успеваешь выстрелить дважды.",
    effects: { doubleShot: 0.04 },
  },
  {
    id: "mag-shotgun-slug", kind: "mod", slot: "mag", fits: S, name: "Пули «Жакан»", tier: 2,
    note: "Одна тяжёлая пуля вместо дроби: дальше летит, но вплотную слабее.",
    effects: { falloff: { near: -0.15, mid: 0.2 }, acc: { mid: 0.04 } },
  },
  {
    id: "mag-shotgun-buck", kind: "mod", slot: "mag", fits: S, name: "Картечь «Волчья»", tier: 3,
    note: "Крупная картечь: вплотную ещё страшнее, дальше слабее.",
    effects: { falloff: { near: 0.1, mid: -0.1 } },
  },
  {
    id: "mag-rifle-twin", kind: "mod", slot: "mag", fits: R, name: "Спаренные магазины", tier: 2,
    note: "Два магазина на скобе: иногда очередь попадает дважды.",
    effects: { doubleShot: 0.05 },
  },
  {
    id: "mag-rifle-ap", kind: "mod", slot: "mag", fits: R, name: "Бронебойные 7Н", tier: 2,
    note: "Прошивают бронежилет, но пуля чуть слабее.",
    effects: { pen: 6, power: -1 },
  },
  {
    id: "mag-rifle-drum", kind: "mod", slot: "mag", fits: R, name: "Барабан на 75", tier: 3,
    note: "Длинные очереди, но с ним тяжело бегать.",
    effects: { doubleShot: 0.08, dashEvade: -0.04 },
  },
  {
    id: "mag-sniper-10", kind: "mod", slot: "mag", fits: N, name: "Магазин на 10", tier: 2,
    note: "Иногда успеваешь выстрелить дважды.",
    effects: { doubleShot: 0.04 },
  },
  {
    id: "mag-sniper-ap", kind: "mod", slot: "mag", fits: N, name: "Бронебойные патроны", tier: 2,
    note: "Прошивают даже тяжёлую броню, но реже дают крит.",
    effects: { pen: 8, crit: -0.04 },
  },

  // Stocks and grips.
  {
    id: "stock-pistol-hvat", kind: "mod", slot: "stock", fits: P, name: "Накладки «Хват»", tier: 1,
    note: "Резиновые накладки: быстрый выстрел вблизи.",
    effects: { acc: { near: 0.03, far: -0.02 } },
  },
  {
    id: "stock-pistol-upor", kind: "mod", slot: "stock", fits: P, name: "Кобура-приклад «Упор-П»", tier: 2,
    note: "Пристёгивается к рукояти, и пистолет стреляет дальше. Вплотную мешает.",
    effects: { acc: { near: -0.03, mid: 0.03, far: 0.03 } },
  },
  {
    id: "stock-shotgun-fold", kind: "mod", slot: "stock", fits: S, name: "Складной приклад", tier: 1,
    note: "Короткий и ухватистый, но держит хуже.",
    effects: { tracking: 0.03, acc: { mid: -0.02 } },
  },
  {
    id: "stock-shotgun-otdacha", kind: "mod", slot: "stock", fits: S, name: "Приклад «Отдача»", tier: 2,
    note: "Амортизатор гасит отдачу. Тяжелее вести бегущую цель.",
    effects: { acc: { near: 0.02, mid: 0.03 }, tracking: -0.02 },
  },
  {
    id: "stock-rifle-opora", kind: "mod", slot: "stock", fits: R, name: "Приклад «Опора»", tier: 2,
    note: "Регулируемый приклад для стрельбы на дальность.",
    effects: { acc: { near: -0.02, mid: 0.03, far: 0.03 } },
  },
  {
    id: "stock-rifle-tesno", kind: "mod", slot: "stock", fits: R, name: "Укороченный приклад «Тесно»", tier: 2,
    note: "Для боя в цехах и подъездах: быстрее вблизи, хуже вдали.",
    effects: { acc: { near: 0.04, mid: -0.01, far: -0.04 } },
  },
  {
    id: "stock-sniper-izgotovka", kind: "mod", slot: "stock", fits: N, name: "Приклад «Изготовка»", tier: 3,
    note: "Подогнан под щёку: без стойки винтовка теряет меньше точности.",
    effects: { noStance: 0.12 },
  },
  {
    id: "stock-sniper-opora", kind: "mod", slot: "stock", fits: N, name: "Приклад «Опора-С»", tier: 2,
    note: "Регулируемый приклад: выстрел в уязвимое место чаще.",
    effects: { crit: 0.03 },
  },
];

const SKIN_LIST: SkinItem[] = [
  { id: "skin-factory", kind: "skin", name: "Заводская", tier: 1, note: "Воронёный металл и чёрный пластик.", effects: {} },
  { id: "skin-salt", kind: "skin", name: "Соль", tier: 1, note: "Грязно-белые разводы, как соляной иней.", effects: {} },
  { id: "skin-olive", kind: "skin", name: "Олива", tier: 1, note: "Армейская олива.", effects: {} },
  { id: "skin-rust", kind: "skin", name: "Ржавчина", tier: 2, note: "Рыжие пятна, как на технике в Ржавых полях.", effects: {} },
  { id: "skin-pixel", kind: "skin", name: "Городской пиксель", tier: 2, note: "Серо-бетонный цифровой камуфляж.", effects: {} },
];

// ---------- what the Persona wears ----------

const WEAR_LIST: WearItem[] = [
  // Head: armour against night and dust optics.
  { id: "head-cap", kind: "head", name: "Кепка", tier: 1, note: "Не защищает, зато на ней место под шеврон.", effects: {} },
  { id: "head-k6", kind: "head", name: "Каска К-6", tier: 1, note: "Простая стальная каска.", effects: { armor: { head: 15 }, dodge: -0.01 } },
  { id: "head-sfera", kind: "head", name: "Шлем «Сфера-М»", tier: 2, note: "Тяжёлый шлем с забралом.", effects: { armor: { head: 25 }, dodge: -0.02 } },
  { id: "head-sych", kind: "head", name: "Шлем с ПНВ «Сыч»", tier: 2, note: "Лёгкий шлем с прибором ночного видения.", effects: { armor: { head: 8 }, nvg: true } },
  { id: "head-marevo", kind: "head", name: "Шлем с тепловизором «Марево»", tier: 2, note: "Видит тепло сквозь пыль и дым.", effects: { armor: { head: 8 }, thermal: true } },
  { id: "head-filin", kind: "head", name: "Шлем «Филин»", tier: 3, note: "Крепкий шлем с ночным видением. Тяжёлый.", effects: { armor: { head: 20 }, nvg: true, dodge: -0.02 } },

  // Body: vest and rig.
  { id: "body-light", kind: "body", name: "Разгрузка «Лёгкая»", tier: 1, note: "Тонкая плита и много подсумков.", effects: { armor: { body: 10 }, grenades: 1, dodge: 0.02 } },
  { id: "body-plita2", kind: "body", name: "Бронежилет «Плита-2»", tier: 1, note: "Стандартная плита на корпус.", effects: { armor: { body: 20 } } },
  { id: "body-plita4", kind: "body", name: "Бронежилет «Плита-4»", tier: 2, note: "Плита, защита паха и подсумок под гранату.", effects: { armor: { body: 20, groin: 10 }, grenades: 1, dodge: -0.01 } },
  { id: "body-bastion", kind: "body", name: "Бронежилет «Бастион»", tier: 3, note: "Тяжёлая штурмовая броня. В ней тяжело бегать.", effects: { armor: { body: 32, groin: 15 }, dodge: -0.03, dashEvade: -0.03 } },

  // Suit: heat and camouflage. Colour and chevron are free to change.
  { id: "suit-field", kind: "suit", name: "Полевой костюм", tier: 1, note: "Базовый костюм. Красится в любой цвет.", effects: {} },
  { id: "suit-solonchak", kind: "suit", name: "Костюм «Солончак»", tier: 1, note: "Светлая ткань с охлаждением: щит не перегревается днём.", effects: { heatProof: true } },
  { id: "suit-leshy", kind: "suit", name: "Маскхалат «Леший»", tier: 2, note: "Издали теряешься на фоне руин. В рывке цепляется.", effects: { camo: { far: 0.06 }, dashEvade: -0.03 } },
  { id: "suit-ryvok", kind: "suit", name: "Комбинезон «Рывок»", tier: 2, note: "Штурмовой комбинезон: легче сделать рывок, но издали заметнее.", effects: { dashEvade: 0.03, camo: { far: -0.02 } } },
  { id: "suit-ten", kind: "suit", name: "Термокостюм «Тень»", tier: 3, note: "Прячет тепло и охлаждает щит.", effects: { heatProof: true, camo: { mid: 0.03, far: 0.04 } } },

  // Boots: leg armour and dashes.
  { id: "boots-bertsy", kind: "boots", name: "Берцы", tier: 1, note: "Армейские ботинки с защитой голени.", effects: { armor: { legs: 10 } } },
  { id: "boots-sprint", kind: "boots", name: "Кроссовки «Спринт»", tier: 1, note: "Лёгкие, без защиты: быстрее уходишь из-под выстрела.", effects: { dodge: 0.02, dashEvade: 0.05 } },
  { id: "boots-hodok", kind: "boots", name: "Ботинки «Ходок»", tier: 2, note: "Защита голени и цепкая подошва для рывка.", effects: { armor: { legs: 10 }, dashEvade: 0.05 } },
  { id: "boots-saper", kind: "boots", name: "Сапоги «Сапёр»", tier: 3, note: "Бронированные сапоги. Тяжёлые.", effects: { armor: { legs: 20 }, dodge: -0.01 } },

  // Knife.
  { id: "knife-std", kind: "knife", name: "Штатный нож", tier: 1, note: "Обычный армейский нож.", effects: {} },
  { id: "knife-shtyk", kind: "knife", name: "Штык-нож", tier: 1, note: "Узкий клинок проходит между пластинами.", effects: { pen: 8 } },
  { id: "knife-zhalo", kind: "knife", name: "Нож «Жало»", tier: 2, note: "Тяжёлый клинок: урон выше.", effects: { power: 5 } },
  { id: "knife-kogot", kind: "knife", name: "Керамбит «Коготь»", tier: 2, note: "Изогнутый клинок: чаще попадает в уязвимое место.", effects: { crit: 0.12 } },
  { id: "knife-iskra", kind: "knife", name: "Термонож «Искра»", tier: 3, note: "Раскалённое лезвие плохо держит щит.", effects: { guard: 0.1, power: 2 } },
];

const GRENADE_LIST: GrenadeItem[] = [
  {
    id: "grenade-rg1", kind: "grenade", name: "Граната РГ-1", tier: 1,
    note: "Осколочная. Бьёт мимо щита по тому, кто стоит на месте.",
    effects: {}, grenade: { type: "frag", base: 35, armorPen: 0.24, minDistance: 2, maxDistance: 8 },
  },
  {
    id: "grenade-grad", kind: "grenade", name: "Граната «Град»", tier: 2,
    note: "Тяжёлая осколочная: урон выше, но летит только на 3–7 шагов.",
    effects: {}, grenade: { type: "frag", base: 42, armorPen: 0.3, minDistance: 3, maxDistance: 7 },
  },
  {
    id: "grenade-emp", kind: "grenade", name: "ЭМИ-граната «Затмение»", tier: 2,
    note: "Урон слабее осколочной, но щит врага выключен на следующий ход.",
    effects: {}, grenade: { type: "emp", base: 22, armorPen: 0.24, minDistance: 2, maxDistance: 8 },
  },
  {
    id: "grenade-smoke", kind: "grenade", name: "Дымовая «Завеса»", tier: 1,
    note: "Дым под ноги: в этот и следующий ход по тебе трудно попасть. Можно подойти или уйти.",
    effects: {}, grenade: { type: "smoke", base: 0, armorPen: 0, minDistance: 1, maxDistance: 99 },
  },
];

export const ITEMS: readonly Item[] = [...WEAPON_LIST, ...MOD_LIST, ...SKIN_LIST, ...WEAR_LIST, ...GRENADE_LIST];

export const ITEM_BY_ID: ReadonlyMap<string, Item> = new Map(ITEMS.map((i) => [i.id, i]));

export function getItem(id: string): Item {
  const item = ITEM_BY_ID.get(id);
  if (!item) throw new Error(`Unknown item: ${id}`);
  return item;
}

/** Which gear slot each kind of item goes into. Weapons go into weapon1 or weapon2. */
export const KIND_FOR_SLOT: Record<Exclude<GearSlot, "weapon1" | "weapon2">, WearItem["kind"] | "grenade"> = {
  head: "head",
  body: "body",
  suit: "suit",
  boots: "boots",
  knife: "knife",
  grenade: "grenade",
};

/** All mods that fit a weapon in a slot, factory part first. */
export function modsFor(weapon: WeaponId, slot: ModSlot): ModItem[] {
  return MOD_LIST.filter((m) => m.slot === slot && m.fits.includes(weapon))
    .sort((a, b) => Number(!!b.isStock) - Number(!!a.isStock) || a.tier - b.tier);
}

/** The base weapon item of a class. */
export function weaponItem(weapon: WeaponId): WeaponItem {
  const item = WEAPON_LIST.find((w) => w.weapon === weapon);
  if (!item) throw new Error(`No weapon item for ${weapon}`);
  return item;
}
