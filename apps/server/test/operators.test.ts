import assert from "node:assert/strict";
import { mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "node:test";
import { OperatorRegistry } from "../src/operators.ts";

test("signing hands out a tag once, and never the same tag twice", () => {
  const picks = [417093, 417093, 5];
  const reg = new OperatorRegistry({ pick: () => picks.shift()!, now: () => 1000 });
  const ann = reg.sign("100", "Аня");
  assert.deepEqual(ann, { tag: "K-417-093", telegramId: "100", name: "Аня", signedAt: 1000 });
  assert.equal(reg.sign("100", "Аня Новая"), ann, "signing again keeps the first tag");
  assert.equal(reg.sign("200", "Боб").tag, "K-000-005", "a taken number is drawn again");
  assert.equal(reg.byTelegram("200")?.name, "Боб");
  assert.equal(reg.byTelegram("300"), undefined);
});

test("tags survive a restart", () => {
  const dir = mkdtempSync(join(tmpdir(), "cd-operators-"));
  try {
    const file = join(dir, "data", "operators.json");
    const first = new OperatorRegistry({ file });
    const tag = first.sign("100", "Аня").tag;
    assert.match(tag, /^K-\d{3}-\d{3}$/);
    assert.equal(JSON.parse(readFileSync(file, "utf8")).length, 1);
    const again = new OperatorRegistry({ file });
    assert.equal(again.byTelegram("100")?.tag, tag);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});
