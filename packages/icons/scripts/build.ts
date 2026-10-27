// Writes one SVG per item into packages/icons/svg/. Run: npm run icons
// Weapons come out with their factory parts. Paints (skins) have no icon.

import { mkdirSync, rmSync, writeFileSync } from "node:fs";
import { ITEMS, type WeaponItem } from "@crowned-dogs/core";
import { itemArt, svg, type Slot, type WeaponArtId } from "../src/index.ts";

const out = new URL("../svg/", import.meta.url);
rmSync(out, { recursive: true, force: true });
mkdirSync(out, { recursive: true });

const stock = Object.fromEntries(
  ITEMS.filter((i): i is WeaponItem => i.kind === "weapon").map((w) => [w.weapon, w.stock]),
) as Record<WeaponArtId, Record<Slot, string>>;

let written = 0;
const missing: string[] = [];
for (const item of ITEMS) {
  if (item.kind === "skin") continue;
  const art = itemArt(item.id, stock);
  if (!art) {
    if (!(item.kind === "mod" && item.isStock)) missing.push(item.id);
    continue;
  }
  writeFileSync(new URL(`${item.id}.svg`, out), svg(art, item.id));
  written += 1;
}
console.log(`${written} icons written to packages/icons/svg/`);
if (missing.length) {
  console.error(`No art for: ${missing.join(", ")}`);
  process.exitCode = 1;
}
