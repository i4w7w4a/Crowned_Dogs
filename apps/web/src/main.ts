// The fight screen: a duel against the bot on the core rules, in the 3D arena.
// The panel reads the duel's order draft after every tap. A played turn runs on a
// short timeline (both walk, our attack lands, theirs lands, next turn) on the same
// beats as the audio package's sound script. The base screen (entry.ts) loads this module when the
// player goes to fight and hands over the kit from the fitting room (startBattle).

import { RADIO_PLAYLIST, TIMING, TRACKS } from "@crowned-dogs/audio";
import {
  VETERAN_KITS, WEAPONS, WEAPON_IDS, ZONES, canThrowGrenade, canUseKnife, draftChance, grenadeSpec,
  guardsNeeded, other, turnLimit, turnSeconds, validateLoadout, type AttackReport, type Loadout, type Side, type TurnReport, type WeaponId, type Zone,
} from "@crowned-dogs/core";
import { Arena, type HitKind } from "./arena/arena.ts";
import { CALIBRATION_FRAME, FRAMES, type Frame } from "./arena/frames.ts";
import { QUALITY, QUALITY_IDS, keepQuality, startQuality, type QualityId } from "./arena/quality.ts";
import { gameAudio, type VolumeKind } from "./audio/arena-audio.ts";
import { LocalDuel } from "./game/local-duel.ts";
import { FightLog, icon } from "./hud/log.ts";
import { liquidGlass } from "./hud/liquid.ts";
import { GUN_ACC, GUN_WITH, TIME_WORD, WEATHER_WORD, endWords, steps, turnsWord, zoneName } from "./hud/words.ts";
import { store } from "./store.ts";
import { activateAfterReady } from "./loading/battle-loading.ts";
import "./style.css";

/** The figure models: the .glb inside JSON as base64, since artifact pages do not serve .glb (see npm run assets).
 *  We play the male Persona; the enemy, Искра, is the female one (both from Art's Tripo parts). VITE_MODEL puts one model on both. */
const MY_MODEL = import.meta.env.VITE_MODEL ?? "models/persona-male.json";
const FOE_MODEL = import.meta.env.VITE_MODEL ?? "models/persona-female.json";
const PLAYER = { name: "Тень", tag: "K-417-093" };
const FOE = "Искра";

const $ = <T extends HTMLElement = HTMLElement>(id: string) => document.getElementById(id) as T;
const scene = $("scene"), stage = $("stage"), bg = $<HTMLImageElement>("bg"), ringsEl = $("rings"), canvas = $<HTMLCanvasElement>("gl");
const panel = document.querySelector<HTMLElement>(".panel")!, movesEl = $("moves"), infoVeil = $("infoVeil"), endVeil = $("endVeil");
const reduceMotion = window.matchMedia?.("(prefers-reduced-motion: reduce)").matches ?? false;

const frames: Frame[] = /[?&]cal\b/.test(location.search) ? [CALIBRATION_FRAME, ...FRAMES] : FRAMES;
let frame = frames[0]!;

/** What the player picked in the menu; kept between visits. */
const choice = {
  weapon: (WEAPON_IDS as readonly string[]).includes(store.get("cd-weapon") ?? "") ? store.get("cd-weapon") as WeaponId : "rifle",
  veteran: false, // the veteran kit is off (Art): the switch is gone from the menu
};

/** The kit from the fitting room; picking a gun or the veteran kit in the fight menu puts it aside. */
let fitted: Loadout | null = null;
let duel = newDuel();
/** True while the fight is on screen; the base screen has it otherwise. */
let active = false;
let exitBattle: (() => void) | null = null;
let busy = false;
/** Bumped by every new fight, so the timeline of an abandoned turn does nothing. */
let fightNo = 0;
let secondsLeft = 0;
let timerId: ReturnType<typeof setInterval> | null = null;
let geo = { x: 0, y: 0, w: 1, h: 1 };
/** Health changes when a hit lands, the turn number when the turn is over: not when the rules compute them. */
let shown = { hp: [0, 0], turn: 1 };

/** The graphics level; `byHand` once the player picked it in the menu (then the game never lowers it on its own). */
let quality: QualityId = "low", byHand = false;

const audio = gameAudio;
const log = new FightLog({ list: $("logList"), ticker: $("ticker"), button: $("logBtn"), sheet: $("logSheet") });
liquidGlass();
let arena: Arena | null = null;
let arenaLoading: Promise<void> | null = null;
let speedFrame = 0;
let speedDelay: ReturnType<typeof setTimeout> | null = null;

/** The bot takes a random gun, like in Telegram. */
function newDuel(): LocalDuel {
  const foeWeapon = WEAPON_IDS[Math.floor(Math.random() * WEAPON_IDS.length)]!;
  return new LocalDuel({
    mapId: frame.mapId, weather: frame.weather, time: frame.time,
    player: { ...PLAYER, weapon: choice.weapon, kit: fitted ?? (choice.veteran ? VETERAN_KITS[choice.weapon] : undefined) },
    bot: { name: FOE, weapon: foeWeapon }, // Art: the bot fights with the plain gun, no veteran kit
  });
}

const pct = (p: number | null) => (p === null ? "—" : Math.round(p * 100) + "%");
const cap = (s: string) => s.charAt(0).toUpperCase() + s.slice(1);
const clamp = (v: number, a: number, b: number) => Math.max(a, Math.min(b, v));

// ---------- attack reticles on the enemy ----------
const ringEls = {} as Record<Zone, HTMLButtonElement>;
for (const z of ZONES) {
  const b = document.createElement("button");
  b.type = "button"; b.className = "ring hint"; b.setAttribute("aria-label", "Атака: " + zoneName(z));
  b.innerHTML = '<svg viewBox="0 0 40 40" aria-hidden="true"><circle class="rc" cx="20" cy="20" r="7.2"/></svg>';
  b.addEventListener("click", () => tap(() => duel.press({ type: "zone", zone: z })));
  ringsEl.appendChild(b); ringEls[z] = b;
}
const tagEl = document.createElement("div");
tagEl.className = "tag"; tagEl.hidden = true;
ringsEl.appendChild(tagEl);

/** Where a zone of the enemy is on screen right now, in stage pixels. */
function point(z: Zone): [number, number] {
  const [u, v] = arena ? arena.zoneOnScreen(z) : [0.645, { head: 0.26, body: 0.34, groin: 0.42, legs: 0.49 }[z]];
  return [geo.x + u * geo.w, geo.y + v * geo.h + RING_NUDGE[z] * ringR];
}
/** Art: the rings a little further apart than the body points: the head a hair up, the chest up, the legs down (in ring radii). */
const RING_NUDGE: Record<Zone, number> = { head: -0.5, body: -1.4, groin: 0, legs: 1.4 };

/** Ring radius in px: the ring is about 4% of the picture's width across, as on Art's fight screen. */
let ringR = 7.2;
/** The line from the aimed ring to its label, px; the same as --lead in the CSS. */
const TAG_LEAD = 16;
/** Where the controls start, in scene px: a ring under them could not be tapped. */
let ctlTop = Infinity;
const ctlEl = panel.querySelector<HTMLElement>(".ctl")!;
function measureCtl(): void { ctlTop = panel.offsetTop + ctlEl.offsetTop - scene.offsetTop; }
new ResizeObserver(measureCtl).observe(ctlEl);
/** Where each ring is drawn this frame. */
const ringAt = {} as Record<Zone, [number, number]>;
function placeRings(): void {
  // On a short screen the enemy's legs (close up, the groin too) sink under the controls: those rings stop
  // just above them, and the rings above keep a finger's width apart.
  let limit = ctlTop - ringR - 6;
  for (const z of [...ZONES].reverse()) {
    const [x, y] = point(z), at = Math.min(y, limit);
    ringAt[z] = [x, at]; limit = at - Math.max(22, 2 * ringR + 8);
  }
  for (const z of ZONES) {
    // each ring's hit area ends halfway to its neighbour, so a tap never lands on the next zone
    const [x, y] = ringAt[z];
    let near = 40;
    for (const o of ZONES) if (o !== z) near = Math.min(near, Math.hypot(ringAt[o][0] - x, ringAt[o][1] - y));
    const el = ringEls[z], h = Math.max(2 * ringR + 4, near).toFixed(0) + "px";
    if (el.style.height !== h) { el.style.height = h; el.style.marginTop = `calc(${h} / -2)`; }
    el.style.transform = `translate(${x.toFixed(1)}px, ${y.toFixed(1)}px)`;
  }
  const zone = duel.draft.zone;
  if (!tagEl.hidden && zone) {
    // the label hangs off the ring to the right, or to the left when the screen edge or the distance column is in the way
    const [x, y] = ringAt[zone], w = tagEl.offsetWidth, gap = ringR + TAG_LEAD;
    const sc = scene.getBoundingClientRect(), col = movesEl.getBoundingClientRect();
    const besideCol = y + 14 > col.top - sc.top && y - 14 < col.bottom - sc.top;
    const right = x + gap + w <= (besideCol ? col.left - sc.left - 6 : scene.clientWidth - 8);
    tagEl.classList.toggle("flip", !right);
    tagEl.style.transform = `translate(${(right ? x + gap : x - gap - w).toFixed(1)}px, ${y.toFixed(1)}px) translateY(-50%)`;
  }
}

function layout(): void {
  if (!active || !scene.clientWidth) return; // hidden behind the base: nothing to measure
  const cw = scene.clientWidth, top = scene.clientHeight * frame.top, ch = scene.clientHeight - top;
  const s = Math.max(cw / frame.w, ch / frame.h);
  const w = frame.w * s, h = frame.h * s;
  geo = { x: (cw - w) * 0.5, y: top, w, h };
  for (const el of [bg, canvas]) Object.assign(el.style, { width: w + "px", height: h + "px", left: geo.x + "px", top: geo.y + "px" });
  // the part of the picture on screen, so the bodycam lens centres on what the player sees
  const vx = Math.max(0, -geo.x), vy = Math.max(0, -geo.y);
  arena?.resize(w, h, { x: vx / w, y: vy / h, w: (Math.min(w, cw - geo.x) - vx) / w, h: (Math.min(h, scene.clientHeight - geo.y) - vy) / h });
  ringR = Math.max(6, w * 0.02 - 0.75);
  for (const z of ZONES) ringEls[z].querySelector("circle")!.setAttribute("r", ringR.toFixed(2));
  measureCtl();
  placeRings();
}

// ---------- the order: defence, movement, attack ----------
/** One tap on the order. A refused tap says why in the log line. */
function tap(fn: () => string | null): void {
  if (!active || busy || duel.over) return;
  const err = fn();
  if (err) log.note(err);
  render();
}

const guards = [...document.querySelectorAll<HTMLButtonElement>(".dz")];
for (const b of guards) {
  const z = b.dataset.z as Zone;
  b.addEventListener("click", () => tap(() => duel.press({ type: "guard", zone: z })));
}

// one tap picks where to stand: forward 3 … back 2
const moves = [...document.querySelectorAll<HTMLButtonElement>(".mv")];
for (const b of moves) {
  const mv = Number(b.dataset.m);
  b.addEventListener("click", () => tap(() => {
    const err = duel.setMove(mv);
    // the knife cannot reach from there: put it away rather than refuse the step
    if (err && duel.draft.kind === "knife") { duel.press({ type: "knife" }); return duel.setMove(mv); }
    return err;
  }));
}
$("knifeBtn").addEventListener("click", () => tap(() => duel.press({ type: "knife" })));
$("nadeBtn").addEventListener("click", () => tap(() => duel.press({ type: "grenade" })));
$("swapBtn").addEventListener("click", () => tap(() => duel.press({ type: "swap" })));

// ---------- render ----------
function render(): void {
  const m = duel.match, d = duel.draft, me = m.fighters[duel.me];
  const lock = busy || duel.over;
  const run = d.kind === "run" || d.kind === "dash" || d.kind === "flee";
  const aimed = d.kind === "shoot" || d.kind === "knife";
  const step = Math.abs(d.move) <= 1;
  const knifeMove = canUseKnife(m, d.move) ? d.move : 1;
  const knifeOk = step && canUseKnife(m, knifeMove);
  const nadeOk = step && canThrowGrenade(m, duel.me);
  const planned = duel.plannedDistance;

  // a shot is a tap on the enemy: the rings stay live while a grenade, a swap or a run is picked, and tapping one turns the order back into a shot
  ringsEl.classList.toggle("off", busy);
  panel.classList.toggle("playing", busy); // the controls step aside while the turn plays out
  ringsEl.classList.toggle("idle", !aimed);
  ringsEl.classList.toggle("gone", duel.over || (arena !== null && !arena.ready)); // without WebGL the rings stay on the picture
  for (const z of ZONES) {
    ringEls[z].classList.toggle("sel", aimed && d.zone === z);
    ringEls[z].classList.toggle("hint", aimed && !d.zone && m.turn === 1);
  }
  const chance = duel.chance;
  tagEl.hidden = !aimed || !d.zone || chance === null || lock;
  if (!tagEl.hidden) tagEl.textContent = (d.kind === "knife" ? "Нож · " : "") + cap(zoneName(d.zone!)) + " · " + pct(chance);
  placeRings();

  // defence: a list of the four zones, a tick on each guarded one, and how many are picked
  for (const b of guards) {
    const on = d.defend.includes(b.dataset.z as Zone);
    b.classList.toggle("on", on); b.disabled = lock; b.setAttribute("aria-pressed", String(on));
  }
  const pips = $("pips"), need = guardsNeeded(d);
  pips.textContent = `${d.defend.length}/${need}`;
  pips.classList.toggle("done", d.defend.length >= need);
  pips.setAttribute("aria-label", `Выбрано ${d.defend.length} из ${need}`);

  // distance: the arrows, and in the middle the distance after the move, with a warning when the move costs the attack
  for (const b of moves) {
    const mv = Number(b.dataset.m), on = mv === d.move && d.kind !== "flee";
    b.classList.toggle("on", on); b.setAttribute("aria-pressed", String(on));
    b.disabled = lock || m.distance - mv < 1;
  }
  const dist = d.move !== 0 && !busy ? planned : m.distance, noShot = (run || d.kind === "swap") && !lock;
  $("distLabel").textContent = `${dist} ${steps(dist)}`;
  const note = $("distNote");
  note.hidden = !noShot; note.textContent = d.kind === "swap" ? "без выстрела" : "без атаки";

  // attack other than a shot: the knife, a grenade, or the other gun; tapping the picked one again goes back to the shot
  const kb = $<HTMLButtonElement>("knifeBtn"), nb = $<HTMLButtonElement>("nadeBtn"), wb = $<HTMLButtonElement>("swapBtn");
  kb.disabled = !knifeOk || lock; nb.disabled = !nadeOk || lock; wb.disabled = lock;
  const on: [HTMLButtonElement, boolean][] = [[kb, d.kind === "knife"], [nb, d.kind === "grenade"], [wb, d.kind === "swap"]];
  for (const [b, v] of on) { b.classList.toggle("on", v); b.setAttribute("aria-pressed", String(v)); }
  // a range is drawn with the steps icon: ≤2 👣
  const STEPS = icon("i-steps");
  $("knifeSub").innerHTML = knifeOk ? pct(draftChance(m, duel.me, { ...d, kind: "knife", move: knifeMove })) : "≤2" + STEPS;
  const g = grenadeSpec(m, duel.me);
  $("nadeSub").innerHTML = me.grenades ? (nadeOk ? "×" + me.grenades : `${g.minDistance}–${g.maxDistance}${STEPS}`) : "×0";
  const spare = duel.spare;
  wb.hidden = !spare;
  if (spare) { $("swapText").textContent = "Взять " + GUN_ACC[spare]; wb.setAttribute("aria-label", "Взять " + GUN_ACC[spare] + ", в этот ход без выстрела"); }

  const go = $<HTMLButtonElement>("goBtn");
  const msg = duel.over ? "Бой окончен" : busy ? "Ход идёт…" : duel.missing;
  go.disabled = !!msg;
  // a run or a dash is told by the distance column ("без атаки"), so the button keeps its one wording
  $("goText").textContent = msg ?? (d.kind === "flee" ? "Уйти из боя" : "Подтвердить ход");

  drawHP(0); drawHP(1);
  $("turn").textContent = `Ход ${shown.turn} / ${turnLimit(m)}`;
  drawTimer();
}

function drawHP(side: Side): void {
  const id = side === 0 ? "Me" : "En", hp = shown.hp[side]!, max = duel.match.fighters[side].maxHp;
  const w = clamp(hp / max * 100, 0, 100) + "%";
  $("hp" + id).textContent = String(Math.max(0, Math.round(hp)));
  $("bar" + id).style.width = w; $("ghost" + id).style.width = w;
  $("side" + id).classList.toggle("low", hp > 0 && hp <= max * 0.25);
}

function drawHeader(): void {
  const c = duel.match.config;
  $("condText").textContent = [TIME_WORD[c.time], WEATHER_WORD[c.weather]].join(" · ");
  $("mapTitle").textContent = frame.name;
  $("nameMe").textContent = duel.fighters[0].name; $("nameEn").textContent = duel.fighters[1].name;
  $("recTag").textContent = duel.fighters[0].tag ?? "";
}

// ---------- timer ----------
/** The turn clock at the top: mm:ss, red for the last ten seconds, dimmed while the turn plays out. */
function drawTimer(): void {
  const t = Math.max(0, secondsLeft), clock = $("timer"), two = (n: number) => String(n).padStart(2, "0");
  clock.textContent = `${two(Math.floor(t / 60))}:${two(t % 60)}`;
  clock.classList.toggle("low", t <= 10 && !busy && !duel.over);
  clock.classList.toggle("idle", busy || duel.over);
}
function startTimer(): void {
  stopTimer();
  if (!active) return;
  secondsLeft = turnSeconds(duel.match);
  drawTimer();
  timerId = setInterval(() => {
    if (busy || duel.over || !infoVeil.hidden) return;
    secondsLeft -= 1; drawTimer();
    if (secondsLeft > 0 && secondsLeft <= 5) audio.play(secondsLeft <= 3 ? "ui.tick.urgent" : "ui.tick");
    // whatever is ready goes; without an attack zone the Persona stands and keeps last turn's shield
    if (secondsLeft <= 0) playTurn(duel.timeout());
  }, 1000);
}
function stopTimer(): void {
  if (timerId) clearInterval(timerId);
  timerId = null;
}

// ---------- the turn ----------
function endTurn(): void {
  if (!active || busy || duel.over) return;
  const r = duel.submit();
  if (typeof r === "string") { log.note(r); return; }
  playTurn(r);
}

const hitKind = (a: AttackReport): HitKind => (a.weapon === "grenade" ? "grenade" : a.blocked ? "blocked" : "hit");

function playTurn(r: TurnReport): void {
  stopTimer(); busy = true;
  const no = fightNo, later = (s: number, fn: () => void) => setTimeout(() => { if (no === fightNo) fn(); }, s * 1000);
  audio.play("ui.confirm");
  audio.turn(r, duel.match);
  armBoth(); // a swap changes the gun before anyone moves
  arena?.move(r.distanceAfter, r.moves[0], r.moves[1]); // both walk first, then the shots
  // each Persona starts its attack early enough that the shot (the stab, the throw) meets its sound
  for (const s of [0, 1] as Side[]) {
    const a = r.attacks[s];
    if (!a) continue;
    const t = s === duel.me ? TIMING.ourAttack : TIMING.enemyAttack;
    arena?.attack(s === duel.me ? "me" : "enemy", a.weapon, a.weapon === "knife" ? t + TIMING.knifeLands : t);
  }
  render();
  later(TIMING.ourAttack, () => strike(r, 0));
  later(TIMING.enemyAttack, () => {
    strike(r, 1);
    const [a, b] = duel.match.fighters;
    log.add(r, [a.name, b.name], [a.weapon, b.weapon]);
  });
  later(2.5, () => {
    busy = false;
    shown.turn = duel.match.turn;
    if (duel.over) return finish();
    audio.play("ui.turn");
    render(); startTimer();
  });
}

/** Both Personas take up the guns they carry now. */
function armBoth(): void {
  for (const s of [0, 1] as Side[]) {
    arena?.arm(s === duel.me ? "me" : "enemy", duel.match.fighters[s].weapon, duel.fighters[s].kit?.weapon1.mods);
  }
}

/** One side's attack lands: the flash, the number, the flinch, and the fall if it was the last. */
function strike(r: TurnReport, side: Side): void {
  const a = r.attacks[side], ours = side === duel.me, target = other(side);
  shown.hp[target] = r.hp[target];
  if (a) {
    if (ours) { fire($("flashOurs")); shake(); showOnEnemy(a); }
    else { if (a.hit && !a.blocked) { fire($("flashHurt")); if (cameraView === "fp") shake(); } showOnMe(a); }
    if (a.hit) arena?.hit(ours ? "enemy" : "me", a.zone ?? "body", hitKind(a));
  }
  if (r.hp[target] <= 0) {
    arena?.fall(ours ? "enemy" : "me");
    // first person, we go down: the bodycam falls with us, so the whole picture tips over (style.css .stage.down)
    if (!ours && cameraView === "fp" && !reduceMotion) stage.classList.add("down");
  }
  render();
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
const label = (a: AttackReport) => (a.weapon === "grenade" ? "граната" : a.crit ? "крит" : zoneName(a.zone ?? "body"));
function showOnEnemy(a: AttackReport): void {
  const [x, y] = point(a.zone ?? "body");
  if (!a.hit) return floatAt(ringsEl, x + 30, y - 10, "miss", "Промах");
  if (a.blocked) { rippleAt(ringsEl, x, y); return floatAt(ringsEl, x + 36, y - 14, "blocked", "−" + a.damage, "Зонт"); }
  floatAt(ringsEl, x + 36, y - 14, "dmg", "−" + a.damage, label(a));
}
/** The enemy's attack on us: the controls are away while the turn plays, so the number floats over our fighter's shoulder. */
function showOnMe(a: AttackReport): void {
  const x = stage.clientWidth * 0.3, y = stage.clientHeight * 0.6;
  if (!a.hit) return floatAt(ringsEl, x, y, "miss", "мимо");
  if (a.blocked) return floatAt(ringsEl, x, y, "blocked", "−" + a.damage, "Зонт");
  floatAt(ringsEl, x, y, "dmg", "−" + a.damage, label(a));
}

// ---------- the end ----------
function finish(): void {
  stopTimer(); render();
  const m = duel.match, res = m.result!, turns = m.history.length;
  const w = endWords(res, duel.me, duel.fighters[1].name, turns);
  let dealt = 0, taken = 0;
  for (const t of m.history) { dealt += t.attacks[0]?.damage ?? 0; taken += t.attacks[1]?.damage ?? 0; }
  endVeil.classList.toggle("lost", w.lost);
  $("endTitle").textContent = w.title;
  $("endText").textContent = w.text;
  $("againBtn").textContent = w.again;
  $("endStats").innerHTML = `<div><b>${dealt}</b><span>Нанесено</span></div><div><b>${taken}</b><span>Получено</span></div><div><b>${turns}</b><span>${turnsWord(turns)}</span></div>`;
  $("endBadge").innerHTML = icon(w.lost ? "i-signal" : "i-shot");
  // Art: a dead operator wakes up in the capsule, that is back at the base; a winner picks a new fight or the base
  $("baseEndBtn").hidden = w.lost; againGoesHome = w.lost;
  endVeil.hidden = false; $("againBtn").focus();
  audio.finish(res.winner === null ? null : res.winner === duel.me);
}

/** A new fight. From the menu the sheet stays open, so weapon and kit can be changed in one go. */
function restart(keepSheet = false): void {
  fightNo += 1;
  endVeil.hidden = true; stage.classList.remove("down");
  duel = newDuel(); busy = false;
  shown = { hp: duel.match.fighters.map((f) => f.hp), turn: duel.match.turn };
  armBoth(); arena?.reset(duel.match.distance);
  const d = duel.match.distance;
  log.reset(`Запись идёт · ${FOE} ${GUN_WITH[duel.fighters[1].weapon]}, ${d} ${steps(d)}`);
  drawHeader(); render(); startTimer();
  audio.newFight(duel.match);
  setSheet(keepSheet);
}

/** The menu sheet: it pauses the fight and its music, and the radio comes on. */
function setSheet(open: boolean): void {
  infoVeil.hidden = !open;
  if (open) { renderMenu(); $("closeInfo").focus({ preventScroll: true }); }
  audio.sheet(open);
}

function renderMenu(): void {
  const seg = (id: string, n: number) => { const el = $(id); el.innerHTML = ""; el.style.setProperty("--n", String(n)); return el; };
  const option = (parent: HTMLElement, html: string, pressed: boolean, onPick: () => void) => {
    const b = document.createElement("button"); b.type = "button"; b.className = "glass"; b.innerHTML = html;
    b.setAttribute("aria-pressed", String(pressed));
    b.addEventListener("click", onPick);
    parent.appendChild(b);
  };

  // the gun in hand at the start, which is the fitting room's when its kit is on
  const held = duel.fighters[duel.me].weapon;
  const weapons = seg("weaponSeg", WEAPON_IDS.length);
  for (const w of WEAPON_IDS) {
    option(weapons, WEAPONS[w].name, w === held, () => {
      if (w === held) return;
      choice.weapon = w; store.set("cd-weapon", w); fitted = null;
      restart(true);
    });
  }
  const vet = $("vetBtn"), vetOn = choice.veteran && !fitted;
  vet.classList.toggle("off", !vetOn); vet.setAttribute("aria-pressed", String(vetOn));
  $("vetState").textContent = vetOn ? "Вкл" : "Выкл";

  const sb = $("soundBtn");
  sb.classList.toggle("off", !audio.on); sb.setAttribute("aria-pressed", String(audio.on));
  $("soundState").textContent = audio.on ? "Вкл" : "Выкл";
  for (const k of ["fx", "music", "fight"] as const) drawVolume(k);

  const tracks = $("trackList"); tracks.innerHTML = "";
  for (const s of RADIO_PLAYLIST) {
    const now = s === audio.station;
    option(tracks, TRACKS[s].title + `<small>${now ? "в эфире" : "&nbsp;"}</small>`, now, () => { audio.tune(s); renderMenu(); });
  }

  const cs = seg("camSeg", CAMERAS.length);
  for (const [v, name] of CAMERAS) option(cs, name, v === cameraView, () => { if (v !== cameraView) { setCamera(v); renderMenu(); } });

  const gs = seg("gfxSeg", QUALITY_IDS.length);
  for (const id of QUALITY_IDS) option(gs, QUALITY[id].name, id === quality, () => { if (id !== quality) { setQuality(id, true); renderMenu(); } });
  $("gfxNote").textContent = GFX_NOTE[quality] + (byHand ? "" : " Подобрано под это устройство.");

  const fs = seg("frameSeg", frames.length);
  $("frameLabel").hidden = true; fs.hidden = true; // the backdrop is not picked in the menu any more (Art)
  for (const f of frames) option(fs, f.name, f === frame, () => { if (f !== frame) { setFrame(f); restart(); } });
  liquidGlass(infoVeil);
}

// volume sliders: fight sounds and music, 0..100 on screen, 0..1 in the audio
const VOL_ID = { fx: "volFx", music: "volMusic", fight: "volFight" } as const;
function drawVolume(k: VolumeKind): void {
  const v = Math.round(audio.volume[k] * 100), input = $<HTMLInputElement>(VOL_ID[k]);
  input.value = String(v); input.style.setProperty("--v", v + "%");
  $(VOL_ID[k] + "Out").textContent = String(v);
}
for (const k of ["fx", "music", "fight"] as const) {
  const input = $<HTMLInputElement>(VOL_ID[k]);
  input.addEventListener("input", () => { audio.setVolume(k, Number(input.value) / 100); drawVolume(k); });
  // letting go of the fight-sounds slider plays a click at the new level
  if (k === "fx") input.addEventListener("change", () => audio.play("ui.tap"));
}

const GFX_NOTE: Record<QualityId, string> = {
  low: "Для слабых телефонов: без теней, дымки и пыли, картинка попроще.",
  medium: "Для телефона: мягкие тени, лучи света из окон, пыль в воздухе.",
  high: "Для компьютера: дымка с тенями бойцов, текстуры 2К, чёткий фон.",
  ultra: "Для мощного ПК: всё на максимум, текстуры 4К, кадр рисуется крупнее экрана и сглаживается.",
};

function setQuality(id: QualityId, picked: boolean): void {
  quality = id;
  if (picked) { byHand = true; keepQuality(id); }
  arena?.setQuality(QUALITY[id]);
  layout();
}

/** A level the device was guessed to carry, which then runs slower than 24 frames a second, steps down by itself. */
function watchSpeed(): void {
  let frames = 0, t0 = 0;
  const step = (t: number) => {
    if (byHand || !active) return;
    if (document.hidden) { t0 = 0; frames = 0; speedFrame = requestAnimationFrame(step); return; }
    if (!t0) t0 = t;
    frames += 1;
    if (t - t0 < 4000) { speedFrame = requestAnimationFrame(step); return; }
    const i = QUALITY_IDS.indexOf(quality);
    if (frames * 1000 / (t - t0) < 24 && i > 0) { setQuality(QUALITY_IDS[i - 1]!, false); speedDelay = setTimeout(watchSpeed, 1500); }
  };
  cancelAnimationFrame(speedFrame);
  speedFrame = requestAnimationFrame(step);
}

function setFrame(f: Frame): void {
  frame = f;
  bg.src = f.src;
  arena?.setFrame(f);
  layout();
}

// ---------- wiring ----------
$("goBtn").addEventListener("click", endTurn);
let againGoesHome = false;
$("againBtn").addEventListener("click", () => { if (againGoesHome) leaveBattle(); else restart(); });
$("restartBtn").addEventListener("click", () => restart());
$("moreBtn").addEventListener("click", () => setSheet(true));
$("closeInfo").addEventListener("click", () => setSheet(false));
$("closeX").addEventListener("click", () => setSheet(false));
infoVeil.addEventListener("click", (e) => { if (e.target === infoVeil) setSheet(false); });
$("soundBtn").addEventListener("click", () => { audio.toggle(); renderMenu(); });
$("vetBtn").addEventListener("click", () => {
  // with the fitting room's kit on, the switch reads "off": a tap puts the veteran kit on instead
  choice.veteran = fitted ? true : !choice.veteran; store.set("cd-veteran", choice.veteran ? "on" : "off"); fitted = null;
  restart(true);
});
$("logBtn").addEventListener("click", () => log.setOpen(!log.open));
$("logClose").addEventListener("click", () => log.setOpen(false));
document.addEventListener("keydown", (e) => {
  if (e.key !== "Escape") return;
  if (!infoVeil.hidden) setSheet(false); else if (log.open) log.setOpen(false);
});
$("baseInfoBtn").addEventListener("click", leaveBattle);
$("baseEndBtn").addEventListener("click", leaveBattle);
// a soft sound on every fight button, except those with their own
document.addEventListener("click", (e) => {
  const b = (e.target as Element | null)?.closest?.("button");
  if (!active || !b || !$("app").contains(b) || b.id === "goBtn" || b.id === "soundBtn") return;
  audio.play(b.classList.contains("dz") ? "ui.guard" : b.classList.contains("mv") ? "ui.step" : "ui.tap");
});
document.addEventListener("pointerdown", () => audio.unlock(), { once: true });
document.addEventListener("keydown", () => audio.unlock(), { once: true });
window.addEventListener("resize", layout);
// The camera, picked in the menu and remembered: over our shoulder, or first person (our eyes, as a
// bodycam). An old link with #fp at the end still opens first person.
type CameraView = "back" | "fp";
const CAMERAS: [CameraView, string][] = [["back", "Из-за плеча"], ["fp", "От первого лица"]];
// first person unless "from behind" was picked (Art, 2026-10-28)
let cameraView: CameraView = store.get("cd-camera") === "back" && location.hash !== "#fp" ? "back" : "fp";
function setCamera(v: CameraView): void {
  cameraView = v; store.set("cd-camera", v);
  arena?.setFirstPerson(v === "fp");
}
window.addEventListener("hashchange", () => { if (location.hash === "#fp") setCamera("fp"); });

// ---------- go ----------
try {
  arena = new Arena(canvas);
  arena.onFrame = placeRings;
  arena.setFirstPerson(cameraView === "fp");
  const start = startQuality(arena.gl);
  quality = start.id; byHand = start.picked;
  arena.setQuality(QUALITY[quality]);
} catch {
  $("status").textContent = "3D не запустилось на этом устройстве";
}
setFrame(frame);
function prepareArena(): Promise<void> {
  if (!arena) return Promise.reject(new Error("3D не запустилось на этом устройстве"));
  if (arena.ready) return Promise.resolve();
  if (!arenaLoading) {
    arenaLoading = arena.load(MY_MODEL, FOE_MODEL, duel.match.distance).then(() => {
      if (!arena?.ready) throw new Error("Арена ещё не готова");
      armBoth(); arena.reset(duel.match.distance);
      // the loader stays until the textures and the painted hall are in too (at most 25 s)
      const all = Promise.all([arena.texturesDone, bg.decode().catch(() => {})]);
      return Promise.race([all, new Promise((r) => setTimeout(r, 25000))]);
    }).then(() => {
      $("status").hidden = true;
      layout(); render();
    }).catch((error: unknown) => {
      arenaLoading = null;
      throw error;
    });
  }
  return arenaLoading;
}

/** Called by the base screen: the fight takes the stage with the fitting room's kit. */
export async function startBattle(
  kit: Loadout, onExit: () => void, beforeEnter: () => Promise<void> = async () => {}, signal?: AbortSignal,
): Promise<void> {
  const selected = structuredClone(kit);
  // the base's settings may have changed the camera or the graphics since the last fight
  setCamera(store.get("cd-camera") === "back" ? "back" : "fp");
  const q = store.get("cd-quality") as QualityId | null;
  if (arena && q && q in QUALITY && q !== quality) { quality = q; byHand = true; arena.setQuality(QUALITY[q]); }
  if (validateLoadout(selected)) throw new Error("Некорректный комплект");
  await activateAfterReady(prepareArena, beforeEnter, () => {
    fitted = selected;
    exitBattle = onExit;
    active = true;
    $("app").hidden = false;
    restart();
    layout();
    arena?.resume();
    audio.unlock();
    speedDelay = setTimeout(watchSpeed, 1500);
  }, signal);
}

/** Back to the base: the fight stops where it is (it is not saved) and the radio plays on. */
function leaveBattle(): void {
  active = false;
  fightNo += 1; stopTimer(); busy = false;
  cancelAnimationFrame(speedFrame);
  if (speedDelay) clearTimeout(speedDelay);
  speedDelay = null;
  arena?.pause();
  infoVeil.hidden = true; endVeil.hidden = true; log.setOpen(false);
  audio.finish(null);
  $("app").hidden = true;
  exitBattle?.();
}

/** For screenshots and checks from the browser console. */
Object.assign(window, { __cd: { get duel() { return duel; }, audio, arena } });
