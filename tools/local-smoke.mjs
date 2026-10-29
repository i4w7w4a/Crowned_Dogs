// Run against either local launcher, already running on loopback.
import { request } from 'node:http';
import { readFileSync } from 'node:fs';
import { resolve, join } from 'node:path';
import { createHash } from 'node:crypto';
import assert from 'node:assert/strict';

const folder = resolve(import.meta.dirname, '../release-local/Crowned-Dogs');
function get(path, method = 'GET', headers = {}) {
  return new Promise((resolve, reject) => {
    const req = request({ hostname: '127.0.0.1', port: 5197, path, method, headers }, res => {
      const parts = [];
      res.on('data', chunk => parts.push(chunk));
      res.on('end', () => resolve({ status: res.statusCode, headers: res.headers, body: Buffer.concat(parts) }));
    });
    req.on('error', reject); req.end();
  });
}
const manifest = JSON.parse(readFileSync(join(folder, 'MANIFEST.json'), 'utf8'));
for (const file of manifest.files) {
  const data = readFileSync(join(folder, file.path));
  assert.equal(data.length, file.size, file.path);
  assert.equal(createHash('sha256').update(data).digest('hex'), file.sha256, file.path);
}
const health = await get('/__cd_health');
assert.equal(health.status, 200);
assert.equal(JSON.parse(health.body).engine, 'Rust/WASM');
const index = await get('/');
assert.equal(index.status, 200);
assert.ok(!index.body.toString().includes('fonts.googleapis.com'));
const game = manifest.files.filter(f => f.path.startsWith('game/'));
for (let i = 0; i < game.length; i += 8) {
  await Promise.all(game.slice(i, i + 8).map(async file => {
    const response = await get('/' + file.path.slice(5), 'HEAD');
    assert.equal(response.status, 200, file.path);
    assert.equal(Number(response.headers['content-length']), file.size, file.path);
    assert.equal(response.body.length, 0);
  }));
}
const wasmFile = game.find(f => f.path.endsWith('.wasm'));
assert.ok(wasmFile);
const wasm = await get('/' + wasmFile.path.slice(5));
assert.equal(wasm.headers['content-type'], 'application/wasm');
assert.deepEqual([...wasm.body.subarray(0, 4)], [0, 97, 115, 109]);
const audioFile = game.find(f => f.path.endsWith('.mp3'));
assert.ok(audioFile);
const audioPath = '/' + audioFile.path.slice(5);
const audio = readFileSync(join(folder, audioFile.path));
const partial = await get(audioPath, 'GET', { Range: 'bytes=10-39' });
assert.equal(partial.status, 206); assert.deepEqual(partial.body, audio.subarray(10, 40));
const tail = await get(audioPath, 'GET', { Range: 'bytes=-20' });
assert.equal(tail.status, 206); assert.deepEqual(tail.body, audio.subarray(-20));
assert.equal((await get(audioPath, 'GET', { Range: 'bytes=999999999-' })).status, 416);
for (const path of ['/../Cargo.toml', '/%2e%2e/Cargo.toml', '/a%5c..%5cCargo.toml', '/C:/Windows/win.ini', '/a%00b']) {
  assert.equal((await get(path)).status, 403, path);
}
assert.equal((await get('/', 'GET', { Host: 'untrusted.example:5197' })).status, 403);
assert.equal((await get('/', 'POST')).status, 405);
assert.equal((await get('/does-not-exist')).status, 404);
console.log(`Local package: ${manifest.files.length} file hashes, ${game.length} HTTP assets, WASM MIME, audio seeking and path/Host restrictions passed.`);
