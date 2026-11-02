import assert from "node:assert/strict";
import { readFileSync, existsSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { test } from "node:test";

type Part = { name: string; material: string; positions: number[]; indices: number[]; occludesFigures: boolean };
type Manifest = {
  version: number; calibration: { fov: number; aspect: number; camHeight: number; horizon: number };
  textures: Record<string, string>; textures2x: Record<string, string>; meshes: Part[];
  repairPolygons: number[][][]; reconstruction: { id: string; sourceSize: number[]; factoryMeshes: boolean };
};
const load = (map: string) => {
  const source = fileURLToPath(new URL(`../public/img/${map}-stage/manifest.json`, import.meta.url));
  return { source, manifest: JSON.parse(readFileSync(source, "utf8")) as Manifest };
};

for (const [map, horizon, props] of [["dam", 0.462, ["APC", "AA gun", "Near blocks"]], ["drybed", 0.365, ["Tanker barge", "River tug", "Near concrete ruin"]],
  ["intake", 0.40, ["Collapse rubble", "Curb debris"]], ["backwater", 0.395, ["Ship hull", "Keel blocks"]],
  ["railyard", 0.41, ["Diesel locomotive", "Wheelsets", "Workbench"]], ["sludgeponds", 0.381, ["Far excavator"]]] as const) {
  test(`${map}: its own stage resolves every material and keeps the measured horizon`, () => {
    const { source, manifest } = load(map);
    assert.equal(manifest.version, 1); assert.equal(manifest.reconstruction.id, `${map}-day`);
    assert.equal(manifest.reconstruction.factoryMeshes, false);
    assert.deepEqual(manifest.reconstruction.sourceSize, [941, 1672]);
    assert.equal(manifest.calibration.fov, 55); assert.equal(manifest.calibration.camHeight, 2);
    assert.ok(Math.abs(manifest.calibration.aspect - 941 / 1672) < 1e-10);
    assert.equal(manifest.calibration.horizon, horizon);
    for (const path of [...Object.values(manifest.textures), ...Object.values(manifest.textures2x)]) {
      assert.ok(existsSync(resolve(dirname(source), path)), `missing ${map} material ${path}`);
      assert.ok(resolve(dirname(source), path).replaceAll("\\", "/").includes(`/${map}-`), `${map} must not borrow another map's plates: ${path}`);
    }
    for (const part of manifest.meshes) {
      assert.ok(part.positions.length >= 9 && part.positions.length % 3 === 0, part.name);
      assert.ok(part.positions.every(Number.isFinite), part.name);
      assert.ok(part.indices.length >= 3 && part.indices.length % 3 === 0, part.name);
      assert.ok(part.indices.every(index => Number.isInteger(index) && index >= 0 && index < part.positions.length / 3), part.name);
    }
    assert.ok(manifest.repairPolygons.length > 0 && manifest.repairPolygons.length <= 128);
    assert.ok(manifest.repairPolygons.every(polygon => polygon.length >= 3 && polygon.every(point => point.length === 2 && point.every(value => value >= 0 && value <= 1))));
  });

  test(`${map}: ground stays walkable and removed props have real depth`, () => {
    const { manifest } = load(map);
    const ground = manifest.meshes.find(part => part.material === "floor")!;
    assert.ok(ground); assert.equal(ground.occludesFigures, false);
    assert.ok(ground.positions.filter((_, i) => i % 3 === 1).every(y => y === 0));
    for (const name of props) {
      const part = manifest.meshes.find(p => p.name.startsWith(name) && p.material === "equipment");
      assert.ok(part, `${map}: ${name} must be its own volume`);
      assert.equal(part.occludesFigures, true);
      const depths = part.positions.filter((_, i) => i % 3 === 2);
      assert.ok(Math.max(...depths) - Math.min(...depths) >= 0.8, `${name} needs depth, not a cutout`);
    }
    // Nothing that hides a fighter (shoulders about 0.5 m wide) may stand in the combat lane between 6 and 18 metres.
    for (const part of manifest.meshes.filter(p => p.occludesFigures)) {
      for (let i = 0; i < part.positions.length; i += 3) {
        const [x, y, z] = part.positions.slice(i, i + 3) as [number, number, number];
        assert.ok(!(Math.abs(x) < 0.6 && y > 0 && y < 2.2 && z < -6 && z > -18), `${map}: ${part.name} blocks the lane at ${x},${y},${z}`);
      }
    }
  });
}
