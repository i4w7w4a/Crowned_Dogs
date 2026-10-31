// Record provenance after an intentional bake; otherwise verify existing bytes.
import { readFileSync, writeFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { resolve } from 'node:path';
import assert from 'node:assert/strict';
const root = resolve(import.meta.dirname, '../..');
const relative = 'apps/web/public/tex/relief/';
const manifest = JSON.parse(readFileSync(resolve(root, relative, 'manifest.json'), 'utf8'));
const paths = new Set([relative + 'manifest.json']);
for (const model of Object.values(manifest.models)) {
  paths.add('apps/web/public/' + model.source);
  for (const material of Object.values(model.materials)) {
    for (const key of ['height', 'height16', 'mask', 'relief']) {
      const path = 'apps/web/public/' + material[key]; paths.add(path);
      const bytes = readFileSync(resolve(root, path));
      assert.equal(bytes.toString('hex', 0, 8), '89504e470d0a1a0a');
      assert.equal(bytes.readUInt32BE(16), material.resolution[0]);
      assert.equal(bytes.readUInt32BE(20), material.resolution[1]);
      assert.equal(bytes[24], key === 'height16' ? 16 : 8);
    }
  }
}
const hashes = Object.fromEntries([...paths].sort().map(path => [path, createHash('sha256').update(readFileSync(resolve(root, path))).digest('hex')]));
const record = resolve(root, 'tools/height-baker/checksums.json');
if (process.argv.includes('--record')) writeFileSync(record, JSON.stringify(hashes, null, 2) + '\n');
else assert.deepEqual(hashes, JSON.parse(readFileSync(record, 'utf8')));
const sets = new Set(Object.values(manifest.models).flatMap(model => Object.values(model.materials).map(m => m.relief))).size;
console.log(`${paths.size} sources/maps/manifest verified; ${sets} unique material sets, dimensions and PNG bit depth match.`);
