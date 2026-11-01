import test from "node:test";
import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

// Frame calibration reads this one browser setting at module initialization.
const previous = Object.getOwnPropertyDescriptor(globalThis, "location");
Object.defineProperty(globalThis, "location", { configurable: true, value: { search: "" } });
const { FRAMES } = await import("../src/arena/frames.ts");
if (previous) Object.defineProperty(globalThis, "location", previous);
else Reflect.deleteProperty(globalThis, "location");
const publicFile = (path: string) => fileURLToPath(new URL(`../public/${path}`, import.meta.url));

test("all four battle times have complete plates and preserve the shared spatial calibration", () => {
  const day = FRAMES.find(frame => frame.id === "hall1")!;
  const times = FRAMES.filter(frame => frame.mapId === "saltHall" && frame.weather === "clear");
  assert.deepEqual(times.map(frame => frame.time).sort(), ["day", "evening", "morning", "night"]);
  for (const frame of times) {
    assert.deepEqual(frame.cal, day.cal, `${frame.time}: live figures must stay registered`);
    assert.equal(frame.depth, day.depth, `${frame.time}: changing light must not change occlusion`);
    assert.equal(frame.projectedStage?.manifest, day.projectedStage!.manifest);
    const sources = [frame.src, frame.src2x, frame.depth, frame.projectedStage?.manifest,
      ...Object.values(frame.projectedStage?.textures ?? {}), ...Object.values(frame.projectedStage?.textures2x ?? {})];
    for (const path of sources.filter((path): path is string => !!path)) {
      assert.ok(existsSync(publicFile(path)), `${frame.time}: missing packaged asset ${path}`);
    }
    if (frame.time !== "day") {
      for (const key of ["clean", "equipment"] as const) {
        const png: Buffer = readFileSync(publicFile(frame.projectedStage!.textures2x![key]!));
        assert.equal(png.toString("hex", 0, 8), "89504e470d0a1a0a");
        assert.equal(png.readUInt32BE(16), frame.w);
        assert.equal(png.readUInt32BE(20), frame.h);
      }
    }
  }
});
