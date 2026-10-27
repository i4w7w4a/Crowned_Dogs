import assert from "node:assert/strict";
import { test } from "node:test";
import {
  botOrder, createMatch, MAPS, resolveTurn, TIMES, WEAPON_IDS, WEATHERS,
  type Action, type AttackReport, type Side, type TurnReport,
} from "@crowned-dogs/core";
import { SOUNDS, type SoundId } from "../src/bank.ts";
import { cuesForTurn, fightSounds, TIMING, type Cue } from "../src/cues.ts";

const wait: Action = { type: "wait" };

function report(over: Partial<TurnReport>): TurnReport {
  return {
    turn: 1,
    distanceBefore: 5,
    distanceAfter: 5,
    limit: 4,
    actions: [wait, wait],
    defend: [[], []],
    moves: [0, 0],
    attacks: [null, null],
    hp: [100, 100],
    afk: [false, false],
    ...over,
  };
}

function attack(over: Partial<AttackReport>): AttackReport {
  return { weapon: "rifle", zone: "body", chance: 0.5, hit: true, damage: 20, crit: false, blocked: false, shieldBroken: false, ...over };
}

const shoot = (move: -1 | 0 | 1 = 0): Action => ({ type: "shoot", move, zone: "body" });
const sounds = (cues: Cue[]) => cues.map((c) => c.sound);
const ours = (a: AttackReport, viewer: Side = 0) =>
  cuesForTurn(report({ actions: [shoot(), wait], attacks: [a, null] }), { viewer });

test("our shot is near, theirs is far, and ours lands first", () => {
  const cues = cuesForTurn(
    report({ actions: [shoot(), shoot()], attacks: [attack({ weapon: "sniper" }), attack({ weapon: "shotgun" })] }),
    { viewer: 0 },
  );
  const near = cues.find((c) => c.sound === "shot.sniper.near")!;
  const far = cues.find((c) => c.sound === "shot.shotgun.far")!;
  assert.equal(near.at, TIMING.ourAttack);
  assert.equal(far.at, TIMING.enemyAttack);
  assert.ok(near.pan < 0 && far.pan > 0);
  // the same turn from the other side swaps them
  const theirs = cuesForTurn(
    report({ actions: [shoot(), shoot()], attacks: [attack({ weapon: "sniper" }), attack({ weapon: "shotgun" })] }),
    { viewer: 1 },
  );
  assert.ok(sounds(theirs).includes("shot.shotgun.near"));
  assert.ok(sounds(theirs).includes("shot.sniper.far"));
});

test("our rifle drops a casing, the shotgun keeps its shell", () => {
  assert.ok(sounds(ours(attack({ weapon: "rifle" }))).includes("casing"));
  assert.ok(!sounds(ours(attack({ weapon: "shotgun" }))).includes("casing"));
});

test("a hit sounds like plate on armoured zones and like the suit elsewhere", () => {
  assert.ok(sounds(ours(attack({ zone: "body" }))).includes("hit.armor"));
  assert.ok(sounds(ours(attack({ zone: "legs" }))).includes("hit.armor"));
  assert.ok(sounds(ours(attack({ zone: "groin" }))).includes("hit.suit"));
  const head = sounds(ours(attack({ zone: "head" })));
  assert.ok(head.includes("hit.suit") && head.includes("hit.head"));
  // gear can change the zone's armour
  const naked = cuesForTurn(report({ actions: [shoot(), wait], attacks: [attack({ zone: "body" }), null] }), {
    viewer: 0,
    armor: [{}, { body: 0 }],
  });
  assert.ok(sounds(naked).includes("hit.suit"));
});

test("misses, shield blocks, breaks and crits", () => {
  assert.ok(sounds(ours(attack({ hit: false }))).includes("miss.far"));
  assert.ok(sounds(ours(attack({ hit: false }), 1)).includes("miss.near"));

  const block = sounds(ours(attack({ blocked: true })));
  assert.ok(block.includes("shield.block"));
  assert.ok(!block.includes("hit.armor"));

  // a broken shield lets a quieter hit through; across the hall both fade with distance
  const level = (cues: Cue[]) => cues.find((c) => c.sound === "hit.armor")!.gainDb;
  const broken = ours(attack({ blocked: true, shieldBroken: true }));
  assert.ok(sounds(broken).includes("shield.break"));
  assert.equal(level(broken), level(ours(attack({}))) - 6);

  assert.ok(sounds(ours(attack({ crit: true }))).includes("hit.crit"));
});

test("knife and grenade", () => {
  const knife = cuesForTurn(
    report({ actions: [{ type: "knife", move: 1, zone: "body" }, wait], attacks: [attack({ weapon: "knife" }), null] }),
    { viewer: 0 },
  );
  assert.deepEqual(sounds(knife), ["step", "gear.rustle", "knife.swing", "knife.hit"]);

  const blocked = ours(attack({ weapon: "knife", blocked: true }));
  assert.ok(sounds(blocked).includes("knife.blocked"));

  const grenade = cuesForTurn(
    report({ actions: [{ type: "grenade", move: 0 }, wait], attacks: [attack({ weapon: "grenade" }), null] }),
    { viewer: 0 },
  );
  assert.deepEqual(sounds(grenade), ["grenade.pin", "grenade.throw", "grenade.bounce", "grenade.blast", "hit.suit"]);
});

test("running, dashing and a knockout", () => {
  const run = sounds(cuesForTurn(report({ actions: [{ type: "run", move: 2 }, { type: "dash" }] }), { viewer: 0 }));
  assert.equal(run.filter((s) => s === "step").length, 5);
  assert.ok(run.includes("dash.whoosh"));

  const ko = cuesForTurn(report({ actions: [shoot(), wait], attacks: [attack({}), null], hp: [40, 0] }), { viewer: 1 });
  assert.deepEqual(sounds(ko).slice(-3), ["body.fall", "persona.powerdown", "sync.lost"]);
  assert.ok(ko.at(-1)!.at > TIMING.ourAttack);
});

test("the enemy grows quieter with distance, never by more than the cap", () => {
  const at = (d: number) =>
    cuesForTurn(report({ distanceAfter: d, actions: [wait, shoot()], attacks: [null, attack({})] }), { viewer: 0 })
      .find((c) => c.sound === "shot.rifle.far")!.gainDb;
  assert.equal(at(1), 0);
  assert.ok(at(6) < at(3));
  assert.equal(at(16), -6);
});

test("whole bot fights only ever ask for sounds that exist, in time order", () => {
  const maps = Object.keys(MAPS);
  for (let i = 0; i < 40; i++) {
    const weapons = [WEAPON_IDS[i % 4]!, WEAPON_IDS[(i >> 2) % 4]!] as const;
    let m = createMatch({
      seed: `audio-${i}`,
      mapId: maps[i % maps.length]!,
      weather: WEATHERS[i % WEATHERS.length]!,
      time: TIMES[i % TIMES.length]!,
      fighters: [
        { id: "a", name: "a", weapon: weapons[0] },
        { id: "b", name: "b", weapon: weapons[1] },
      ],
    });
    const preload = new Set<SoundId>(fightSounds(weapons[0], weapons[1]));
    while (!m.result) {
      const step = resolveTurn(m, [botOrder(m, 0), botOrder(m, 1)]);
      const cues = cuesForTurn(step.report, { viewer: 0 });
      cues.forEach((c, k) => {
        assert.ok(SOUNDS[c.sound], c.sound);
        assert.ok(preload.has(c.sound), `${c.sound} is not preloaded`);
        assert.ok(c.at >= 0);
        if (k > 0) assert.ok(c.at >= cues[k - 1]!.at);
      });
      m = step.match;
    }
  }
});
