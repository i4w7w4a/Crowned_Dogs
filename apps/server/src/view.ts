// What the Telegram bot shows: message text (Telegram HTML) and button rows.
// Plain data, so it can be tested without Telegram.

import {
  TIME_NAMES, WEAPONS, WEATHER, ZONES, ZONE_NAMES, canFlee, canThrowGrenade, swapWeapon,
  distanceLimit, draftChance, guardsNeeded, missing, other, turnLimit, type Action, type AttackReport, type Command,
  type Draft, type Side, type TurnReport, type WeaponId, type Zone,
} from "@crowned-dogs/core";
import { encodeCommand } from "./codes.ts";
import type { Duel } from "./duels.ts";

export interface Button {
  text: string;
  /** Callback data sent back on a tap. */
  data: string;
  /** Opens a link instead of sending data. */
  url?: string;
}

export interface Screen {
  text: string;
  buttons: Button[][];
}

const ZONE_LABEL: Record<Zone, string> = { head: "Голова", body: "Корпус", groin: "Пах", legs: "Ноги" };
/** "Взять снайперку": the weapon after a verb. */
const WEAPON_ACC: Record<WeaponId, string> = { pistol: "пистолет", shotgun: "дробовик", rifle: "автомат", sniper: "снайперку" };
/** "Выстрел в голову": the zone after «в». */
const ZONE_INTO: Record<Zone, string> = { head: "голову", body: "корпус", groin: "пах", legs: "ноги" };

export function escapeHtml(s: string): string {
  return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
}

/** Russian plural: 1 шаг, 2 шага, 5 шагов. */
function plural(n: number, one: string, few: string, many: string): string {
  const a = Math.abs(n) % 100, b = a % 10;
  const word = a > 10 && a < 20 ? many : b === 1 ? one : b >= 2 && b <= 4 ? few : many;
  return `${n} ${word}`;
}

export const steps = (n: number): string => plural(n, "шаг", "шага", "шагов");

/** Callback data for a turn button. Stays far below Telegram's 64-byte limit. */
export function turnData(duel: Duel, payload: string): string {
  return `t:${duel.id}:${duel.match.turn}:${payload}`;
}

const btn = (text: string, data: string): Button => ({ text, data });
const cmd = (duel: Duel, text: string, c: Command): Button => btn(text, turnData(duel, encodeCommand(c)));

function hp(duel: Duel, side: Side): string {
  const f = duel.match.fighters[side];
  return `${f.hp}/${f.maxHp}`;
}

function conditions(duel: Duel): string {
  const m = duel.match;
  return `${m.map.name} · ${TIME_NAMES[m.config.time]}, ${WEATHER[m.config.weather].name}`;
}

function chanceLine(duel: Duel, side: Side, d: Draft): string {
  const p = draftChance(duel.match, side, d);
  return p === null ? "" : ` · попадание ~${Math.round(p * 100)}%`;
}

function moveWords(move: number): string {
  if (move === 0) return "стоять";
  return move > 0 ? `вперёд на ${steps(move)}` : `назад на ${steps(-move)}`;
}

function planText(duel: Duel, side: Side, d: Draft): string {
  switch (d.kind) {
    case "shoot":
      return d.zone ? `выстрел в ${ZONE_INTO[d.zone]}, ${moveWords(d.move)}${chanceLine(duel, side, d)}` : `выстрел, ${moveWords(d.move)}`;
    case "knife":
      return d.zone ? `нож в ${ZONE_INTO[d.zone]}, ${moveWords(d.move)}${chanceLine(duel, side, d)}` : `нож, ${moveWords(d.move)}`;
    case "grenade":
      return `граната, ${moveWords(d.move)}`;
    case "swap": {
      const next = swapWeapon(duel.match, side);
      return `взять ${next ? WEAPON_ACC[next] : "другое оружие"}, без выстрела, ${moveWords(d.move)}`;
    }
    case "run":
      return `перебежка ${d.move > 0 ? "вперёд" : "назад"} на 2 шага, без выстрела`;
    case "dash":
      return "рывок вперёд на 3 шага: без выстрела, щит на одну зону, попасть по тебе труднее";
    case "flee":
      return "уйти из боя (половина добычи остаётся у тебя)";
  }
}

/** The screen where a player puts together this turn's order. */
export function turnScreen(duel: Duel, side: Side): Screen {
  const m = duel.match;
  const me = m.fighters[side], foe = m.fighters[other(side)];
  const d = duel.drafts[side];
  const ready = duel.orders[side] !== null;
  const lines = [
    `<b>Ход ${m.turn} из ${turnLimit(m)}</b> · ${steps(m.distance)} до врага`,
    `<i>${escapeHtml(conditions(duel))}</i>`,
    "",
    `<b>Ты</b> · ${WEAPONS[me.weapon].name} · ${hp(duel, side)}`,
    `<b>${escapeHtml(foe.name)}</b> · ${WEAPONS[foe.weapon].name} · ${hp(duel, other(side))}`,
    "",
    `Атака: ${planText(duel, side, d)}`,
    `Щит: ${d.defend.length ? d.defend.map((z) => ZONE_NAMES[z]).join(", ") : "—"} (${d.defend.length}/${guardsNeeded(d)})`,
  ];
  if (m.distance >= distanceLimit(m)) lines.push("", "<i>Ты у края карты: можно уйти из боя.</i>");
  if (ready) {
    lines.push("", "✅ Приказ отдан. Ждём соперника.");
    return { text: lines.join("\n"), buttons: [] };
  }

  const attackRow: Button[] = [btn("🎯", turnData(duel, "x"))];
  const guardRow: Button[] = [btn("🛡", turnData(duel, "x"))];
  for (const z of ZONES) {
    const aimed = d.zone === z && (d.kind === "shoot" || d.kind === "knife");
    attackRow.push(cmd(duel, aimed ? `● ${ZONE_LABEL[z]}` : ZONE_LABEL[z], { type: "zone", zone: z }));
    guardRow.push(cmd(duel, d.defend.includes(z) ? `✓ ${ZONE_LABEL[z]}` : ZONE_LABEL[z], { type: "guard", zone: z }));
  }
  const withShot = d.kind === "shoot" || d.kind === "knife" || d.kind === "grenade" || d.kind === "swap";
  const mark = (on: boolean, text: string) => (on ? `● ${text}` : text);
  const stepRow: Button[] = [
    cmd(duel, mark(withShot && d.move === -1, "← Шаг назад"), { type: "step", move: -1 }),
    cmd(duel, mark(withShot && d.move === 0, "Стоять"), { type: "step", move: 0 }),
    cmd(duel, mark(withShot && d.move === 1, "Шаг вперёд →"), { type: "step", move: 1 }),
  ];
  const runRow: Button[] = [
    cmd(duel, mark(d.kind === "run" && d.move < 0, "⇇ Назад 2"), { type: "run", move: -2 }),
    cmd(duel, mark(d.kind === "run" && d.move > 0, "Вперёд 2 ⇉"), { type: "run", move: 2 }),
    cmd(duel, mark(d.kind === "dash", "Рывок 3 ⇶"), { type: "dash" }),
  ];
  const extra: Button[] = [];
  if (m.distance <= 3) extra.push(cmd(duel, mark(d.kind === "knife", "🔪 Нож"), { type: "knife" }));
  if (canThrowGrenade(m, side)) extra.push(cmd(duel, mark(d.kind === "grenade", "💣 Граната"), { type: "grenade" }));
  const spare = swapWeapon(m, side);
  if (spare) extra.push(cmd(duel, mark(d.kind === "swap", `🔁 Взять ${WEAPON_ACC[spare]}`), { type: "swap" }));
  if (canFlee(m)) extra.push(cmd(duel, mark(d.kind === "flee", "🚪 Уйти из боя"), { type: "flee" }));

  const gap = missing(d);
  const rows = [attackRow, guardRow, stepRow, runRow];
  if (extra.length) rows.push(extra);
  rows.push([btn(gap ?? "✅ Готово", turnData(duel, "go"))]);
  return { text: lines.join("\n"), buttons: rows };
}

// ---------- after a turn ----------

function attackWords(a: Action, r: AttackReport | null, move: number): string {
  switch (a.type) {
    case "run": return `перебежка ${move > 0 ? "вперёд" : "назад"}`;
    case "dash": return "рывок вперёд";
    case "flee": return "уходит из боя";
    case "swap": return `смена оружия${move > 0 ? ", шаг вперёд" : move < 0 ? ", шаг назад" : ""}`;
    case "wait": return "пропустил ход";
    default: break;
  }
  if (!r) return "без атаки";
  const step = move > 0 ? ", шаг вперёд" : move < 0 ? ", шаг назад" : "";
  if (r.weapon === "grenade") {
    return r.hit ? `граната${step}: −${r.damage}` : `граната${step}: мимо, цель сдвинулась`;
  }
  const what = `${r.weapon === "knife" ? "нож" : "выстрел"} в ${r.zone ? ZONE_INTO[r.zone] : "корпус"}${step}`;
  if (!r.hit) return `${what}: мимо (${Math.round(r.chance * 100)}%)`;
  const notes = [r.crit ? "крит" : "", r.blocked ? (r.shieldBroken ? "щит пробит" : "щит принял") : ""].filter(Boolean);
  return `${what}: −${r.damage}${notes.length ? ` (${notes.join(", ")})` : ""}`;
}

export function turnReport(duel: Duel, side: Side, r: TurnReport): string {
  const foe = other(side);
  const names = ["", ""] as [string, string];
  names[side] = "Ты";
  names[foe] = escapeHtml(duel.players[foe].name);
  const dist = r.distanceBefore === r.distanceAfter ? steps(r.distanceAfter) : `${r.distanceBefore} → ${steps(r.distanceAfter)}`;
  const lines = [
    `<b>Ход ${r.turn}</b> · ${dist}`,
    `${names[side]}: ${attackWords(r.actions[side], r.attacks[side], r.moves[side])}`,
    `${names[foe]}: ${attackWords(r.actions[foe], r.attacks[foe], r.moves[foe])}`,
    `Ты ${r.hp[side]} · ${names[foe]} ${r.hp[foe]}`,
  ];
  return lines.join("\n");
}

export function finalScreen(duel: Duel, side: Side): Screen {
  const res = duel.match.result!;
  const foe = duel.players[other(side)];
  const won = res.winner === side;
  let text: string;
  if (res.winner === null) {
    text = res.reason === "double_ko"
      ? "<b>Ничья.</b> Обе Персоны выведены из строя одновременно."
      : res.reason === "fled" ? "<b>Ничья.</b> Оба ушли из боя." : "<b>Ничья.</b> Время вышло, здоровья поровну.";
  } else if (won) {
    const why = res.reason === "fled" ? `${escapeHtml(foe.name)} ушёл из боя.`
      : res.reason === "afk" ? `${escapeHtml(foe.name)} три хода не отвечал.`
      : res.reason === "turn_limit" ? "Время вышло, у тебя больше здоровья."
      : `${escapeHtml(foe.name)} выведен из строя за ${plural(duel.match.turn, "ход", "хода", "ходов")}.`;
    text = `<b>Цель подавлена.</b> ${why} Добыча твоя.`;
  } else {
    const why = res.reason === "fled" ? "Ты ушёл из боя и сохранил половину добычи."
      : res.reason === "afk" ? "Ты три хода не отдавал приказ."
      : res.reason === "turn_limit" ? "Время вышло, у соперника больше здоровья."
      : "Персона уничтожена. Ты очнёшься в капсуле на базе «Соль».";
    text = `<b>Связь потеряна.</b> ${why}`;
  }
  return { text, buttons: menuButtons() };
}

// ---------- contract ----------

export const CONTRACT = [
  "<b>Контракт оператора</b>",
  "",
  "Ты поступаешь на службу в ЧВК на несколько лет. Сам ты живёшь на передовой базе, а в бой идёт Персона, боевой робот с твоим лицом. Ты чувствуешь всё, что чувствует она, и её смерть тоже. Отказаться от боя нельзя: это ведь не твоя смерть.",
  "",
  "Многие здесь подписали контракт прямо из тюрьмы. О прошлом не спрашивают.",
  "",
  "После подписи тебе выдадут жетон с личным номером.",
].join("\n");

/** After signing, the bot carries on with what the player wanted: an invite, a fight or a duel. */
export type AfterContract = "" | "bot" | "duel" | `duel_${string}`;

export function contractButtons(next: AfterContract = ""): Button[][] {
  return [[btn("✍️ Подписать", next ? `c:${next}` : "c")]];
}

export function signedText(tag: string): string {
  return `Контракт подписан. Твой жетон: <b>${tag}</b>.\nЭто твой номер в игре: по нему тебя найдут и в боте, и в клиенте.`;
}

export function tagText(tag: string, name: string): string {
  return `Жетон <b>${tag}</b>, позывной <b>${escapeHtml(name)}</b>.`;
}

/** "<b>Аня</b> (K-417-093)", or just the name for a bot. */
export function nameWithTag(p: { name: string; tag?: string }): string {
  return `<b>${escapeHtml(p.name)}</b>${p.tag ? ` (${p.tag})` : ""}`;
}

// ---------- menus ----------

export function menuButtons(): Button[][] {
  return [[btn("⚔️ Бой с ботом", "m:bot"), btn("🤝 Дуэль с другом", "m:duel")]];
}

export type PickMode = "bot" | "duel" | `join:${string}`;

/** Veteran kits: until the gear screen exists, the only way to fight with mods. */
export function weaponPicker(mode: PickMode, veteran = false): Button[][] {
  const hint: Record<WeaponId, string> = { pistol: "3–5", shotgun: "1–3", rifle: "5–8", sniper: "7+" };
  const kit = veteran ? ":v" : "";
  const row = (a: WeaponId, b: WeaponId) => [a, b].map((w) => btn(`${WEAPONS[w].name} · ${hint[w]}`, `w:${mode}:${w}${kit}`));
  return [
    row("pistol", "shotgun"),
    row("rifle", "sniper"),
    [btn(veteran ? "● 🎖 Набор ветерана" : "🎖 Набор ветерана", `wk:${mode}:${veteran ? "s" : "v"}`)],
  ];
}

export const VETERAN_NOTE = "Набор ветерана: оружие с обвесами, броня получше, у снайпера запасной пистолет. Бот получит такой же.";

export const WELCOME = [
  "<b>Crowned Dogs</b> — пошаговые дуэли Персон.",
  "",
  "Каждый ход оба тайно выбирают атаку (куда стрелять), щит на две зоны и движение. Потом ход разыгрывается одновременно.",
  "",
  "/fight — бой с ботом",
  "/duel — ссылка, чтобы позвать друга",
  "/rules — коротко о правилах",
  "/me — твой жетон",
].join("\n");

export const RULES = [
  "<b>Правила</b>",
  "• У каждого 100 здоровья, старт в 7 шагах. На ход 60 секунд.",
  "• Атака: зона на враге. Голова бьёт сильнее всего, ноги слабее всего.",
  "• Щит прикрывает 2 зоны и пропускает около четверти урона. Нож проходит щит наполовину.",
  "• С выстрелом можно сделать шаг вперёд или назад.",
  "• Перебежка на 2 шага и рывок на 3 шага вперёд заменяют выстрел. При рывке щит только на одну зону, зато попасть по тебе труднее.",
  "• Нож бьёт с 2 шагов. Граната бьёт мимо щита, но не задевает того, кто убежал. Обычная граната одна на бой и летит на 2–8 шагов.",
  "• Если в наборе есть второй ствол, его можно взять вместо атаки. Шаг и щит при этом остаются.",
  "• После 6-го хода поле сжимается. Бой длится до 15 ходов (в грозу 10), потом побеждает тот, у кого больше здоровья.",
  "• Снайперке нужно стоять два хода подряд, иначе точность вдвое ниже.",
  "• Три пропущенных хода подряд — поражение.",
].join("\n");
