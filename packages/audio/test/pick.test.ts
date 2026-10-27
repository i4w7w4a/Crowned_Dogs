import assert from "node:assert/strict";
import { test } from "node:test";
import { pickOther, Shuffle, VariantPicker } from "../src/pick.ts";
import { seeded } from "./seeded.ts";

test("pickOther never repeats and reaches every option", () => {
  const random = seeded("pick");
  for (const n of [2, 3, 5]) {
    let last: number | undefined;
    const seen = new Set<number>();
    for (let i = 0; i < 500; i++) {
      const v = pickOther(n, last, random);
      assert.ok(v >= 0 && v < n);
      assert.notEqual(v, last);
      seen.add(v);
      last = v;
    }
    assert.equal(seen.size, n);
  }
  assert.equal(pickOther(1, 0, random), 0);
});

test("variants are remembered per sound", () => {
  const picker = new VariantPicker(seeded("variants"));
  const lastOf: Record<string, string> = {};
  for (let i = 0; i < 300; i++) {
    const key = i % 2 ? "a" : "b";
    const v = picker.pick(key, ["1", "2", "3"]);
    assert.notEqual(v, lastOf[key]);
    lastOf[key] = v;
  }
});

test("a playlist never plays the same song twice in a row", () => {
  const list = new Shuffle(["x", "y", "z"], seeded("shuffle"));
  assert.equal(list.now, undefined);
  let last = list.next();
  for (let i = 0; i < 300; i++) {
    const v = list.next();
    assert.notEqual(v, last);
    assert.equal(list.now, v);
    last = v;
  }
  assert.throws(() => new Shuffle([]));
});
