// Read-only geometry/export checks. Writes only its JSON evidence report.
import assert from 'node:assert/strict';
import { existsSync, readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import { resolve, join, dirname } from 'node:path';
import { createHash } from 'node:crypto';
import { Matrix4, Quaternion, Vector3, Box3 } from 'three';
import { MeshoptDecoder } from 'three/addons/libs/meshopt_decoder.module.js';

const repo = resolve(import.meta.dirname, '../..');
const models = join(repo, 'apps/web/models');
const output = process.argv[2] ? resolve(process.argv[2]) : join(repo, 'output/beards/assets-validation.json');
await MeshoptDecoder.ready;
const widths = { SCALAR: 1, VEC2: 2, VEC3: 3, VEC4: 4, MAT4: 16 };
const types = { 5120: [1, 'getInt8', 127], 5121: [1, 'getUint8', 255], 5122: [2, 'getInt16', 32767], 5123: [2, 'getUint16', 65535], 5125: [4, 'getUint32', 4294967295], 5126: [4, 'getFloat32', 1] };
function open(name, file = join(models, `${name}.glb`)) {
  const content = readFileSync(file);
  const bytes = file.endsWith('.json') ? Buffer.from(JSON.parse(content).data, 'base64') : content;
  assert.equal(bytes.readUInt32LE(0), 0x46546c67);
  const jsonLength = bytes.readUInt32LE(12);
  const data = JSON.parse(bytes.subarray(20, 20 + jsonLength).toString());
  const binOffset = 20 + jsonLength + 8;
  const bin = bytes.subarray(binOffset);
  const views = new Map(), arrays = new Map(), worlds = new Map(), parents = new Map();
  data.nodes.forEach((n, i) => n.children?.forEach(child => parents.set(child, i)));
  function view(i) {
    if (views.has(i)) return views.get(i);
    const v = data.bufferViews[i], ext = v.extensions?.EXT_meshopt_compression;
    let out;
    if (ext) {
      assert.equal(ext.buffer, 0, 'embedded compression buffer');
      out = new Uint8Array(ext.count * ext.byteStride);
      MeshoptDecoder.decodeGltfBuffer(out, ext.count, ext.byteStride, bin.subarray(ext.byteOffset || 0, (ext.byteOffset || 0) + ext.byteLength), ext.mode, ext.filter);
    } else {
      assert.equal(v.buffer, 0, 'embedded uncompressed buffer');
      out = bin.subarray(v.byteOffset || 0, (v.byteOffset || 0) + v.byteLength);
    }
    views.set(i, out); return out;
  }
  function accessor(i) {
    if (arrays.has(i)) return arrays.get(i);
    const a = data.accessors[i];
    assert.ok(!a.sparse, 'sparse attributes need an explicit decoder');
    const [size, method, divisor] = types[a.componentType];
    const width = widths[a.type], source = view(a.bufferView), dv = new DataView(source.buffer, source.byteOffset, source.byteLength);
    const stride = data.bufferViews[a.bufferView].byteStride || width * size;
    const values = Array.from({ length: a.count }, (_, index) => Array.from({ length: width }, (_, k) => {
      const value = dv[method]((a.byteOffset || 0) + index * stride + k * size, true);
      assert.ok(Number.isFinite(value), `${name} accessor ${i} contains non-finite value`);
      return a.normalized ? Math.max(-1, value / divisor) : value;
    }));
    arrays.set(i, values); return values;
  }
  function world(i) {
    if (worlds.has(i)) return worlds.get(i);
    const n = data.nodes[i];
    const matrix = n.matrix ? new Matrix4().fromArray(n.matrix) : new Matrix4().compose(new Vector3(...(n.translation || [0, 0, 0])), new Quaternion(...(n.rotation || [0, 0, 0, 1])), new Vector3(...(n.scale || [1, 1, 1])));
    if (parents.has(i)) matrix.premultiply(world(parents.get(i)));
    assert.ok(matrix.elements.every(Number.isFinite));
    worlds.set(i, matrix); return matrix;
  }
  function points(i, replacements = new Map()) {
    const node = data.nodes[i];
    const skin = node.skin === undefined ? null : data.skins[node.skin];
    const matrices = skin ? accessor(skin.inverseBindMatrices).map((v, j) => world(skin.joints[j]).clone().multiply(new Matrix4().fromArray(v))) : null;
    return data.meshes[node.mesh].primitives.flatMap(p => {
      const pos = replacements.get(p.attributes.POSITION) ?? accessor(p.attributes.POSITION);
      if (!skin) return pos.map(v => new Vector3(...v).applyMatrix4(world(i)));
      const joints = accessor(p.attributes.JOINTS_0), weights = accessor(p.attributes.WEIGHTS_0);
      return pos.map((v, k) => {
        const out = new Vector3();
        for (let j = 0; j < 4; j++) if (weights[k][j]) out.addScaledVector(new Vector3(...v).applyMatrix4(matrices[joints[k][j]]), weights[k][j]);
        return out;
      });
    });
  }
  return { name, data, accessor, world, points, sha256: createHash('sha256').update(bytes).digest('hex') };
}
const game = open('persona-male'), male = open('beards-male'), authored = open('beards-authored');
// The clean head companion is switched off in the game (Art, 2026-11-01); check it only when present.
const skin = existsSync(join(models, 'beards-skin.glb')) ? open('beards-skin') : null;
const authoredGame = open('persona-authored', join(repo, 'apps/web/public/preview/persona-authored.json'));
const headIndex = game.data.nodes.findIndex(n => n.name === 'head');
assert.ok(headIndex >= 0, 'current game head bone exists');
const head = game.world(headIndex);
const scale = .9630493809698595;
const fit = new Matrix4().makeScale(scale, scale, scale).setPosition(-.00429309, .17305149, .00425317);
const box = points => new Box3().setFromPoints(points);
const plainBox = b => ({ min: b.min.toArray(), max: b.max.toArray() });
const report = { sources: Object.fromEntries([game, authoredGame, male, authored, skin].filter(Boolean).map(p => [p.name, p.sha256])), headRestMatrix: head.toArray(), assertions: [], repairs: [], fits: [] };
const expectedSkinNodes = ['CleanHead_m2', 'CleanHead_m4', 'CleanNormals_m2', 'CleanNormals_m4', 'CleanNormalsAuthored_m2', 'CleanNormalsAuthored_m4'];
const repairedGame = new Map();
if (skin) {
assert.deepEqual(skin.data.nodes.filter(n => n.mesh !== undefined).map(n => n.name).sort(), [...expectedSkinNodes].sort());
assert.equal(skin.data.skins?.length || 0, 0, 'skin companions are data only');
for (const source of [game, authoredGame]) for (const id of ['m2', 'm4']) {
  const name = `${source === game ? 'CleanNormals' : 'CleanNormalsAuthored'}_${id}`;
  const carrierIndex = skin.data.nodes.findIndex(n => n.name === name), carrier = skin.data.nodes[carrierIndex];
  const sourceIndex = source.data.nodes.findIndex(n => n.name === `Head_${id}`), sourceNode = source.data.nodes[sourceIndex];
  assert.ok(sourceNode, `${source.name}: ${id} exists`);
  const carrierPrimitives = skin.data.meshes[carrier.mesh].primitives;
  const sourcePrimitives = source.data.meshes[sourceNode.mesh].primitives;
  assert.equal(carrierPrimitives.length, 1); assert.equal(sourcePrimitives.length, 1);
  const attrs = carrierPrimitives[0].attributes;
  assert.equal(skin.data.accessors[attrs.POSITION].componentType, 5126, `${name}: original position keys stay float32`);
  assert.ok(attrs._CLEAN_POSITION !== undefined, `${name}: repaired position companion exists`);
  assert.equal(skin.data.accessors[attrs._CLEAN_POSITION].componentType, 5126);
  const carry = skin.world(carrierIndex);
  const keys = skin.accessor(attrs.POSITION).map(p => new Vector3(...p).applyMatrix4(carry).toArray());
  const normals = skin.accessor(attrs.NORMAL);
  const targets = skin.accessor(attrs._CLEAN_POSITION).map(p => new Vector3(...p).applyMatrix4(carry).toArray());
  assert.equal(keys.length, normals.length); assert.equal(keys.length, targets.length);
  // Measured Blender float drift: 0.746µm; approved 2µm matching tolerance.
  const epsilon = .000002, maxRepair = .002, key = p => p.map(v => Math.floor(v / epsilon));
  const buckets = new Map();
  let maxDeclaredDeltaM = 0;
  keys.forEach((p, i) => {
    const length = Math.hypot(...normals[i]); assert.ok(length > 1e-6, `${name}: usable normal`);
    const delta = Math.hypot(...p.map((v, k) => v - targets[i][k]));
    assert.ok(delta <= maxRepair, `${name}: repair ${delta}m exceeds 2mm`);
    maxDeclaredDeltaM = Math.max(maxDeclaredDeltaM, delta);
    const bucket = key(p).join(','); if (!buckets.has(bucket)) buckets.set(bucket, []); buckets.get(bucket).push(i);
  });
  let maxMatchErrorM = 0, maxAppliedDeltaM = 0;
  const repaired = source.points(sourceIndex).map(physical => {
    const p = physical.toArray();
    const base = key(p), matches = [];
    for (let x = -1; x <= 1; x++) for (let y = -1; y <= 1; y++) for (let z = -1; z <= 1; z++) {
      for (const i of buckets.get([base[0] + x, base[1] + y, base[2] + z].join(',')) ?? []) {
        const error = Math.hypot(...p.map((v, k) => v - keys[i][k]));
        if (error <= epsilon) { matches.push(i); maxMatchErrorM = Math.max(maxMatchErrorM, error); }
      }
    }
    assert.ok(matches.length, `${name}: original vertex ${p} missing within 2µm`);
    const target = [0, 1, 2].map(k => matches.reduce((s, i) => s + targets[i][k], 0) / matches.length);
    const normal = [0, 1, 2].map(k => matches.reduce((s, i) => s + normals[i][k] / Math.hypot(...normals[i]), 0));
    assert.ok(Math.hypot(...normal) > 1e-6, `${name}: coincident normal samples do not cancel`);
    const delta = Math.hypot(...p.map((v, k) => v - target[k]));
    assert.ok(delta <= maxRepair, `${name}: applied repair exceeds 2mm`);
    maxAppliedDeltaM = Math.max(maxAppliedDeltaM, delta);
    return target;
  });
  if (source === game) repairedGame.set(`Head_${id}`, repaired.map(p => new Vector3(...p)));
  report.repairs.push({ source: source.name, head: id, vertices: repaired.length, maxMatchErrorM, maxDeclaredDeltaM, maxAppliedDeltaM });
}
}
for (const pack of [male, authored]) {
  assert.equal(pack.data.skins?.length || 0, 0, `${pack.name} must use the existing head transform`);
  assert.equal(pack.data.animations?.length || 0, 0);
  assert.ok(!pack.data.nodes.some(n => n.name === 'head' || n.name === 'Armature'));
  for (let i = 0; i < pack.data.accessors.length; i++) pack.accessor(i);
  for (let i = 0; i < pack.data.nodes.length; i++) pack.world(i);
  for (const material of pack.data.materials) {
    if (material.name.startsWith('M_Beard_Cards')) {
      assert.equal(material.alphaMode, 'MASK', `${pack.name}: fibre cards must export MASK`);
      assert.ok(material.alphaCutoff > 0 && material.alphaCutoff < 1);
      assert.ok(material.pbrMetallicRoughness.baseColorTexture);
    } else if (material.name.startsWith('M_Beard_Root')) assert.equal(material.alphaMode, 'BLEND');
    else assert.fail(`Unexpected beard material ${material.name}`);
    assert.equal(material.doubleSided, true);
  }
}
for (let face = 1; face <= 4; face++) {
  const index = game.data.nodes.findIndex(n => n.name === `Head_m${face}`);
  assert.ok(index >= 0);
  const headBounds = box(repairedGame.get(`Head_m${face}`) ?? game.points(index));
  for (const style of ['boxed', 'goatee', 'full', 'tapered']) {
    const prefix = `Beard_m${face}_${style}_`;
    const selected = male.data.nodes.map((n, i) => ({ n, i })).filter(({ n }) => n.mesh !== undefined && n.name?.startsWith(prefix));
    assert.ok(selected.length >= 2, `${prefix} needs roots and cards`);
    const all = [], paired = [];
    for (const { n, i } of selected) {
      const other = authored.data.nodes.findIndex(o => o.name === n.name && o.mesh !== undefined);
      assert.ok(other >= 0, `${n.name}: matching authored export`);
      const a = male.points(i).map(p => p.applyMatrix4(head)), b = authored.points(other);
      assert.equal(a.length, b.length, `${n.name}: vertex counts`);
      let maxError = 0;
      for (let j = 0; j < a.length; j++) maxError = Math.max(maxError, a[j].clone().applyMatrix4(fit).distanceTo(b[j]));
      assert.ok(maxError < .00025, `${n.name}: rest→authored discrepancy ${maxError}m`);
      all.push(...a); paired.push({ name: n.name, vertices: a.length, maxAuthoredErrorM: maxError });
    }
    const bounds = box(all), allowed = headBounds.clone().expandByScalar(.045);
    assert.ok(allowed.containsBox(bounds), `${prefix}: beard lies outside head bounds +45mm (transform/placement error)`);
    report.fits.push({ face: `m${face}`, style, headBounds: plainBox(headBounds), beardBounds: plainBox(bounds), parts: paired });
  }
}
report.assertions = ['16 face/style fits with roots and cards in both packs', 'No independent skins, skeleton or animations', 'Finite accessors and node transforms', 'Cards MASK / roots BLEND / double-sided materials', 'Animated pack stays in repaired head bounds +45mm at rest', 'Authored export matches FIT × current-head-rest × animated export within 0.25mm', ...(skin ? ['All four normal/position companions match actual animated and authored heads within 2µm (measured Blender drift <0.75µm)', 'Applied scan seam position repairs stay within 2mm and preserve source topology'] : [])];
mkdirSync(dirname(output), { recursive: true });
writeFileSync(output, JSON.stringify(report, null, 2) + '\n');
console.log(`PASS: ${report.fits.length} fits, 2 packs; ${output}`);
