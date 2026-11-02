// Bot against bot with items. Three checks:
//   npm run kits              veteran kits against each other, and each veteran against its own starter kit
//   npm run kits -- items     every item alone on a starter kit: how much it moves the win rate
// An optional last number sets fights per pair (default 400). Starter kits play
// exactly like npm run balance; differences are measured on the same seeds, so
// small shifts are real and not dice.

import { availableParallelism } from "node:os";
import { Worker, isMainThread, parentPort, workerData } from "node:worker_threads";
import { botOrder } from "../src/ai.ts";
import { ITEMS, MOD_SLOTS, type GearSlot } from "../src/items.ts";
import { VETERAN_KITS, kitFighter, starterLoadout, type Loadout } from "../src/loadout.ts";
import { createMatch, resolveTurn, type Side } from "../src/match.ts";
import { MAPS, TIMES, WEAPONS, WEAPON_IDS, WEATHERS, type WeaponId } from "../src/rules.ts";

interface Job {
  a: Loadout;
  b: Loadout;
  fights: number;
  seed: string;
}

const maps = Object.keys(MAPS);

/** Share of fights the first kit wins (draws count half). Sides swap every other fight. */
function score(job: Job): number {
  let won = 0;
  for (let i = 0; i < job.fights; i++) {
    const swap = i % 2 === 1;
    const [x, y] = swap ? [job.b, job.a] : [job.a, job.b];
    let m = createMatch({
      seed: `${job.seed}-${i >> 1}`,
      mapId: maps[(i >> 1) % maps.length]!,
      weather: WEATHERS[(i >> 3) % WEATHERS.length]!,
      time: TIMES[(i >> 5) % TIMES.length]!,
      fighters: [kitFighter("a", "a", x), kitFighter("b", "b", y)],
    });
    while (!m.result) {
      const orders = ([0, 1] as Side[]).map((s) => botOrder(m, s));
      m = resolveTurn(m, [orders[0]!, orders[1]!]).match;
    }
    const mine: Side = swap ? 1 : 0;
    if (m.result.winner === mine) won += 1;
    else if (m.result.winner === null) won += 0.5;
  }
  return won / job.fights;
}

async function runAll(jobs: Job[]): Promise<number[]> {
  const out = new Array<number>(jobs.length);
  const size = Math.min(availableParallelism(), jobs.length);
  let next = 0;
  await Promise.all(Array.from({ length: size }, () => new Promise<void>((done, fail) => {
    const w = new Worker(new URL(import.meta.url));
    const feed = () => {
      if (next >= jobs.length) { void w.terminate(); done(); return; }
      const id = next++;
      w.postMessage({ ...jobs[id], id });
    };
    w.on("message", (r: { id: number; value: number }) => { out[r.id] = r.value; feed(); });
    w.on("error", fail);
    feed();
  })));
  return out;
}

const pct = (v: number) => `${Math.round(v * 100)}%`;
const name = (w: WeaponId) => WEAPONS[w].name;

async function main(): Promise<void> {
  const args = process.argv.slice(2);
  const fights = Number(args.find((a) => /^\d+$/.test(a)) ?? 400);
  if (args.includes("items")) await itemScan(fights);
  else await veterans(fights);
}

async function veterans(fights: number): Promise<void> {
  const pairs = WEAPON_IDS.flatMap((a) => WEAPON_IDS.map((b) => [a, b] as const));
  const vet = await runAll(pairs.map(([a, b]) => ({ a: VETERAN_KITS[a], b: VETERAN_KITS[b], fights, seed: `vet-${a}-${b}` })));
  const base = await runAll(pairs.map(([a, b]) => ({ a: starterLoadout(a), b: starterLoadout(b), fights, seed: `vet-${a}-${b}` })));
  const gap = await runAll(WEAPON_IDS.map((w) => ({ a: VETERAN_KITS[w], b: starterLoadout(w), fights, seed: `gap-${w}` })));

  const table = (title: string, values: number[]) => {
    console.log(`\n${title}`);
    console.log("".padEnd(12) + WEAPON_IDS.map((w) => name(w).padStart(11)).join("") + "    в среднем");
    WEAPON_IDS.forEach((a, i) => {
      const row = values.slice(i * WEAPON_IDS.length, (i + 1) * WEAPON_IDS.length);
      const avg = row.reduce((x, y) => x + y, 0) / row.length;
      console.log(name(a).padEnd(12) + row.map((v) => pct(v).padStart(11)).join("") + pct(avg).padStart(13));
    });
  };
  table("Стартовое снаряжение у обоих (как npm run balance):", base);
  table("Ветеранские наборы у обоих:", vet);
  console.log("\nВетеран против себя же в стартовом:");
  WEAPON_IDS.forEach((w, i) => console.log(`  ${name(w).padEnd(10)} ${pct(gap[i]!)}`));
}

/** Where an item can go on a starter kit of this class, or null if it does not fit. */
function withItem(w: WeaponId, itemId: string): Loadout | null {
  const item = ITEMS.find((i) => i.id === itemId)!;
  const kit = starterLoadout(w);
  switch (item.kind) {
    case "mod":
      if (item.isStock || !item.fits.includes(w)) return null;
      return { ...kit, weapon1: { ...kit.weapon1, mods: { [item.slot]: item.id } } };
    case "head": case "body": case "suit": case "boots": case "knife": case "grenade": {
      const slot: GearSlot = item.kind;
      if (kit[slot] === item.id) return null;
      return { ...kit, [slot]: item.id };
    }
    default:
      return null;
  }
}

async function itemScan(fights: number): Promise<void> {
  const rows: { item: string; w: WeaponId }[] = [];
  for (const item of ITEMS) for (const w of WEAPON_IDS) if (withItem(w, item.id)) rows.push({ item: item.id, w });
  // Against every class's starter kit, on the same seeds as the plain starter.
  const jobs: Job[] = [];
  for (const r of rows) for (const foe of WEAPON_IDS) {
    jobs.push({ a: withItem(r.w, r.item)!, b: starterLoadout(foe), fights, seed: `item-${r.w}-${foe}` });
  }
  const plain = await runAll(WEAPON_IDS.flatMap((w) => WEAPON_IDS.map((foe) => ({ a: starterLoadout(w), b: starterLoadout(foe), fights, seed: `item-${w}-${foe}` }))));
  const values = await runAll(jobs);
  const avg = (xs: number[]) => xs.reduce((a, b) => a + b, 0) / xs.length;
  const plainAvg = Object.fromEntries(WEAPON_IDS.map((w, i) => [w, avg(plain.slice(i * 4, i * 4 + 4))])) as Record<WeaponId, number>;
  console.log("Предмет на стартовом наборе: изменение средней доли побед против всех классов (пункты)");
  const lines = rows.map((r, i) => {
    const v = avg(values.slice(i * 4, i * 4 + 4));
    const item = ITEMS.find((x) => x.id === r.item)!;
    return { text: `${item.name.padEnd(34)} ${name(r.w).padEnd(10)} ${((v - plainAvg[r.w]) * 100).toFixed(1).padStart(6)}`, d: v - plainAvg[r.w], kind: item.kind };
  });
  for (const l of lines) console.log(l.text);
  const mods = lines.filter((l) => l.kind === "mod").map((l) => l.d);
  console.log(`\nОбвесы: среднее ${(avg(mods) * 100).toFixed(1)}, от ${(Math.min(...mods) * 100).toFixed(1)} до ${(Math.max(...mods) * 100).toFixed(1)} пунктов.`);
  console.log(`Слотов на оружии: ${MOD_SLOTS.length}.`);
}

if (!isMainThread) {
  parentPort!.on("message", (job: Job & { id: number }) => parentPort!.postMessage({ id: job.id, value: score(job) }));
} else {
  await main();
}
