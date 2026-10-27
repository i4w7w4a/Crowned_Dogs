// The fight screen: a duel against the bot on the core rules, shown in the 3D arena.
// The DOM HUD reads the duel after every tap; the turn plays out on a short timeline
// (both walk, our attack lands, theirs lands, next turn).

import {
  TIME_NAMES, WEAPONS, WEATHER, ZONES, ZONE_NAMES, canThrowGrenade, canUseKnife, grenadeSpec, turnLimit, turnSeconds,
  type AttackReport, type Side, type TurnReport, type Zone,
} from "@crowned-dogs/core";
import { Arena, type HitKind } from "./arena/arena.ts";
import { CALIBRATION_FRAME, FRAMES, type Frame } from "./arena/frames.ts";
import { STATIONS, Sound } from "./audio/sound.ts";
import { LocalDuel, MOVE_MAX, MOVE_MIN } from "./game/local-duel.ts";
import "./style.css";

/** Music, effects and the model live outside git (see README); pictures ship with the app. */
const ASSETS = import.meta.env.VITE_ASSETS ?? "media/";
const MODEL = import.meta.env.VITE_MODEL ?? `${ASSETS}models/mannequin.glb`;

const PLAYER = { name: "Тень", weapon: "rifle", tag: "K-417-093" } as const;
const BOT = { name: "Искра", weapon: "rifle" } as const;

const $ = <T extends HTMLElement = HTMLElement>(id: string) => document.getElementById(id) as T;
const scene = $("scene"), stage = $("stage"), bg = $<HTMLImageElement>("bg"), ringsEl = $("rings"), canvas = $<HTMLCanvasElement>("gl");
const reduceMotion = window.matchMedia?.("(prefers-reduced-motion: reduce)").matches ?? false;

const frames: Frame[] = /[?&]cal\b/.test(location.search) ? [CALIBRATION_FRAME, ...FRAMES] : FRAMES;
let frame = frames[0]!;
let duel = newDuel();
let busy = false;
let secondsLeft = 0;
let timerId: ReturnType<typeof setInterval> | null = null;
let geo = { x: 0, y: 0, w: 1, h: 1 };

const sound = new Sound(ASSETS);
let arena: Arena | null = null;

function newDuel(): LocalDuel {
  return new LocalDuel({ mapId: frame.mapId, weather: frame.weather, time: frame.time, player: { ...PLAYER }, bot: { ...BOT } });
}

// ---------- words ----------
function plural(n: number, one: string, few: string, many: string): string {
  const a = Math.abs(n) % 100, b = a % 10;
  if (a > 10 && a < 20) return many;
  if (b === 1) return one;
  if (b >= 2 && b <= 4) return few;
  return many;
}
const steps = (n: number) => plural(n, "шаг", "шага", "шагов");
const capital = (s: string) => s.charAt(0).toUpperCase() + s.slice(1);

// ---------- rings on the enemy ----------
const ringEls = {} as Record<Zone, HTMLButtonElement>;
for (const z of ZONES) {
  const b = document.createElement("button");
  b.type = "button"; b.className = "ring hint"; b.setAttribute("aria-label", "Атака: " + ZONE_NAMES[z]);
  b.addEventListener("click", () => tap(() => duel.press({ type: "zone", zone: z })));
  ringsEl.appendChild(b); ringEls[z] = b;
}
const tagEl = document.createElement("div");
tagEl.className = "tag"; tagEl.hidden = true;
ringsEl.appendChild(tagEl);

/** Where a zone of the enemy is in stage pixels. */
function point(z: Zone): [number, number] {
  const [u, v] = arena ? arena.zoneOnScreen(z) : [0.645, { head: 0.26, body: 0.34, groin: 0.42, legs: 0.49 }[z]];
  return [geo.x + u * geo.w, geo.y + v * geo.h];
}

function placeRings(): void {
  for (const z of ZONES) { const [x, y] = point(z); ringEls[z].style.transform = `translate(${x.toFixed(1)}px, ${y.toFixed(1)}px)`; }
  const zone = duel.draft.zone;
  if (!tagEl.hidden && zone) {
    // the label sits right of the ring, or left of it when it would run off the screen
    const [x, y] = point(zone), w = tagEl.offsetWidth, right = x + 26 + w <= scene.clientWidth - 8;
    tagEl.style.transform = `translate(${(right ? x + 26 : x - 26 - w).toFixed(1)}px, ${y.toFixed(1)}px) translateY(-50%)`;
  }
}

function layout(): void {
  const cw = scene.clientWidth, top = scene.clientHeight * frame.top, ch = scene.clientHeight - top;
  const s = Math.max(cw / frame.w, ch / frame.h);
  const w = frame.w * s, h = frame.h * s;
  geo = { x: (cw - w) * 0.5, y: top, w, h };
  for (const el of [bg, canvas]) Object.assign(el.style, { width: w + "px", height: h + "px", left: geo.x + "px", top: geo.y + "px" });
  arena?.resize(w, h);
  placeRings();
}

// ---------- defence, movement, specials ----------
const guards = [...document.querySelectorAll<HTMLButtonElement>(".dz")];
for (const b of guards) b.addEventListener("click", () => tap(() => duel.press({ type: "guard", zone: b.dataset.z as Zone })));
$("backStep").addEventListener("click", () => tap(() => duel.setMove(duel.draft.move - 1)));
$("fwdStep").addEventListener("click", () => tap(() => duel.setMove(duel.draft.move + 1)));
$("knifeBtn").addEventListener("click", () => tap(() => duel.press({ type: "knife" })));
$("nadeBtn").addEventListener("click", () => tap(() => duel.press({ type: "grenade" })));

/** Runs one tap on the order. A refused tap shows its reason in the log line. */
function tap(fn: () => string | null): void {
  if (busy || duel.over) return;
  const err = fn();
  if (err) say(err);
  render();
}

// ---------- render ----------
function render(): void {
  const m = duel.match, d = duel.draft, me = m.fighters[duel.me], en = m.fighters[duel.bot];
  const run = d.kind === "run" || d.kind === "dash" || d.kind === "flee", dash = d.kind === "dash";
  const aimed = d.kind === "shoot" || d.kind === "knife";
  const step = Math.abs(d.move) <= 1;
  const knifeOk = step && (canUseKnife(m, d.move) || canUseKnife(m, 1));
  const nadeOk = step && canThrowGrenade(m, duel.me);

  ringsEl.classList.toggle("off", !aimed || busy);
  ringsEl.classList.toggle("gone", duel.over || (arena !== null && !arena.ready)); // without WebGL the rings stay on the picture
  for (const z of ZONES) {
    ringEls[z].classList.toggle("sel", aimed && d.zone === z);
    ringEls[z].classList.toggle("hint", !d.zone && aimed && m.turn === 1);
  }
  const chance = duel.chance;
  tagEl.hidden = !aimed || !d.zone || chance === null || busy || duel.over;
  if (!tagEl.hidden) {
    tagEl.innerHTML = (d.kind === "knife" ? "Нож · " : "") + ZONE_NAMES[d.zone!] + "<b>" + Math.round(chance! * 100) + "%</b>";
  }

  for (const b of guards) {
    const on = d.defend.includes(b.dataset.z as Zone);
    b.classList.toggle("on", on); b.disabled = busy || duel.over; b.setAttribute("aria-pressed", String(on));
  }
  $("defCount").textContent = `${d.defend.length}/${dash ? 1 : 2}`;

  const st = $("steps");
  const words = dash ? "рывок" : d.kind === "run" ? (d.move > 0 ? "перебежка вперёд" : "перебежка назад")
    : d.move > 0 ? "шаг вперёд" : d.move < 0 ? "шаг назад" : "на месте";
  st.innerHTML = (d.move > 0 ? "+" : "") + d.move + "<small>" + words + "</small>";
  st.classList.toggle("dash", run);
  const planned = duel.plannedDistance;
  $("distLabel").textContent = d.move && !busy ? `${m.distance} → ${planned} ${steps(planned)}` : `${m.distance} ${steps(m.distance)}`;
  ($("backStep") as HTMLButtonElement).disabled = busy || duel.over || d.move <= MOVE_MIN;
  ($("fwdStep") as HTMLButtonElement).disabled = busy || duel.over || d.move >= MOVE_MAX || m.distance - d.move <= 1;

  const kb = $<HTMLButtonElement>("knifeBtn"), nb = $<HTMLButtonElement>("nadeBtn");
  kb.disabled = !knifeOk || busy || duel.over; nb.disabled = !nadeOk || busy || duel.over;
  kb.classList.toggle("on", d.kind === "knife"); nb.classList.toggle("on", d.kind === "grenade");
  kb.setAttribute("aria-pressed", String(d.kind === "knife")); nb.setAttribute("aria-pressed", String(d.kind === "grenade"));
  $("knifeSub").textContent = knifeOk ? "вблизи" : "с 2 шагов";
  const nade = grenadeSpec(m, 0);
  $("nadeSub").textContent = me.grenades ? (nadeOk ? "мимо щита" : `${nade.minDistance}–${nade.maxDistance} шагов`) : "нет";

  const go = $<HTMLButtonElement>("goBtn");
  const msg = duel.over ? "Бой окончен" : busy ? "Ход идёт…" : duel.missing;
  go.disabled = !!msg;
  go.textContent = msg ?? (dash ? "Рывок и завершить ход" : d.kind === "run" ? "Перебежка и завершить ход" : d.kind === "flee" ? "Уйти из боя" : "Завершить ход");

  $("hpMe").textContent = String(Math.max(0, me.hp)); $("hpEn").textContent = String(Math.max(0, en.hp));
  $("barMe").style.width = Math.max(0, me.hp / me.maxHp * 100) + "%";
  $("barEn").style.width = Math.max(0, en.hp / en.maxHp * 100) + "%";
  $("turn").textContent = `Ход ${m.turn} из ${turnLimit(m)}`;
  drawTimer();
}

function drawHeader(): void {
  const m = duel.match;
  $("mapTitle").textContent = frame.name;
  $("cond").textContent = `${capital(TIME_NAMES[m.config.time])} · ${capital(WEATHER[m.config.weather].name)}`;
  $("nameMe").textContent = duel.fighters[0].name; $("nameEn").textContent = duel.fighters[1].name;
  $("gearMe").textContent = WEAPONS[duel.fighters[0].weapon].name; $("gearEn").textContent = WEAPONS[duel.fighters[1].weapon].name;
  $("recTag").textContent = duel.fighters[0].tag ?? "";
}

// ---------- timer ----------
function drawTimer(): void {
  const t = Math.max(0, secondsLeft), total = turnSeconds(duel.match);
  $("timer").textContent = `${Math.floor(t / 60)}:${t % 60 < 10 ? "0" : ""}${t % 60}`;
  $("dialArc").setAttribute("stroke-dashoffset", (157.08 * (1 - t / total)).toFixed(2));
  $("dial").classList.toggle("low", t <= 10);
}
function startTimer(): void {
  stopTimer();
  secondsLeft = turnSeconds(duel.match);
  drawTimer();
  timerId = setInterval(() => {
    if (busy || duel.over || !$("infoVeil").hidden) return;
    secondsLeft -= 1; drawTimer();
    if (secondsLeft > 0 && secondsLeft <= 5) sound.play("ui-tick");
    if (secondsLeft <= 0) playTurn(duel.timeout());
  }, 1000);
}
function stopTimer(): void {
  if (timerId) clearInterval(timerId);
  timerId = null;
}

// ---------- the turn ----------
function endTurn(): void {
  if (busy || duel.over) return;
  const r = duel.submit();
  if (typeof r === "string") { say(r); return; }
  playTurn(r);
}

const weaponOf = (a: AttackReport) => a.weapon === "grenade" ? "grenade" : a.weapon === "knife" ? "knife" : "gun";
const hitKind = (a: AttackReport): HitKind => a.weapon === "grenade" ? "grenade" : a.blocked ? "blocked" : "hit";

function playTurn(r: TurnReport): void {
  stopTimer(); busy = true;
  sound.play("ui-confirm");
  arena?.move(r.distanceAfter, r.moves[0], r.moves[1]); // both walk first, then the shots
  render();
  for (const side of [0, 1] as Side[]) if (r.actions[side].type === "grenade") sound.play("grenade-throw", side === 0 ? 300 : 900);

  const strike = (side: Side) => {
    const a = r.attacks[side];
    if (!a) return;
    const ours = side === duel.me, who = ours ? "enemy" : "me";
    sound.attack(weaponOf(a), a.hit, a.blocked, ours);
    if (ours) { fire($("flashOurs")); shake(); showOnEnemy(a); } else { if (a.hit && !a.blocked) fire($("flashHurt")); showOnGuard(a); }
    if (a.hit) arena?.hit(who, a.zone ?? "body", hitKind(a));
    if (r.hp[ours ? 1 : 0] <= 0) { arena?.fall(who); sound.play("bodyfall", 700); }
  };
  setTimeout(() => { strike(0); hpOnly(r, 1); }, 850);
  setTimeout(() => { strike(1); hpOnly(r, 0); sayTurn(r); }, 1450);
  setTimeout(() => {
    busy = false;
    if (duel.over) return finish();
    render(); startTimer();
  }, 2500);
}

/** The health numbers change when the hit lands, not when the turn is computed. */
function hpOnly(r: TurnReport, side: Side): void {
  const f = duel.match.fighters[side], id = side === 0 ? "Me" : "En";
  $("hp" + id).textContent = String(Math.max(0, r.hp[side]));
  $("bar" + id).style.width = Math.max(0, r.hp[side] / f.maxHp * 100) + "%";
}

function fire(el: HTMLElement): void { el.classList.remove("on"); void el.offsetWidth; el.classList.add("on"); }
function shake(): void { if (reduceMotion) return; stage.classList.remove("shake"); void stage.offsetWidth; stage.classList.add("shake"); }

function floatAt(parent: HTMLElement, x: number, y: number, cls: string, big: string, small?: string): void {
  const f = document.createElement("div"); f.className = "float " + cls;
  f.style.left = x + "px"; f.style.top = y + "px";
  f.innerHTML = big + (small ? "<small>" + small + "</small>" : "");
  parent.appendChild(f); setTimeout(() => f.remove(), 1700);
}
function rippleAt(parent: HTMLElement, x: number, y: number): void {
  const r = document.createElement("div"); r.className = "ripple"; r.style.left = x + "px"; r.style.top = y + "px";
  parent.appendChild(r); setTimeout(() => r.remove(), 900);
}
const label = (a: AttackReport) => a.weapon === "grenade" ? "граната" : a.crit ? "крит" : ZONE_NAMES[a.zone ?? "body"];
function showOnEnemy(a: AttackReport): void {
  const [x, y] = point(a.zone ?? "body");
  if (!a.hit) return floatAt(ringsEl, x + 30, y - 10, "miss", "Промах");
  if (a.blocked) { rippleAt(ringsEl, x, y); return floatAt(ringsEl, x + 36, y - 14, "blocked", "−" + a.damage, "Зонт"); }
  floatAt(ringsEl, x + 36, y - 14, "dmg", "−" + a.damage, label(a));
}
function showOnGuard(a: AttackReport): void {
  const body = $("guardBody"), y = { head: 13, body: 45, groin: 75, legs: 106 }[a.zone ?? "body"];
  if (!a.hit) return floatAt(body, 37, y, "miss", "мимо");
  if (a.blocked) { rippleAt(body, 37, y); return floatAt(body, 37, y - 4, "blocked", "−" + a.damage, "Зонт"); }
  floatAt(body, 37, y - 4, "dmg", "−" + a.damage, label(a));
}

// ---------- the log line ----------
function line(who: string, r: TurnReport, side: Side): string {
  const act = r.actions[side], a = r.attacks[side];
  if (act.type === "dash") return who + ": рывок вперёд";
  if (act.type === "run") return who + ": перебежка " + (act.move > 0 ? "вперёд" : "назад");
  if (act.type === "flee") return who + ": уходит из боя";
  if (act.type === "wait" || !a) return who + ": без выстрела";
  const how = a.weapon === "knife" ? "нож, " : "";
  if (!a.hit) return who + ": " + (a.weapon === "grenade" ? "граната, " : how) + "промах";
  return who + ": " + (a.weapon === "grenade" ? "граната" : how + ZONE_NAMES[a.zone ?? "body"]) + " −" + a.damage + (a.blocked ? " (Зонт)" : "") + (a.crit ? ", крит" : "");
}
let logTimer: ReturnType<typeof setTimeout> | undefined;
function say(html: string): void {
  const l = $("log");
  l.innerHTML = html;
  l.classList.add("on"); clearTimeout(logTimer);
  logTimer = setTimeout(() => l.classList.remove("on"), 5200);
}
function sayTurn(r: TurnReport): void {
  say(line("Ты", r, 0) + " <span>·</span> " + line(duel.fighters[1].name, r, 1));
}

// ---------- the end ----------
function finish(): void {
  stopTimer(); render();
  const res = duel.match.result!, foe = duel.fighters[1].name, turns = duel.match.history.length;
  const won = res.winner === duel.me, lost = res.winner === duel.bot;
  const v = $("endVeil");
  v.classList.toggle("lost", lost);
  $("endTitle").textContent = lost ? "Связь потеряна" : won ? "Цель подавлена" : "Ничья";
  $("endText").textContent =
    res.winner === null ? (res.reason === "double_ko" ? "Обе Персоны выведены из строя одновременно." : res.reason === "fled" ? "Оба ушли из боя." : "Время вышло, здоровья поровну.")
    : won ? (res.reason === "fled" ? `Персона «${foe}» ушла из боя.` : res.reason === "afk" ? "Соперник три хода не отвечал."
      : res.reason === "turn_limit" ? "Время вышло, у тебя больше здоровья." : `Персона «${foe}» выведена из строя за ${turns} ${plural(turns, "ход", "хода", "ходов")}. Добыча твоя.`)
    : res.reason === "fled" ? "Ты ушёл из боя и сохранил половину добычи." : res.reason === "afk" ? "Ты три хода не отдавал приказ."
      : res.reason === "turn_limit" ? "Время вышло, у соперника больше здоровья." : "Персона уничтожена. Ты очнёшься в капсуле на базе «Соль».";
  $("againBtn").textContent = lost ? "Очнуться в капсуле" : "Новый бой";
  v.hidden = false; $("againBtn").focus();
  sound.scene("menu");
}

function restart(): void {
  $("endVeil").hidden = true; $("log").classList.remove("on");
  duel = newDuel(); busy = false;
  arena?.reset(duel.match.distance);
  drawHeader(); render(); startTimer();
  sound.newFight();
  setSheet(false);
}

/** The menu sheet: opening it switches the music to the radio. */
function setSheet(open: boolean): void {
  $("infoVeil").hidden = !open;
  if (open) $("closeInfo").focus();
  sound.scene(open || duel.over ? "menu" : "fight");
}

function setFrame(f: Frame): void {
  frame = f;
  bg.src = f.src;
  arena?.setFrame(f);
  layout();
  renderFrames();
}

function renderFrames(): void {
  const seg = $("frameSeg"); seg.innerHTML = ""; seg.style.setProperty("--n", String(frames.length));
  $("frameLabel").hidden = frames.length < 2; seg.hidden = frames.length < 2;
  for (const f of frames) {
    const b = document.createElement("button"); b.type = "button"; b.textContent = f.name;
    b.setAttribute("aria-pressed", String(f === frame));
    b.addEventListener("click", () => { if (f !== frame) { setFrame(f); restart(); } });
    seg.appendChild(b);
  }
}

function renderSound(): void {
  const b = $("soundBtn");
  b.classList.toggle("off", !sound.on); b.setAttribute("aria-pressed", String(sound.on));
  b.querySelector(".wave")!.toggleAttribute("hidden", !sound.on); b.querySelector(".mute")!.toggleAttribute("hidden", sound.on);
  const list = $("trackList"); list.innerHTML = "";
  for (const t of STATIONS) {
    const btn = document.createElement("button"); btn.type = "button";
    btn.innerHTML = t.name + "<small>" + t.len + "</small>";
    btn.setAttribute("aria-pressed", String(t.id === sound.station));
    btn.addEventListener("click", () => { if (!sound.on) sound.toggle(); sound.unlock(); sound.tune(t.id); renderSound(); });
    list.appendChild(btn);
  }
}

// ---------- wiring ----------
$("goBtn").addEventListener("click", endTurn);
$("againBtn").addEventListener("click", restart);
$("restartBtn").addEventListener("click", restart);
$("moreBtn").addEventListener("click", () => setSheet(true));
$("closeInfo").addEventListener("click", () => setSheet(false));
$("infoVeil").addEventListener("click", (e) => { if (e.target === $("infoVeil")) setSheet(false); });
$("soundBtn").addEventListener("click", () => { sound.toggle(); renderSound(); });
document.addEventListener("keydown", (e) => { if (e.key === "Escape" && !$("infoVeil").hidden) setSheet(false); });
// a soft click on every button, except those with their own sound
document.addEventListener("click", (e) => {
  const b = (e.target as Element | null)?.closest?.("button");
  if (b && b.id !== "goBtn" && b.id !== "soundBtn") sound.play("ui-click");
});
document.addEventListener("pointerdown", () => sound.unlock(), { once: true });
document.addEventListener("keydown", () => sound.unlock(), { once: true });
window.addEventListener("resize", layout);

(function grain() {
  const el = $("grain"), urls: string[] = [], c = document.createElement("canvas"); c.width = c.height = 128;
  const ctx = c.getContext("2d")!;
  for (let i = 0; i < 4; i++) {
    const img = ctx.createImageData(128, 128);
    for (let k = 0; k < img.data.length; k += 4) { const v = Math.random() * 255; img.data[k] = img.data[k + 1] = img.data[k + 2] = v; img.data[k + 3] = 255; }
    ctx.putImageData(img, 0, 0); urls.push("url(" + c.toDataURL() + ")");
  }
  let n = 0;
  el.style.backgroundImage = urls[0]!;
  if (!reduceMotion) setInterval(() => { n = (n + 1) % urls.length; el.style.backgroundImage = urls[n]!; }, 90);
})();

try {
  arena = new Arena(canvas);
  arena.onFrame = placeRings;
} catch {
  $("status").textContent = "3D не запустилось на этом устройстве";
}
setFrame(frame);
drawHeader(); render(); renderSound(); startTimer();
arena?.load(MODEL, duel.match.distance)
  .then(() => { $("status").hidden = true; layout(); render(); })
  .catch(() => { $("status").textContent = "Модель не загрузилась. Запусти npm run assets и обнови страницу."; });
