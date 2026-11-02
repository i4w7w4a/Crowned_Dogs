import assert from "node:assert/strict";
import { test } from "node:test";
import {
  canThrowGrenade, canUseKnife, createMatch, distanceLimit, grenadeSpec, swapWeapon,
  type Match, type TurnReport, type Action, type AttackReport,
} from "../../../packages/core/reference/match.ts";
import { draftChance, guardsNeeded, missing, type Draft, type DraftKind } from "../../../packages/core/reference/draft.ts";
import { kitFighter, VETERAN_KITS } from "../../../packages/core/reference/loadout.ts";
import { WEAPON_IDS } from "../../../packages/core/reference/rules.ts";
import { callRust } from "../../../packages/runtime/src/index.ts";
import { turnLine, type Line } from "../src/hud/words.ts";

// Migration oracle: these expressions are from main.ts at e8251f1. The rule
// imports intentionally use the frozen pre-Rust implementation, not the facade.
function originalControls(m: Match, d: Draft, busy: boolean) {
  const over = m.result !== null, lock = busy || over;
  const run = d.kind === "run" || d.kind === "dash" || d.kind === "flee";
  const aimed = d.kind === "shoot" || d.kind === "knife";
  const step = Math.abs(d.move) <= 1;
  const knifeMove = canUseKnife(m, d.move) ? d.move : 1;
  const knifeOk = step && canUseKnife(m, knifeMove);
  const nadeOk = step && canThrowGrenade(m, 0);
  const planned = Math.max(1, Math.min(distanceLimit(m), m.distance - d.move));
  return {
    lock, run, aimed, step, knifeMove, knifeOk, nadeOk, planned,
    chance: draftChance(m, 0, d), knifeChance: draftChance(m, 0, { ...d, kind: "knife", move: knifeMove }),
    need: guardsNeeded(d), grenade: grenadeSpec(m, 0), spare: swapWeapon(m, 0),
    message: over ? "Бой окончен" : busy ? "Ход идёт…" : missing(d),
    dist: d.move !== 0 && !busy ? planned : m.distance,
    noShot: (run || d.kind === "swap") && !lock,
  };
}

function close(actual: unknown, expected: unknown, path = "value"): void {
  if (typeof expected === "number") {
    assert.equal(typeof actual, "number", path);
    assert.ok(Math.abs((actual as number) - expected) < 1e-10, `${path}: ${actual} != ${expected}`);
  } else if (expected && typeof expected === "object") {
    assert.ok(actual && typeof actual === "object", path);
    assert.deepEqual(Object.keys(actual as object).sort(), Object.keys(expected).sort(), path);
    for (const [key, value] of Object.entries(expected)) close((actual as Record<string, unknown>)[key], value, `${path}.${key}`);
  } else assert.equal(actual, expected, path);
}

test("Rust HUD retains original availability, chances and copy across range/turn/action boundaries", () => {
  const kinds: DraftKind[] = ["shoot", "knife", "grenade", "swap", "run", "dash", "flee"];
  for (const weapon of WEAPON_IDS) for (const distance of [1, 2, 7, 12]) for (const turn of [1, 7, 14]) {
    const m = createMatch({ seed: "hud parity", mapId: "saltHall", weather: "clear", time: "night", startDistance: distance,
      fighters: [kitFighter("me", "Тень", VETERAN_KITS[weapon]), { id: "foe", name: "Искра", weapon: "rifle" }] });
    m.turn = turn;
    m.fighters[0].grenades = turn === 14 ? 0 : 1;
    m.fighters[0].still = turn % 3;
    m.fighters[0].winded = turn === 7;
    m.result = turn === 14 ? { winner: 0, reason: "ko" } : null;
    for (const kind of kinds) for (const move of [-2, 0, 3]) for (const busy of [false, true]) {
      const d: Draft = { kind, move, zone: move === 0 ? null : "head", defend: busy ? ["head", "body"] : [] };
      close(callRust("ui.battleControls", [m, d, busy]), originalControls(m, d, busy), `${weapon}/${distance}/${turn}/${kind}/${move}/${busy}`);
    }
  }
});

test("Rust health animation retains rounding and the quarter-health threshold", () => {
  for (const max of [70, 100, 125]) for (const hp of [-5, 0, .49, .5, .51, max * .25 - .01, max * .25, max * .25 + .01, max, max + 1]) {
    const result = callRust<{ text: string; width: string; low: boolean }>("ui.health", [hp, max]);
    assert.equal(result.text, String(Math.max(0, Math.round(hp))));
    close(parseFloat(result.width), Math.min(100, Math.max(0, hp / max * 100)));
    assert.equal(result.low, hp > 0 && hp <= max * .25);
  }
});

// Original log composition; words.turnLine has separate differential/core tests.
const icon = (id: string, cls = "") => `<svg class="ico${cls ? " " + cls : ""}" aria-hidden="true"><use href="#${id}"/></svg>`;
function originalLine(side: "me" | "en", who: string, l: Line): string {
  const ic = l.icon === "move" ? icon(l.dash ? "i-mv-3" : "i-mv-2", l.back ? "flip" : "") : icon(l.icon === "shot" ? "i-shot" : "i-" + l.icon);
  const o = l.outcome;
  const res = o.kind === "none" ? `<span class="r nil">${o.text}</span>` : o.kind === "miss" ? '<span class="r nil">мимо</span>'
    : o.kind === "blocked" ? `<span class="r blk">${icon("i-zont")}−${o.damage}</span>` : `<span class="r dmg">${o.crit ? icon("i-crit") : ""}−${o.damage}</span>`;
  return `<div class="ln ${side}">${ic}<span class="w">${who}</span><span class="a">${l.act || "на месте"}</span>${res}</div>`;
}
function originalShort(l: Line, mine: boolean): string {
  const o = l.outcome;
  if (o.kind === "none") return l.icon === "swap" ? "смена оружия" : l.act || "на месте";
  if (o.kind === "miss") return `<em class="nil">${l.icon === "grenade" ? "граната мимо" : "промах"}</em>`;
  if (o.kind === "blocked") return `<em class="blk">Зонт −${o.damage}</em>`;
  return `<em class="${mine ? "us" : "them"}">${l.target} −${o.damage}${o.crit ? icon("i-crit") : ""}</em>`;
}

test("Rust fight log retains icons and result markup for shots, moves, swaps and grenades", () => {
  const actions: Action[] = [{ type: "shoot", move: 0, zone: "head" }, { type: "knife", move: 1, zone: "legs" },
    { type: "grenade", move: 0 }, { type: "swap", move: -1 }, { type: "run", move: -2 }, { type: "dash" }, { type: "wait" }];
  const hit: AttackReport = { weapon: "rifle", zone: "head", chance: .6, hit: true, damage: 30, crit: false, blocked: false, shieldBroken: false };
  const attacks: (AttackReport | null)[] = [hit, { ...hit, blocked: true, damage: 6 }, { ...hit, crit: true }, { ...hit, hit: false, damage: 0 }, { ...hit, weapon: "grenade", zone: null }, null];
  for (const action of actions) for (const attack of attacks) for (const distanceAfter of [6, 7]) {
    const r: TurnReport = { turn: 3, distanceBefore: 7, distanceAfter, limit: 12, actions: [action, action], defend: [[], []], moves: [0, 0], attacks: [attack, attack], hp: [70, 70], afk: [false, false] };
    const lines = [turnLine(r, 0, "pistol"), turnLine(r, 1, "rifle")] as const;
    const dist = r.distanceBefore === r.distanceAfter ? `${r.distanceAfter}` : `${r.distanceBefore} → ${r.distanceAfter}`;
    assert.deepEqual(callRust("ui.fightLog", [r, ["Тень", "Искра"], ["pistol", "rifle"]]), {
      entry: `<div class="eh"><span>Ход ${r.turn}</span><span>дистанция ${dist}</span></div>` + originalLine("me", "Ты", lines[0]) + originalLine("en", "Искра", lines[1]),
      ticker: `<b>Ход ${r.turn}</b>Ты: ${originalShort(lines[0], true)} · Искра: ${originalShort(lines[1], false)}`,
    });
  }
});

test("Rust loader uses the original rotation/breath and cover distance", () => {
  for (const time of [0, .3, 4, 17, 103]) for (const reduced of [true, false]) for (const idle of [null, .5, 2, 8.7]) {
    const sway = reduced ? 0 : 1, rad = Math.PI / 180;
    close(callRust("ui.loaderPose", [time, reduced, idle]), {
      rotation: [(16 + sway * 2 * Math.sin(time * .7)) * rad, (-22 + sway * 9 * Math.sin(time * .45)) * rad, (-13 + sway * 1.5 * Math.cos(time * .33)) * rad],
      scale: 1 + .012 * (idle === null ? 0 : .5 - .5 * Math.cos(2 * Math.PI * (idle % 4) / 4)),
    });
  }
  for (const aspect of [320 / 640, 390 / 844, 1920 / 1080]) {
    const vfov = 35 * Math.PI / 180 / 2, hfov = Math.atan(Math.tan(vfov) * aspect);
    close(callRust("ui.loaderDistance", [35, aspect, 2.5]), 2.5 / Math.sin(Math.min(vfov, hfov)) * 1.02);
  }
});

test("Rust battle cover and reticle constraints retain phone and desktop placement", () => {
  for (const [cw, ch] of [[320, 640], [390, 844], [768, 1024], [1920, 1080]]) for (const frame of [{ w: 941, h: 1672, top: 0 }, { w: 1672, h: 941, top: .1 }, { w: 1672, h: 941, top: -.1 }]) {
    const top = ch! * frame.top, s = Math.max(cw! / frame.w, (ch! - top) / frame.h), w = frame.w * s, h = frame.h * s;
    const x = (cw! - w) * .5, vx = Math.max(0, -x), vy = Math.max(0, -top), radius = Math.max(6, w * .02 - .75);
    close(callRust("ui.layout", [cw, ch, frame]), { x, y: top, w, h, radius, viewport: { x: vx / w, y: vy / h, w: (Math.min(w, cw! - x) - vx) / w, h: (Math.min(h, ch! - top) - vy) / h } });
    const points = [[cw! * .5, ch! * .2], [cw! * .52, ch! * .45], [cw! * .5, ch! * .6], [cw! * .5, ch! * .8]];
    const ctlTop = ch! * .65, placed: [number, number, number][] = [];
    let limit = ctlTop - radius - 6;
    for (let i = points.length - 1; i >= 0; i--) {
      const at = Math.min(points[i]![1]!, limit);
      placed[i] = [points[i]![0]!, at, 0]; limit = at - Math.max(22, 2 * radius + 8);
    }
    for (const [i, p] of placed.entries()) {
      let near = 40;
      for (const [j, other] of placed.entries()) if (j !== i) near = Math.min(near, Math.hypot(other[0] - p[0], other[1] - p[1]));
      p[2] = Number(Math.max(2 * radius + 4, near).toFixed(0));
    }
    close(callRust("ui.reticles", [points, radius, ctlTop]), placed);
  }
});
