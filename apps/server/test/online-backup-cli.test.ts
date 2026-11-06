import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { test } from "node:test";

test("backup CLI requires an explicit operation, target and absolute directory", () => {
  for (const args of [[], ["restore", "--directory", "private"], ["backup", "--database-url", "postgresql://u:secret@remote/data"]]) {
    const result = spawnSync(process.execPath, ["apps/server/src/online/backup-cli.ts", ...args], { encoding: "utf8" });
    assert.equal(result.status, 1);
    assert.deepEqual(JSON.parse(result.stderr), { ok: false, category: "invalid_options" });
    assert.equal(result.stdout, "");
  }
});
