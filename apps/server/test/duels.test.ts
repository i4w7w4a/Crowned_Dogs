import assert from "node:assert/strict";
import { test } from "node:test";
import { createMatch } from "@crowned-dogs/core";
import { applyCommand, decodeCommand, encodeCommand, freshDraft, toOrder, type Command, type Draft } from "../src/draft.ts";
import { DuelError, DuelService, type Clock, type DuelEvent } from "../src/duels.ts";
import { turnReport, turnScreen, weaponPicker, finalScreen } from "../src/view.ts";

/** A clock the test moves by hand. */
function fakeClock() {
  let now = 1_000_000;
  let timers: { at: number; fn: () => void; id: number }[] = [];
  let next = 1;
  const clock: Clock = {
    now: () => now,
    setTimeout: (fn, ms) => { const id = next++; timers.push({ at: now + ms, fn, id }); return id; },
    clearTimeout: (h) => { timers = timers.filter((t) => t.id !== h); },
  };
  const advance = (ms: number) => {
    now += ms;
    const due = timers.filter((t) => t.at <= now);
    timers = timers.filter((t) => t.at > now);
    for (const t of due) t.fn();
  };
  return { clock, advance, pending: () => timers.length };
}

function service() {
  const events: DuelEvent[] = [];
  const c = fakeClock();
  let n = 0;
  const duels = new DuelService({
    onEvent: (e) => { events.push(e); },
    clock: c.clock,
    seed: () => `seed-${n++}`,
    setting: { mapId: "saltHall", weather: "clear", time: "morning" },
  });
  return { duels, events, ...c };
}

const ann = { id: "100", name: "Аня", weapon: "rifle" as const };
const bob = { id: "200", name: "Боб", weapon: "shotgun" as const };

function press(d: DuelService, who: string, duelId: string, cmds: Command[]) {
  const turn = d.get(duelId)!.match.turn;
  for (const c of cmds) d.press(who, duelId, turn, c);
  return d.submit(who, duelId, turn);
}

const plainShot: Command[] = [{ type: "zone", zone: "body" }, { type: "guard", zone: "head" }, { type: "guard", zone: "body" }];

// ---------- draft ----------

test("buttons build an order step by step", () => {
  const m = createMatch({ seed: "d", mapId: "saltHall", weather: "clear", time: "morning",
    fighters: [{ id: "a", name: "a", weapon: "rifle" }, { id: "b", name: "b", weapon: "rifle" }] });
  let d: Draft = freshDraft(m, 0);
  assert.equal(toOrder(m, 0, d), "Выбери зону атаки");
  const apply = (c: Command) => { const r = applyCommand(m, 0, d, c); assert.equal(typeof r, "object", String(r)); d = r as Draft; };
  apply({ type: "zone", zone: "head" });
  assert.equal(toOrder(m, 0, d), "Выбери 2 зоны защиты");
  apply({ type: "guard", zone: "body" });
  apply({ type: "guard", zone: "legs" });
  apply({ type: "guard", zone: "head" });
  assert.deepEqual(d.defend, ["legs", "head"], "a third guard pushes out the oldest");
  apply({ type: "step", move: 1 });
  assert.deepEqual(toOrder(m, 0, d), { action: { type: "shoot", move: 1, zone: "head" }, defend: ["legs", "head"] });

  apply({ type: "dash" });
  assert.deepEqual(d.defend, ["head"], "a dash leaves one guarded zone");
  assert.deepEqual(toOrder(m, 0, d), { action: { type: "dash" }, defend: ["head"] });
  apply({ type: "zone", zone: "groin" });
  assert.equal(d.kind, "shoot", "aiming again cancels the dash");
  assert.equal(d.move, 0);

  assert.equal(applyCommand(m, 0, d, { type: "knife" }), "Нож бьёт только с 2 шагов");
  assert.equal(applyCommand(m, 0, d, { type: "flee" }), "Уйти можно только с края карты");
});

test("the knife steps in for you when one step is enough", () => {
  const m = createMatch({ seed: "k", mapId: "saltHall", weather: "clear", time: "morning", startDistance: 3,
    fighters: [{ id: "a", name: "a", weapon: "rifle" }, { id: "b", name: "b", weapon: "rifle" }] });
  const d = applyCommand(m, 0, freshDraft(m, 0), { type: "knife" }) as Draft;
  assert.equal(d.kind, "knife");
  assert.equal(d.move, 1);
});

test("button codes survive the round trip and stay short", () => {
  const all: Command[] = [
    { type: "zone", zone: "legs" }, { type: "guard", zone: "groin" }, { type: "step", move: -1 },
    { type: "run", move: 2 }, { type: "dash" }, { type: "knife" }, { type: "grenade" }, { type: "flee" },
  ];
  for (const c of all) assert.deepEqual(decodeCommand(encodeCommand(c)), c);
  assert.equal(decodeCommand("zx"), null);
  assert.equal(decodeCommand("s5"), null);
});

// ---------- duels ----------

test("a fight against the bot plays a turn as soon as the player is ready", () => {
  const { duels, events } = service();
  const duel = duels.startVsBot(ann);
  assert.equal(events[0]?.type, "start");
  assert.equal(duels.duelOf(ann.id)?.id, duel.id);
  press(duels, ann.id, duel.id, plainShot);
  assert.equal(events[1]?.type, "turn");
  assert.equal(duels.get(duel.id)?.match.turn, 2);
});

test("a duel with a friend waits for both secret orders", () => {
  const { duels, events } = service();
  const inv = duels.createInvite(ann);
  assert.equal(inv.code.length, 6);
  assert.throws(() => duels.acceptInvite(inv.code, ann), DuelError, "you cannot accept your own invite");
  const duel = duels.acceptInvite(inv.code, bob);
  assert.equal(duels.inviteInfo(inv.code), undefined, "an invite works once");
  press(duels, ann.id, duel.id, plainShot);
  assert.equal(events.filter((e) => e.type === "turn").length, 0, "still waiting for Bob");
  assert.throws(() => duels.press(ann.id, duel.id, 1, { type: "zone", zone: "head" }), /Приказ уже отдан/);
  press(duels, bob.id, duel.id, plainShot);
  assert.equal(events.filter((e) => e.type === "turn").length, 1);
});

test("old buttons and strangers are refused", () => {
  const { duels } = service();
  const duel = duels.startVsBot(ann);
  press(duels, ann.id, duel.id, plainShot);
  assert.throws(() => duels.press(ann.id, duel.id, 1, { type: "zone", zone: "head" }), /уже прошёл/);
  assert.throws(() => duels.press(bob.id, duel.id, 2, { type: "zone", zone: "head" }), /не твой бой/);
  assert.throws(() => duels.startVsBot(ann), /уже в бою/);
});

test("the timer plays the turn for players who did not answer, and three misses lose", () => {
  const { duels, events, advance } = service();
  const duel = duels.acceptInvite(duels.createInvite(ann).code, bob);
  // Bob keeps running back and forth (no shots), Ann never answers.
  for (let i = 0; i < 3; i++) {
    const turn = duel.match.turn;
    if (turn === 1) for (const z of ["head", "body"] as const) duels.press(bob.id, duel.id, turn, { type: "guard", zone: z });
    duels.press(bob.id, duel.id, turn, { type: "run", move: i % 2 ? 2 : -2 });
    duels.submit(bob.id, duel.id, turn);
    advance(60_000);
  }
  const last = events.at(-1);
  assert.equal(last?.type, "turn");
  assert.deepEqual(last.duel.match.result, { winner: 1, reason: "afk" });
  assert.equal(duels.duelOf(ann.id), undefined, "both players are free again");
  assert.equal(duels.duelOf(bob.id), undefined);
  assert.equal(duels.get(duel.id), undefined);
});

test("invites expire after 30 minutes", () => {
  const { duels, advance } = service();
  const inv = duels.createInvite(ann);
  advance(31 * 60_000);
  assert.throws(() => duels.acceptInvite(inv.code, bob), /не найдено или устарело/);
});

test("a whole fight against the bot ends and frees the player", () => {
  const { duels, events } = service();
  const duel = duels.startVsBot(ann);
  for (let i = 0; i < 20 && duels.get(duel.id); i++) {
    // From turn 2 the shield keeps last turn's zones, so only the target is chosen again.
    press(duels, ann.id, duel.id, i === 0 ? plainShot : [{ type: "zone", zone: "head" }]);
  }
  const last = events.at(-1)!;
  assert.ok(last.type === "turn" && last.duel.match.result);
  assert.equal(duels.duelOf(ann.id), undefined);
});

// ---------- view ----------

test("screens fit Telegram limits and speak plainly", () => {
  const { duels, events } = service();
  const duel = duels.startVsBot(ann);
  const s = turnScreen(duel, 0);
  assert.match(s.text, /Ход 1 из 15/);
  assert.match(s.text, /7 шагов до врага/);
  for (const row of s.buttons) {
    assert.ok(row.length <= 5);
    for (const b of row) assert.ok(Buffer.byteLength(b.data) <= 64, b.data);
  }
  assert.equal(s.buttons.at(-1)?.[0]?.text, "Выбери зону атаки");
  press(duels, ann.id, duel.id, plainShot);
  const turn = events[1]!;
  assert.equal(turn.type, "turn");
  if (turn.type === "turn") assert.match(turnReport(turn.duel, 0, turn.report), /^<b>Ход 1<\/b>/);
  for (const row of weaponPicker("join:ABC234")) for (const b of row) assert.ok(Buffer.byteLength(b.data) <= 64);
});

test("the end screen names the winner from each side", () => {
  const { duels, events, advance } = service();
  const duel = duels.acceptInvite(duels.createInvite(ann).code, bob);
  for (let i = 0; i < 3; i++) advance(60_000);
  const last = events.at(-1)!;
  assert.equal(last.duel.id, duel.id);
  assert.equal(last.duel.match.result?.winner, null, "both missed three turns: a draw");
  assert.match(finalScreen(last.duel, 0).text, /Ничья/);
});

test("the end screen speaks from each side", () => {
  const { duels, events, advance } = service();
  const duel = duels.acceptInvite(duels.createInvite(ann).code, bob);
  for (let i = 0; i < 3; i++) {
    const turn = duel.match.turn;
    if (turn === 1) for (const z of ["head", "body"] as const) duels.press(bob.id, duel.id, turn, { type: "guard", zone: z });
    duels.press(bob.id, duel.id, turn, { type: "run", move: i % 2 ? 2 : -2 });
    duels.submit(bob.id, duel.id, turn);
    advance(60_000);
  }
  const last = events.at(-1)!;
  assert.match(finalScreen(last.duel, 0).text, /Связь потеряна/);
  assert.match(finalScreen(last.duel, 1).text, /Цель подавлена/);
});
