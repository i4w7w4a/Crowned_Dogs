import assert from "node:assert/strict";
import test from "node:test";
import * as original from "../../../crates/game-wasm/reference/icons/shapes.ts";
import * as legacyDraw from "../../../crates/game-wasm/reference/icons/draw.ts";
import * as current from "../src/shapes.ts";
import * as draw from "../src/draw.ts";

const clean = <T>(value: T): T => JSON.parse(JSON.stringify(value));
const stock = Object.fromEntries(Object.keys(original.WEAPON_ART).map((weapon) => [weapon,
  Object.fromEntries(["optic", "muzzle", "rail", "mag", "stock"].map((slot) => [slot, `${weapon}-${slot}-stock`])),
])) as Record<current.WeaponArtId, Record<current.Slot, string>>;

// The drawings are redrawn in the catalog; the frozen TypeScript drawing code stays the
// oracle for how they are placed, bounded, masked and composed.
const legacyCompose = (weapon: current.WeaponArtId, parts: Record<current.Slot, string>) => {
  const art = current.WEAPON_ART[weapon];
  const placed = (Object.keys(parts) as current.Slot[]).map((slot) => {
    const part = current.PART_ART[parts[slot]];
    if (!part) throw new Error(`No art for ${parts[slot]}`);
    const [x, y] = art.anchors[slot];
    return legacyDraw.place(part.mount, x, y, part.units === "rifle" && art.units === "pistol" ? 2.5 : 1);
  });
  return legacyDraw.merge(...placed, art.body);
};
const legacyItemArt = (id: string) => {
  if (current.WEAR_ART[id]) return current.WEAR_ART[id];
  const part = current.PART_ART[id];
  if (part) return part.icon ?? (part.mount.body.length ? part.mount : null);
  const weapon = (Object.keys(current.WEAPON_ART) as current.WeaponArtId[]).find((w) => id.startsWith(`${w}-`));
  return weapon ? legacyCompose(weapon, stock[weapon]) : null;
};

test("Rust draws every catalog icon with the original SVG mask geometry", () => {
  assert.deepEqual(Object.keys(current.WEAPON_ART).sort(), Object.keys(original.WEAPON_ART).sort());
  assert.deepEqual(Object.keys(current.PART_ART).sort(), Object.keys(original.PART_ART).sort());
  assert.deepEqual(Object.keys(current.WEAR_ART).sort(), Object.keys(original.WEAR_ART).sort());
  for (const weapon of Object.keys(stock) as current.WeaponArtId[]) {
    assert.deepEqual(current.WEAPON_ART[weapon].anchors, original.WEAPON_ART[weapon].anchors, `${weapon} sockets stay put`);
  }
  const ids = [...Object.keys(current.WEAR_ART), ...Object.keys(current.PART_ART), ...Object.keys(stock).map((w) => `${w}-test`)];
  for (const id of ids) {
    const before = legacyItemArt(id), after = current.itemArt(id, stock);
    assert.deepEqual(after, clean(before), id);
    if (!before || !after) continue;
    assert.deepEqual(draw.bbox(after), legacyDraw.bbox(before), id);
    assert.deepEqual(draw.layers(after), legacyDraw.layers(before), id);
    assert.equal(draw.svg(after, id), legacyDraw.svg(before, id), id);
  }
});

test("Rust attachment placement matches the original across all weapon unit systems", () => {
  for (const weapon of Object.keys(stock) as current.WeaponArtId[]) {
    for (const part of Object.keys(current.PART_ART)) {
      for (const slot of ["optic", "muzzle", "rail", "mag", "stock"] as current.Slot[]) {
        const parts = { ...stock[weapon], [slot]: part };
        assert.deepEqual(current.composeWeapon(weapon, parts), clean(legacyCompose(weapon, parts)), `${weapon}/${slot}/${part}`);
      }
    }
  }
  assert.throws(() => current.composeWeapon("rifle", { ...stock.rifle, optic: "missing" }), /No art for missing/);
});

test("Rust geometry handles scaling, holes, bounds, empty shapes and rounded arcs", () => {
  const shapes = Object.values(original.WEAR_ART);
  for (const shape of shapes) {
    for (const [x, y, scale] of [[0, 0, 1], [-3.25, 7.83, 2.5], [19, -12, 0.4]]) {
      assert.deepEqual(draw.place(shape, x!, y!, scale!), clean(legacyDraw.place(shape, x!, y!, scale!)));
    }
  }
  assert.deepEqual(draw.merge(...shapes), clean(legacyDraw.merge(...shapes)));
  assert.deepEqual(draw.bbox({ body: [] }), [0, 0, 1, 1]);
  for (const steps of [3, 16, 24, 50]) {
    assert.deepEqual(draw.arc(17, -3, 21, 39, -75, 213, steps), legacyDraw.arc(17, -3, 21, 39, -75, 213, steps));
  }
});
