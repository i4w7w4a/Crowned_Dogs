import test from "node:test";
import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { createMatch } from "@crowned-dogs/core";
import { callRust } from "../../../packages/runtime/src/index.ts";
import type { Frame } from "../src/arena/frames.ts";

// Frame calibration reads this one browser setting at module initialization.
const previous = Object.getOwnPropertyDescriptor(globalThis, "location");
Object.defineProperty(globalThis, "location", { configurable: true, value: { search: "" } });
const { FRAMES } = await import("../src/arena/frames.ts");
if (previous) Object.defineProperty(globalThis, "location", previous);
else Reflect.deleteProperty(globalThis, "location");
const publicFile = (path: string) => fileURLToPath(new URL(`../public/${path}`, import.meta.url));

test("every drawn map has four complete time variants with their own shared calibration and geometry", () => {
  for (const mapId of ["saltHall", "blocks", "dam", "dryBed", "intake", "backwater", "railyard", "sludgePonds"]) {
    const times = FRAMES.filter(frame => frame.mapId === mapId && frame.weather === "clear");
    const day = times.find(frame => frame.time === "day")!;
    assert.deepEqual(times.map(frame => frame.time).sort(), ["day", "evening", "morning", "night"], mapId);
    for (const frame of times) {
      const context = `${mapId}/${frame.time}`;
      assert.deepEqual(frame.cal, day.cal, `${context}: figures must stay registered`);
      assert.deepEqual([frame.w, frame.h, frame.top], [day.w, day.h, day.top], `${context}: framing stays fixed`);
      assert.equal(frame.depth, day.depth, `${context}: changing light must not change occlusion`);
      assert.equal(frame.projectedStage?.manifest, day.projectedStage!.manifest, `${context}: shared geometry`);
      const sources = [frame.src, frame.src2x, frame.depth, frame.projectedStage?.manifest,
        ...Object.values(frame.projectedStage?.textures ?? {}), ...Object.values(frame.projectedStage?.textures2x ?? {})];
      for (const path of sources.filter((path): path is string => !!path)) {
        assert.ok(existsSync(publicFile(path)), `${context}: missing packaged asset ${path}`);
      }
      if (frame.time !== "day") {
        assert.notEqual(frame.src, day.src, `${context}: needs its own relit plate`);
        for (const key of ["clean", "equipment"] as const) {
          const png: Buffer = readFileSync(publicFile(frame.projectedStage!.textures2x![key]!));
          assert.equal(png.toString("hex", 0, 8), "89504e470d0a1a0a", context);
          assert.deepEqual([png.readUInt32BE(16), png.readUInt32BE(20)], [frame.w, frame.h], context);
        }
      }
    }
  }
});

test("real map frames route blocks night to its night plate with the catalogue's field size", () => {
  const chosen = callRust<Frame>("client.conditions.initial", [FRAMES, "night", null, null, false, "blocks", null]);
  assert.equal(chosen.id, "blocks-night");
  assert.equal(chosen.mapId, "blocks");
  assert.equal(chosen.time, "night");
  assert.deepEqual(callRust<{ id: string }[]>("client.conditions.choices", [FRAMES, chosen.mapId]).map(choice => choice.id), ["morning", "day", "evening", "night"]);
  const match = createMatch({ seed: "blocks-real-frame", mapId: chosen.mapId, weather: chosen.weather, time: chosen.time,
    fighters: [{ id: "a", name: "Тень", weapon: "rifle" }, { id: "b", name: "Искра", weapon: "rifle" }] });
  assert.equal(match.map.maxDistance, 12);
  assert.equal(match.distance, 7);
});

test("Kaskad maps route every time to their own plate with the catalogue's field size", () => {
  for (const [mapId, prefix, maxDistance] of [["dam", "dam", 14], ["dryBed", "drybed", 13], ["intake", "intake", 12], ["backwater", "backwater", 13],
    ["railyard", "railyard", 13], ["sludgePonds", "sludgeponds", 14]] as const) {
    for (const time of ["morning", "day", "evening", "night"] as const) {
      const chosen = callRust<Frame>("client.conditions.initial", [FRAMES, time, null, null, false, mapId, null]);
      assert.deepEqual([chosen.id, chosen.mapId, chosen.time], [`${prefix}-${time}`, mapId, time]);
      const match = createMatch({ seed: `${prefix}-real-frame`, mapId, weather: chosen.weather, time,
        fighters: [{ id: "a", name: "Тень", weapon: "rifle" }, { id: "b", name: "Искра", weapon: "rifle" }] });
      assert.equal(match.map.maxDistance, maxDistance);
    }
  }
  const regions = callRust<{ id: string; name: string }[]>("client.conditions.regions", [FRAMES]);
  assert.deepEqual(regions.map(region => region.name), ["Белый Яр", "Каскад"]);
});

test("a synthetic day-only map still falls back without relabelling daytime as night", () => {
  const limited = FRAMES.filter(frame => frame.mapId !== "blocks" || frame.time === "day");
  const chosen = callRust<Frame>("client.conditions.initial", [limited, "night", null, null, false, "blocks", null]);
  assert.deepEqual([chosen.id, chosen.time], ["blocks-day", "day"]);
  assert.equal(callRust("client.conditions.frame", [limited, { mapId: "blocks", time: "night", weather: "clear" }, null]), null);
});
