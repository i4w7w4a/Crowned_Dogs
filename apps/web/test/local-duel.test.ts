import assert from "node:assert/strict";
import { test } from "node:test";
import { VETERAN_KITS } from "@crowned-dogs/core";
import { LocalDuel, moveCommand } from "../src/game/local-duel.ts";

const duel = (seed = "web-1") => new LocalDuel({
  seed, weather: "dust", time: "evening",
  player: { name: "Тень", weapon: "rifle", tag: "K-417-093" },
  bot: { name: "Искра", weapon: "rifle" },
});

test("the move control maps onto steps, runs and the dash", () => {
  assert.deepEqual(moveCommand(0), { type: "step", move: 0 });
  assert.deepEqual(moveCommand(-1), { type: "step", move: -1 });
  assert.deepEqual(moveCommand(2), { type: "run", move: 2 });
  assert.deepEqual(moveCommand(-2), { type: "run", move: -2 });
  assert.deepEqual(moveCommand(3), { type: "dash" });
});

test("an order is ready once the zone and both shields are chosen", () => {
  const d = duel();
  assert.equal(d.missing, "Выбери зону атаки");
  assert.equal(d.submit(), "Выбери зону атаки");
  d.press({ type: "zone", zone: "head" });
  assert.equal(d.missing, "Выбери 2 зоны защиты");
  assert.ok(d.chance! > 0 && d.chance! < 1, "the planned shot has a chance");
  d.press({ type: "guard", zone: "head" });
  d.press({ type: "guard", zone: "body" });
  assert.equal(d.missing, null);
  const report = d.submit();
  assert.equal(typeof report, "object");
  assert.equal(d.match.turn, 2);
  assert.deepEqual(d.draft.defend, ["head", "body"], "the next turn starts with last turn's shield");
});

test("the move control switches between step, run and dash without getting stuck", () => {
  const d = duel();
  d.press({ type: "zone", zone: "body" });
  for (const [move, kind] of [[1, "shoot"], [2, "run"], [3, "dash"], [2, "run"], [-2, "run"], [0, "shoot"], [-1, "shoot"]] as const) {
    assert.equal(d.setMove(move), null, `move ${move}`);
    assert.equal(d.draft.move, move);
    assert.equal(d.draft.kind, kind);
  }
  assert.equal(d.plannedDistance, 8);
  d.setMove(3);
  assert.equal(d.missing, "Выбери зону защиты", "a dash keeps one shield");
});

test("the same seed plays the same fight, and the timer still plays a turn", () => {
  const a = duel("same"), b = duel("same");
  for (const d of [a, b]) {
    d.press({ type: "zone", zone: "legs" });
    d.press({ type: "guard", zone: "head" });
    d.press({ type: "guard", zone: "legs" });
  }
  assert.deepEqual(a.submit(), b.submit());
  const late = a.timeout();
  assert.equal(late.turn, 2, "a turn is played even with no order");
});

test("a whole fight against the bot ends", () => {
  const d = duel("long");
  for (let i = 0; i < 40 && !d.over; i++) {
    d.press({ type: "zone", zone: "body" });
    if (d.draft.defend.length < 2) { d.press({ type: "guard", zone: "head" }); d.press({ type: "guard", zone: "body" }); }
    const r = d.submit();
    assert.equal(typeof r, "object", String(r));
  }
  assert.ok(d.over, "the fight has a result");
  assert.equal(d.submit(), "Бой окончен");
});

test("a veteran sniper has a pistol to swap to, and the swap plays as a turn without a shot", () => {
  const d = new LocalDuel({
    seed: "vet", weather: "clear", time: "day",
    player: { name: "Тень", weapon: "sniper", kit: VETERAN_KITS.sniper },
    bot: { name: "Искра", weapon: "shotgun", kit: VETERAN_KITS.shotgun },
  });
  assert.equal(d.spare, "pistol");
  assert.ok(d.match.config.fighters[1].kit, "the bot wears its kit too");
  d.press({ type: "swap" });
  d.press({ type: "guard", zone: "head" });
  d.press({ type: "guard", zone: "body" });
  assert.equal(d.missing, null, "a swap needs no target");
  const r = d.submit();
  assert.ok(typeof r === "object");
  assert.equal(r.attacks[0], null);
  assert.equal(d.match.fighters[0].weapon, "pistol");
  assert.equal(d.spare, "sniper", "and back again next time");
});

test("without a kit there is nothing to swap to", () => {
  const d = duel();
  assert.equal(d.spare, null);
  assert.equal(d.press({ type: "swap" }), "Второго оружия нет");
});
