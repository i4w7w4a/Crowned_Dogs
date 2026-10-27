import assert from "node:assert/strict";
import { test } from "node:test";
import {
  createMatch, distanceLimit, hitChance, resolveTurn, validateOrder,
  type Match, type MatchConfig, type Order,
} from "../src/match.ts";
import type { WeaponId } from "../src/rules.ts";

function duel(over: Partial<MatchConfig> = {}, weapons: [WeaponId, WeaponId] = ["rifle", "rifle"]): Match {
  return createMatch({
    seed: "test",
    mapId: "saltHall",
    weather: "clear",
    time: "morning",
    fighters: [
      { id: "a", name: "Тень", weapon: weapons[0] },
      { id: "b", name: "Искра", weapon: weapons[1] },
    ],
    ...over,
  });
}

const shoot = (move: -1 | 0 | 1 = 0): Order => ({ action: { type: "shoot", move, zone: "body" }, defend: ["head", "body"] });
const runBack: Order = { action: { type: "run", move: -2 }, defend: ["head", "body"] };
const runIn: Order = { action: { type: "run", move: 2 }, defend: ["head", "body"] };

test("a new duel starts at 7 steps with full health", () => {
  const m = duel();
  assert.equal(m.distance, 7);
  assert.equal(m.turn, 1);
  assert.deepEqual(m.fighters.map((f) => f.hp), [100, 100]);
  assert.equal(m.result, null);
});

test("orders are checked against the rules", () => {
  const m = duel();
  assert.equal(validateOrder(m, 0, shoot()), null);
  assert.equal(validateOrder(m, 0, { action: { type: "knife", move: 1, zone: "head" }, defend: ["head", "body"] }), "knife_out_of_reach");
  assert.equal(validateOrder(m, 0, { action: { type: "dash" }, defend: ["head", "body"] }), "bad_defence", "a dash keeps only one zone");
  assert.equal(validateOrder(m, 0, { action: { type: "dash" }, defend: ["head"] }), null);
  assert.equal(validateOrder(m, 0, { action: { type: "run", move: 2 }, defend: ["head"] }), "bad_defence", "a run keeps two zones");
  assert.equal(validateOrder(m, 0, { action: { type: "run", move: 1 as never }, defend: ["head", "body"] }), "bad_move");
  assert.equal(validateOrder(m, 0, { action: { type: "shoot", move: 2 as never, zone: "head" }, defend: ["head", "body"] }), "bad_move");
  assert.equal(validateOrder(m, 0, { action: { type: "shoot", move: 0, zone: "head" }, defend: ["head", "head"] }), "bad_defence");
  assert.equal(validateOrder(m, 0, { action: { type: "flee" }, defend: ["head", "body"] }), "flee_unavailable");
  const close = duel({ startDistance: 1 });
  assert.equal(validateOrder(close, 0, { action: { type: "grenade", move: 0 }, defend: ["head", "body"] }), "grenade_unavailable");
  assert.equal(validateOrder(close, 0, { action: { type: "knife", move: 0, zone: "head" }, defend: ["head", "body"] }), null);
});

test("an invalid order is refused, not silently fixed", () => {
  const m = duel();
  assert.throws(() => resolveTurn(m, [{ action: { type: "dash" }, defend: ["head", "body"] }, shoot()]));
});

test("movement happens first, then both attacks fly at the new distance", () => {
  const m = duel();
  const { match, report } = resolveTurn(m, [runIn, { action: { type: "shoot", move: 1, zone: "head" }, defend: ["head", "body"] }]);
  assert.equal(report.distanceBefore, 7);
  assert.equal(report.distanceAfter, 4);
  assert.equal(match.distance, 4);
  assert.equal(report.attacks[0], null, "a run replaces the attack");
  assert.ok(report.attacks[1], "the other side still shoots");
  assert.equal(m.distance, 7, "the old match is not changed");
});

test("the same seed and the same orders replay the same fight", () => {
  const play = () => {
    let m = duel({ seed: "replay" });
    const log = [];
    for (let i = 0; i < 6 && !m.result; i++) {
      const r = resolveTurn(m, [shoot(), shoot(i % 2 ? 1 : 0)]);
      m = r.match;
      log.push(r.report);
    }
    return log;
  };
  assert.deepEqual(play(), play());
});

test("the field shrinks after turn 6, down to 4 steps", () => {
  const m = duel({ mapId: "dam" });
  assert.equal(distanceLimit(m, 1), 14);
  assert.equal(distanceLimit(m, 6), 14);
  assert.equal(distanceLimit(m, 7), 13);
  assert.equal(distanceLimit(m, 12), 8);
  assert.equal(distanceLimit(m, 30), 4);
});

test("a sniper needs a stance: standing this turn and the last one", () => {
  const m = duel({ startDistance: 9 }, ["sniper", "rifle"]);
  const standing = hitChance(m, 0, "sniper", 9, 2, false);
  const moved = hitChance(m, 0, "sniper", 9, 0, false);
  assert.ok(standing > 0.8, String(standing));
  assert.ok(moved < standing / 2 + 0.01, `${moved} vs ${standing}`);
});

test("a dash makes you harder to hit", () => {
  const m = duel();
  const normal = hitChance(m, 0, "rifle", 6, 1, false);
  const dashing = hitChance(m, 0, "rifle", 6, 1, true);
  assert.ok(Math.abs(normal - dashing - 0.1) < 1e-9);
});

test("weather and time of day change accuracy", () => {
  const clear = hitChance(duel(), 0, "rifle", 10, 1, false);
  const dust = hitChance(duel({ weather: "dust" }), 0, "rifle", 10, 1, false);
  const night = hitChance(duel({ time: "night" }), 0, "rifle", 10, 1, false);
  const nightNvg = hitChance(
    createMatch({ ...duel().config, time: "night", fighters: [{ id: "a", name: "a", weapon: "rifle", gear: { nvg: true } }, { id: "b", name: "b", weapon: "rifle" }] }),
    0, "rifle", 10, 1, false,
  );
  assert.ok(dust < clear);
  assert.ok(night < clear);
  assert.equal(nightNvg, clear);
});

test("at night a shot without a suppressor gives you away", () => {
  const m = createMatch({
    ...duel().config,
    time: "night",
    fighters: [
      { id: "a", name: "a", weapon: "rifle" },
      { id: "b", name: "b", weapon: "rifle", gear: { suppressor: true } },
    ],
  });
  const { match } = resolveTurn(m, [shoot(), shoot()]);
  assert.equal(match.fighters[0].flash, true);
  assert.equal(match.fighters[1].flash, false);
  assert.ok(hitChance(match, 1, "rifle", 6, 1, false) > hitChance(match, 0, "rifle", 6, 1, false));
});

test("missing the timer three times in a row loses the fight", () => {
  let m = duel();
  for (let i = 0; i < 3; i++) m = resolveTurn(m, [null, runBack]).match;
  assert.deepEqual(m.result, { winner: 1, reason: "afk" });
});

test("a missed turn keeps last turn's defence and does not attack", () => {
  let m = duel();
  m = resolveTurn(m, [{ action: { type: "run", move: -2 }, defend: ["groin", "legs"] }, runBack]).match;
  const { report } = resolveTurn(m, [null, runIn]);
  assert.deepEqual(report.defend[0], ["groin", "legs"]);
  assert.deepEqual(report.actions[0], { type: "wait" });
  assert.equal(report.attacks[0], null);
});

test("leaving the fight is only possible at the edge, and the other side wins", () => {
  const m = duel({ startDistance: 11 });
  const flee: Order = { action: { type: "flee" }, defend: ["head", "body"] };
  assert.equal(validateOrder(m, 0, flee), null);
  const { match } = resolveTurn(m, [flee, runBack]);
  assert.deepEqual(match.result, { winner: 1, reason: "fled" });
});

test("when time runs out, the higher health share wins; equal health is a draw", () => {
  // A storm ends the fight after 10 turns. Nobody shoots, so it is a draw.
  let m = duel({ weather: "storm" });
  let n = 0;
  while (!m.result) {
    m = resolveTurn(m, n % 2 ? [runIn, runIn] : [runBack, runBack]).match;
    n += 1;
  }
  assert.equal(n, 10);
  assert.deepEqual(m.result, { winner: null, reason: "turn_limit" });
});

test("a fight ends when someone is out of health", () => {
  let m = duel({ seed: "ko", startDistance: 2 }, ["shotgun", "pistol"]);
  while (!m.result) m = resolveTurn(m, [shoot(), { action: { type: "shoot", move: 0, zone: "legs" }, defend: ["groin", "legs"] }]).match;
  assert.equal(m.result.reason, "ko");
  const loser = m.fighters[m.result.winner === 0 ? 1 : 0];
  assert.equal(loser.hp, 0);
});
