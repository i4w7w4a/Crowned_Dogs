import assert from "node:assert/strict";
import { test } from "node:test";
import {
  botOrder, canSwap, createMatch, kitFighter, MAPS, resolveTurn, TIMES, validateOrder, VETERAN_KITS, WEAPON_IDS, WEATHERS,
  type Order,
  type Action, type AttackReport, type Side, type TurnReport, type WeaponId,
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
  let swaps = 0;
  for (let i = 0; i < 40; i++) {
    const weapons = [WEAPON_IDS[i % 4]!, WEAPON_IDS[(i >> 2) % 4]!] as const;
    // veteran kits: the sniper carries a pistol as a sidearm
    const fighters = weapons.map((w, k) => kitFighter(String(k), String(k), VETERAN_KITS[w]));
    const guns = fighters.map((f) => [f.kit!.weapon.id, f.kit!.sidearm?.id].filter((g) => g !== undefined) as WeaponId[]);
    let m = createMatch({
      seed: `audio-${i}`,
      mapId: maps[i % maps.length]!,
      weather: WEATHERS[i % WEATHERS.length]!,
      time: TIMES[i % TIMES.length]!,
      fighters: [fighters[0]!, fighters[1]!],
    });
    const preload = new Set<SoundId>(fightSounds(guns[0]!, guns[1]!));
    while (!m.result) {
      // the bot rarely swaps on its own, so make it swap every few turns
      const orders = ([0, 1] as const).map((side): Order => {
        const o = botOrder(m, side);
        const swap: Order = { ...o, action: { type: "swap", move: 0 } };
        return m.turn % 3 === 1 && canSwap(m, side) && !validateOrder(m, side, swap) ? swap : o;
      });
      const step = resolveTurn(m, [orders[0]!, orders[1]!]);
      const inHand: [WeaponId, WeaponId] = [step.match.fighters[0].weapon, step.match.fighters[1].weapon];
      const cues = cuesForTurn(step.report, { viewer: 0, weapons: inHand });
      swaps += step.report.actions.filter((a) => a.type === "swap").length;
      cues.forEach((c, k) => {
        assert.ok(SOUNDS[c.sound], c.sound);
        assert.ok(preload.has(c.sound), `${c.sound} is not preloaded`);
        assert.ok(c.at >= 0);
        if (k > 0) assert.ok(c.at >= cues[k - 1]!.at);
      });
      m = step.match;
    }
  }
  assert.ok(swaps > 10, `only ${swaps} swaps`);
});

test("a swap draws the other gun on the attack beat", () => {
  const swap = report({ actions: [{ type: "swap", move: 1 }, { type: "swap", move: 0 }] });
  const cues = cuesForTurn(swap, { viewer: 0, weapons: ["pistol", "sniper"] });
  assert.deepEqual(sounds(cues), ["step", "gear.rustle", "equip.pistol", "equip.sniper"]);
  assert.equal(cues.find((c) => c.sound === "equip.pistol")!.at, TIMING.ourAttack);
  assert.equal(cues.find((c) => c.sound === "equip.sniper")!.at, TIMING.enemyAttack);
  // without the guns in hand there is nothing to draw, only the step
  assert.deepEqual(sounds(cuesForTurn(swap, { viewer: 0 })), ["step", "gear.rustle"]);
});
