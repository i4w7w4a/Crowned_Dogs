import assert from "node:assert/strict";
import { test } from "node:test";
import { ITEMS, MOD_SLOTS, modsFor, type WeaponItem } from "@crowned-dogs/core";
import { PART_ART, bbox, composeWeapon, itemArt, svg, type Slot, type WeaponArtId } from "../src/index.ts";

const weapons = ITEMS.filter((i): i is WeaponItem => i.kind === "weapon");
const stock = Object.fromEntries(weapons.map((w) => [w.weapon, w.stock])) as Record<WeaponArtId, Record<Slot, string>>;

test("every item has an icon, except paints and invisible factory parts", () => {
  for (const item of ITEMS) {
    if (item.kind === "skin") continue;
    if (item.kind === "mod") assert.ok(PART_ART[item.id], `${item.id} can be mounted`);
    if (item.kind === "mod" && item.isStock) continue;
    const art = itemArt(item.id, stock);
    assert.ok(art && art.body.length > 0, `${item.id} has an icon`);
    assert.match(svg(art, item.id), /^<svg [^>]*viewBox="[-\d. ]+"/);
  }
});

test("every mod mounts on every weapon it fits and changes the silhouette", () => {
  for (const w of weapons) {
    const plain = bbox(composeWeapon(w.weapon, w.stock));
    for (const slot of MOD_SLOTS) {
      for (const mod of modsFor(w.weapon, slot)) {
        const shape = composeWeapon(w.weapon, { ...w.stock, [slot]: mod.id });
        assert.ok(shape.body.length > 0);
        if (mod.isStock) assert.deepEqual(bbox(shape), plain);
      }
    }
  }
});
