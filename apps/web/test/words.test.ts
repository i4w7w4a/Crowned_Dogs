import assert from "node:assert/strict";
import { test } from "node:test";
import type { Action, AttackReport, TurnReport } from "@crowned-dogs/core";
import { endWords, steps, turnLine } from "../src/hud/words.ts";

const shot = (o: Partial<AttackReport> = {}): AttackReport => ({
  weapon: "rifle", zone: "head", chance: 0.6, hit: true, damage: 31, crit: false, blocked: false, shieldBroken: false, ...o,
});
const report = (actions: [Action, Action], attacks: [AttackReport | null, AttackReport | null]): TurnReport => ({
  turn: 3, distanceBefore: 7, distanceAfter: 6, limit: 11, actions, defend: [["head", "body"], ["legs", "groin"]],
  moves: [1, 0], attacks, hp: [80, 69], afk: [false, false],
});

test("Russian counts", () => {
  assert.deepEqual([1, 2, 5, 11, 21, 22].map((n) => `${n} ${steps(n)}`), ["1 шаг", "2 шага", "5 шагов", "11 шагов", "21 шаг", "22 шага"]);
});

test("a log line says the move, the attack and what came of it", () => {
  const r = report(
    [{ type: "shoot", move: 1, zone: "head" }, { type: "knife", move: 0, zone: "legs" }],
    [shot({ crit: true }), shot({ weapon: "knife", zone: "legs", blocked: true, damage: 6 })],
  );
  const mine = turnLine(r, 0, "rifle"), theirs = turnLine(r, 1, "rifle");
  assert.equal(mine.act, "шаг вперёд, выстрел в голову");
  assert.deepEqual(mine.outcome, { kind: "hit", damage: 31, crit: true });
  assert.equal(mine.target, "голова");
  assert.equal(theirs.act, "нож по ногам");
  assert.deepEqual(theirs.outcome, { kind: "blocked", damage: 6 });
});

test("a swap names the gun taken; a run and a missed timer have no attack", () => {
  const r = report([{ type: "swap", move: -1 }, { type: "run", move: -2 }], [null, null]);
  const swap = turnLine(r, 0, "pistol"), run = turnLine(r, 1, "rifle");
  assert.equal(swap.icon, "swap");
  assert.equal(swap.act, "шаг назад, взять пистолет");
  assert.deepEqual(swap.outcome, { kind: "none", text: "без выстрела" });
  assert.equal(run.act, "перебежка назад");
  assert.ok(run.back);
  const wait = turnLine(report([{ type: "wait" }, { type: "dash" }], [null, null]), 0, "rifle");
  assert.equal(wait.act, "приказ не отдан");
});

test("a grenade is named by its kind", () => {
  const r = report([{ type: "grenade", move: 0 }, { type: "grenade", move: 0 }],
    [shot({ weapon: "grenade", zone: null, damage: 22 }), shot({ weapon: "grenade", zone: null, hit: false, damage: 0, effect: "emp" })]);
  assert.equal(turnLine(r, 0, "rifle").target, "граната");
  assert.equal(turnLine(r, 1, "rifle").act, "ЭМИ-граната");
  assert.deepEqual(turnLine(r, 1, "rifle").outcome, { kind: "miss" });
});

test("the end screen speaks of the Personas, not of the player's gender", () => {
  const won = endWords({ winner: 0, reason: "ko" }, 0, "Искра", 5);
  assert.equal(won.title, "Цель подавлена");
  assert.equal(won.text, "Персона «Искра» выведена из строя за 5 ходов. Добыча твоя.");
  const lost = endWords({ winner: 1, reason: "afk" }, 0, "Искра", 3);
  assert.ok(lost.lost);
  assert.equal(lost.again, "Очнуться в капсуле");
  assert.equal(endWords({ winner: null, reason: "double_ko" }, 0, "Искра", 8).title, "Ничья");
});
