import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { test } from "node:test";

test("import CLI rejects missing explicit targets and reports no path or credentials", () => {
  for (const args of [[], ["--file", "relative-private.json"], ["--database-url", "postgresql://private:secret@remote/private"]]) {
    const result = spawnSync(process.execPath, ["apps/server/src/online/import-cli.ts", ...args], { encoding: "utf8" });
    assert.equal(result.status, 1);
    assert.deepEqual(JSON.parse(result.stderr), { ok: false, categories: { invalid_options: 1 } });
    assert.equal(result.stdout, "");
  }
});
