import assert from "node:assert/strict";
import { test } from "node:test";
import { formatTag, parseTag } from "../src/tag.ts";

test("tags look like K-XXX-XXX", () => {
  assert.equal(formatTag(417093), "K-417-093");
  assert.equal(formatTag(0), "K-000-000");
  assert.equal(formatTag(999999), "K-999-999");
  assert.throws(() => formatTag(1_000_000), RangeError);
  assert.throws(() => formatTag(1.5), RangeError);
});

test("a typed tag is read the way people write it", () => {
  for (const typed of ["K-417-093", "k417093", " К 417 093 ", "к-417093"]) assert.equal(parseTag(typed), "K-417-093", typed);
  for (const bad of ["K-41-7093", "417093", "K-417-0933", "X-417-093", ""]) assert.equal(parseTag(bad), null, bad);
});
