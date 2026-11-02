import assert from "node:assert/strict";
import { mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "node:test";
import { OperatorRegistry } from "../src/operators.ts";

test("tags go by call-up and signing order, once per player", () => {
  let now = Date.UTC(2026, 10, 2);
  const reg = new OperatorRegistry({ now: () => now });
  const ann = reg.sign("100", "Аня");
  assert.deepEqual(ann, { tag: "K-001-001", telegramId: "100", name: "Аня", signedAt: now });
  assert.equal(reg.sign("100", "Аня Новая"), ann, "signing again keeps the first tag");
  assert.equal(reg.sign("200", "Боб").tag, "K-001-002", "the next signer gets the next number");
  now = Date.UTC(2026, 11, 3);
  assert.equal(reg.sign("400", "Вера").tag, "K-002-001", "December is the second call-up");
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
