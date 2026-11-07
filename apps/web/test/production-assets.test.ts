import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { test } from "node:test";

const script = fileURLToPath(new URL("../scripts/verify-production-assets.ts", import.meta.url));

function fixture(t: { after: (run: () => void) => void }) {
  const root = mkdtempSync(join(tmpdir(), "crowned-dogs-production-assets-"));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const dist = join(root, "dist");
  mkdirSync(join(dist, "models"), { recursive: true });
  writeFileSync(join(dist, "models", "sample.glb"), Buffer.from("glTF-sample"));
  return dist;
}

function verify(dist: string) {
  return spawnSync(process.execPath, [script, "--dist", dist], { encoding: "utf8" });
}

test("Production asset verification rejects a missing KTX2 model", (t) => {
  const result = verify(fixture(t));
  assert.notEqual(result.status, 0, "a missing compressed model must fail the build");
  assert.match(result.stderr, /sample\.k\.glb/);
  assert.match(result.stderr, /models:ktx2/);
});

test("Production asset verification accepts a complete KTX2 model set", (t) => {
  const dist = fixture(t);
  writeFileSync(join(dist, "models", "sample.k.glb"), Buffer.from("glTF-compressed"));
  const result = verify(dist);
  assert.equal(result.status, 0, result.stderr);
});
