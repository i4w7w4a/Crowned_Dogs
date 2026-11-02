import assert from "node:assert/strict";
import { test } from "node:test";
import { botOrder } from "../src/ai.ts";
import { createMatch, resolveTurn, turnLimit, validateOrder, type Side } from "../src/match.ts";
import { MAPS, TIMES, WEAPON_IDS, WEATHERS } from "../src/rules.ts";

test("bot against bot: every order is legal and every fight ends in time", () => {
  const maps = Object.keys(MAPS);
  const endings: Record<string, number> = {};
  for (let i = 0; i < 300; i++) {
    let m = createMatch({
      seed: `bots-${i}`,
      mapId: maps[i % maps.length]!,
      weather: WEATHERS[i % WEATHERS.length]!,
      time: TIMES[(i >> 2) % TIMES.length]!,
      fighters: [
        { id: "a", name: "a", weapon: WEAPON_IDS[i % WEAPON_IDS.length]! },
        { id: "b", name: "b", weapon: WEAPON_IDS[(i >> 2) % WEAPON_IDS.length]! },
      ],
    });
    const limit = turnLimit(m);
    while (!m.result) {
      const orders = ([0, 1] as Side[]).map((s) => {
        const o = botOrder(m, s);
        assert.equal(validateOrder(m, s, o), null, JSON.stringify(o));
        return o;
      });
      m = resolveTurn(m, [orders[0]!, orders[1]!]).match;
      assert.ok(m.turn <= limit);
    }
    endings[m.result.reason] = (endings[m.result.reason] ?? 0) + 1;
  }
  // Most fights should be decided by damage, not by the clock.
  assert.ok((endings.ko ?? 0) > 150, JSON.stringify(endings));
});

test("the bot is deterministic for the same match and turn", () => {
  const m = createMatch({
    seed: "same",
    mapId: "blocks",
    weather: "clear",
    time: "day",
    fighters: [{ id: "a", name: "a", weapon: "shotgun" }, { id: "b", name: "b", weapon: "sniper" }],
  });
  assert.deepEqual(botOrder(m, 0), botOrder(m, 0));
});

test("a shotgun closes in and a sniper keeps its distance", () => {
  let closer = 0, keeper = 0;
  for (let i = 0; i < 40; i++) {
    const m = createMatch({
      seed: `range-${i}`,
      mapId: "dam",
      weather: "clear",
      time: "morning",
      fighters: [{ id: "a", name: "a", weapon: "shotgun" }, { id: "b", name: "b", weapon: "sniper" }],
    });
    const forward = (o: ReturnType<typeof botOrder>) =>
      o.action.type === "dash" ? 3 : "move" in o.action ? o.action.move : 0;
    if (forward(botOrder(m, 0)) > 0) closer += 1;
    if (forward(botOrder(m, 1)) <= 0) keeper += 1;
  }
  assert.ok(closer >= 30, `shotgun moved in ${closer}/40`);
  assert.ok(keeper >= 30, `sniper held back ${keeper}/40`);
});
