// Bot against bot for every pair of weapons. Prints how often the row weapon
// beats the column weapon, and its average over all opponents. The concept
// doc's solver put every weapon's average between 41% and 57%; single pairs
// are allowed to be lopsided (pistol beats shotgun, shotgun beats sniper,
// rifle beats pistol). Run: npm run balance [fights per pair]

import { botOrder } from "../src/ai.ts";
import { createMatch, resolveTurn, type Match, type Side } from "../src/match.ts";
import { MAPS, TIMES, WEAPON_IDS, WEATHERS, WEAPONS, type WeaponId } from "../src/rules.ts";

const fights = Number(process.argv[2] ?? 400);
const maps = Object.keys(MAPS);

function play(seed: string, a: WeaponId, b: WeaponId, i: number): Match {
  let m = createMatch({
    seed,
    mapId: maps[i % maps.length]!,
    weather: WEATHERS[(i >> 2) % WEATHERS.length]!,
    time: TIMES[(i >> 4) % TIMES.length]!,
    fighters: [{ id: "a", name: "a", weapon: a }, { id: "b", name: "b", weapon: b }],
  });
  while (!m.result) {
    const orders = ([0, 1] as Side[]).map((s) => botOrder(m, s));
    m = resolveTurn(m, [orders[0]!, orders[1]!]).match;
  }
  return m;
}

const rows: string[] = [];
const header = "".padEnd(12) + WEAPON_IDS.map((w) => WEAPONS[w].name.padStart(11)).join("") + "    в среднем";
rows.push(header);
let turns = 0, games = 0;
const reasons: Record<string, number> = {};
for (const a of WEAPON_IDS) {
  let line = WEAPONS[a].name.padEnd(12);
  let sum = 0;
  for (const b of WEAPON_IDS) {
    let score = 0;
    for (let i = 0; i < fights; i++) {
      // Swap sides every other fight so the first slot has no edge.
      const swap = i % 2 === 1;
      const m = play(`bal-${a}-${b}-${i}`, swap ? b : a, swap ? a : b, i);
      const mine: Side = swap ? 1 : 0;
      if (m.result!.winner === mine) score += 1;
      else if (m.result!.winner === null) score += 0.5;
      turns += m.history.length;
      games += 1;
      reasons[m.result!.reason] = (reasons[m.result!.reason] ?? 0) + 1;
    }
    line += `${((score / fights) * 100).toFixed(0)}%`.padStart(11);
    sum += score / fights;
  }
  line += `${((sum / WEAPON_IDS.length) * 100).toFixed(0)}%`.padStart(13);
  rows.push(line);
}
console.log(rows.join("\n"));
console.log(`\nСредняя длина боя: ${(turns / games).toFixed(1)} хода. Концовки: ${JSON.stringify(reasons)}`);
