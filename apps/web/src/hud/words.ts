// What the fight screen says, in Russian. Pure functions over the core's turn
// reports, so the log and the end screen are tested with node.

import {
  other, type Action, type AttackReport, type MatchResult, type Side, type TimeOfDay, type TurnReport, type Weather,
  type WeaponId, type Zone,
} from "@crowned-dogs/core";

export function plural(n: number, one: string, few: string, many: string): string {
  const a = Math.abs(n) % 100, b = a % 10;
  if (a > 10 && a < 20) return many;
  if (b === 1) return one;
  if (b >= 2 && b <= 4) return few;
  return many;
}
export const steps = (n: number) => plural(n, "шаг", "шага", "шагов");
export const turnsWord = (n: number) => plural(n, "ход", "хода", "ходов");

export const ZONE_TO: Record<Zone, string> = { head: "в голову", body: "в корпус", groin: "в пах", legs: "по ногам" };
/** Who takes what: "взять пистолет". */
export const GUN_ACC: Record<WeaponId, string> = { pistol: "пистолет", shotgun: "дробовик", rifle: "автомат", sniper: "снайперку" };
/** With what: "Искра с дробовиком", "со снайперкой". */
export const GUN_WITH: Record<WeaponId, string> = { pistol: "с пистолетом", shotgun: "с дробовиком", rifle: "с автоматом", sniper: "со снайперкой" };

export const TIME_WORD: Record<TimeOfDay, string> = { morning: "Утро", day: "День", evening: "Вечер", night: "Ночь" };
export const WEATHER_WORD: Record<Weather, string> = { clear: "Ясно", dust: "Пыль", storm: "Гроза", fog: "Туман" };

/** Steps of one side's move, in words. Empty when it stood still. */
export function moveWord(action: Action): string {
  switch (action.type) {
    case "dash": return "рывок";
    case "run": return action.move > 0 ? "перебежка вперёд" : "перебежка назад";
    case "flee": return "уход из боя";
    case "wait": return "";
    default: return action.move > 0 ? "шаг вперёд" : action.move < 0 ? "шаг назад" : "";
  }
}

export type Outcome =
  | { kind: "none"; text: string }
  | { kind: "miss" }
  | { kind: "blocked"; damage: number }
  | { kind: "hit"; damage: number; crit: boolean };

/** One side's line of a turn in the log. */
export interface Line {
  icon: "shot" | "knife" | "grenade" | "swap" | "move";
  /** The move icon points back. */
  back: boolean;
  dash: boolean;
  act: string;
  outcome: Outcome;
  /** What the hit landed on, for the short ticker: a zone or "граната". */
  target: string;
}

const grenadeName = (a: AttackReport | null) =>
  a?.effect === "emp" ? "ЭМИ-граната" : a?.effect === "smoke" ? "дымовая граната" : "граната";

/**
 * What one side did on this turn and what came of it.
 * `inHand` is the gun that side holds after the turn: after a swap, the one it took.
 */
export function turnLine(r: TurnReport, side: Side, inHand: WeaponId): Line {
  const action = r.actions[side], a = r.attacks[side], mv = moveWord(action);
  const join = (...parts: string[]) => parts.filter(Boolean).join(", ");
  const outcome = (): Outcome => {
    if (!a) return { kind: "none", text: "без атаки" };
    if (!a.hit) return { kind: "miss" };
    if (a.blocked) return { kind: "blocked", damage: a.damage };
    return { kind: "hit", damage: a.damage, crit: a.crit };
  };
  const target = a?.weapon === "grenade" ? "граната" : a?.zone ? zoneName(a.zone) : "";
  const base = { back: false, dash: false, target };
  switch (action.type) {
    case "shoot":
    case "knife":
      return { ...base, icon: action.type === "knife" ? "knife" : "shot", act: join(mv, (action.type === "knife" ? "нож " : "выстрел ") + ZONE_TO[action.zone]), outcome: outcome() };
    case "grenade":
      return { ...base, icon: "grenade", act: join(mv, grenadeName(a)), outcome: outcome() };
    case "swap":
      return { ...base, icon: "swap", act: join(mv, "взять " + GUN_ACC[inHand]), outcome: { kind: "none", text: "без выстрела" } };
    case "wait":
      return { ...base, icon: "move", act: "приказ не отдан", outcome: { kind: "none", text: "пропуск" } };
    default:
      return { ...base, icon: "move", back: moveOf(action) < 0, dash: action.type === "dash", act: mv, outcome: outcome() };
  }
}

const moveOf = (a: Action) => (a.type === "run" ? a.move : a.type === "dash" ? 3 : 0);
const ZONE_NAME: Record<Zone, string> = { head: "голова", body: "корпус", groin: "пах", legs: "ноги" };
export const zoneName = (z: Zone) => ZONE_NAME[z];

/** The end screen: title, text and the button. */
export function endWords(res: MatchResult, me: Side, foe: string, turns: number): { title: string; text: string; again: string; lost: boolean } {
  const won = res.winner === me, lost = res.winner === other(me);
  const title = lost ? "Связь потеряна" : won ? "Цель подавлена" : "Ничья";
  let text: string;
  if (res.winner === null) {
    text = res.reason === "double_ko" ? "Обе Персоны выведены из строя одновременно."
      : res.reason === "fled" ? "Обе Персоны ушли из боя." : "Время вышло, здоровья поровну.";
  } else if (won) {
    text = res.reason === "fled" ? `Персона «${foe}» ушла из боя.`
      : res.reason === "afk" ? "Соперник три хода не отвечал."
      : res.reason === "turn_limit" ? "Время вышло, у тебя больше здоровья."
      : `Персона «${foe}» выведена из строя за ${turns} ${turnsWord(turns)}. Добыча твоя.`;
  } else {
    text = res.reason === "fled" ? "Персона ушла из боя и сохранила половину добычи."
      : res.reason === "afk" ? "Три хода подряд без приказа: связь разорвана."
      : res.reason === "turn_limit" ? "Время вышло, у соперника больше здоровья."
      : "Персона уничтожена. Ты очнёшься в капсуле на базе «Соль».";
  }
  return { title, text, again: lost ? "Очнуться в капсуле" : "Новый бой", lost };
}
