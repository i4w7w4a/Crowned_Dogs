import assert from "node:assert/strict";
import { test } from "node:test";
import { botOrder } from "../src/ai.ts";
import { actionOf, applyCommand, freshDraft, grenadeRefusal, type Draft } from "../src/draft.ts";
import { VETERAN_KITS, kitFighter, starterLoadout } from "../src/loadout.ts";
import {
  canSwap, createMatch, hitChance, resolveTurn, swapWeapon, validateOrder, weaponSpec, type Match, type Order,
} from "../src/match.ts";
import { seedRng } from "../src/rng.ts";

const SWAP: Order = { action: { type: "swap", move: 0 }, defend: ["head", "body"] };
const STAND: Order = { action: { type: "run", move: 2 }, defend: ["head", "body"] };

/** A veteran sniper (with a pistol on the side) against a starter shotgun. */
function sniperMatch(startDistance = 3, seed = "swap"): Match {
  return createMatch({
    seed, mapId: "saltHall", weather: "clear", time: "day", startDistance,
    fighters: [kitFighter("a", "Тень", VETERAN_KITS.sniper), kitFighter("b", "Соль", starterLoadout("shotgun"))],
  });
}

test("only a kit with a second gun can swap", () => {
  const m = sniperMatch();
  assert.equal(canSwap(m, 0), true);
  assert.equal(swapWeapon(m, 0), "pistol");
  assert.equal(canSwap(m, 1), false);
  assert.equal(validateOrder(m, 1, SWAP), "swap_unavailable");
  assert.equal(validateOrder(m, 0, { ...SWAP, action: { type: "swap", move: 2 as never } }), "bad_move");
  assert.equal(validateOrder(m, 0, SWAP), null);
});

test("a swap takes the turn's attack and puts the other gun in hand, and back", () => {
  const m0 = sniperMatch(6);
  const sniperShot = hitChance(m0, 0, "sniper", 3, 0, false);
  const { match: m1, report } = resolveTurn(m0, [SWAP, STAND]);
  assert.equal(report.attacks[0], null, "no shot on the turn of the swap");
  assert.deepEqual(report.actions[0], SWAP.action);
  assert.equal(m1.fighters[0].weapon, "pistol");
  assert.equal(m1.fighters[0].sidearm, true);
  assert.equal(weaponSpec(m1, 0, "pistol"), m1.config.fighters[0].kit!.sidearm, "the pistol fires with its own mods");
  assert.ok(hitChance(m1, 0, "pistol", 3, 0, false) > sniperShot, "the pistol is the better gun up close");
  assert.equal(swapWeapon(m1, 0), "sniper");

  const { match: m2 } = resolveTurn(m1, [SWAP, STAND]);
  assert.equal(m2.fighters[0].weapon, "sniper");
  assert.equal(m2.fighters[0].sidearm, false);
});

test("a swap still moves and guards", () => {
  const m0 = sniperMatch(6);
  const step: Order = { action: { type: "swap", move: -1 }, defend: ["legs", "groin"] };
  const { match, report } = resolveTurn(m0, [step, STAND]);
  assert.equal(report.moves[0], -1);
  assert.deepEqual(match.fighters[0].lastDefend, ["legs", "groin"]);
});

test("the order draft has a swap button that toggles", () => {
  const m = sniperMatch();
  const d0: Draft = { ...freshDraft(m, 0), defend: ["head", "body"] };
  const d1 = applyCommand(m, 0, d0, { type: "swap" });
  assert.ok(typeof d1 !== "string");
  assert.deepEqual(actionOf(d1), { type: "swap", move: 0 });
  const d2 = applyCommand(m, 0, d1, { type: "swap" });
  assert.ok(typeof d2 !== "string");
  assert.equal(d2.kind, "shoot");
  assert.equal(applyCommand(m, 1, freshDraft(m, 1), { type: "swap" }), "Второго оружия нет");
});

test("the grenade toast names the range of the grenade the Persona carries", () => {
  const far = createMatch({
    seed: "swap", mapId: "saltHall", weather: "clear", time: "day", startDistance: 9,
    fighters: [kitFighter("a", "Тень", { ...VETERAN_KITS.sniper, grenade: "grenade-rg1" }), kitFighter("b", "Соль", starterLoadout("shotgun"))],
  });
  assert.equal(grenadeRefusal(far, 0), "Граната летит на 2–8 шагов", "this sniper carries an РГ-1");
  assert.equal(grenadeRefusal(far, 1), "Граната летит на 2–8 шагов");
});

test("the bot takes its pistol when the main gun cannot reach, and not when it can", () => {
  // a shotgun is hopeless at 10 steps; a pistol on the side still reaches
  const kit = { ...VETERAN_KITS.shotgun, weapon2: { item: "pistol-pustelga" } };
  const at = (startDistance: number) => createMatch({
    seed: "bot", mapId: "saltHall", weather: "clear", time: "day", startDistance,
    fighters: [kitFighter("a", "Тень", kit), kitFighter("b", "Соль", starterLoadout("shotgun"))],
  });
  const swaps = (m: Match) => Array.from({ length: 40 }, (_, i) => botOrder(m, 0, seedRng(`b${i}`)))
    .filter((o) => o.action.type === "swap").length;
  const far = swaps(at(10)), near = swaps(at(4));
  assert.ok(far >= 15, `swaps far away: ${far}/40`);
  assert.equal(near, 0, "up close the shotgun stays in hand");
});
