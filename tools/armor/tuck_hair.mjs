// Keeps hair squeezed under a hat inside it, in a game Persona GLB.
//
// usage: node tuck_hair.mjs IN.glb OUT.glb [HATS=Cap,Sfera]
//
// Hair under a hat (<hair>_<Hat>, shown only with that hat) was squeezed when fitted, but strand tips still pierce
// the crown and the back band. Seen from the middle of the skull, the hat's inner wall is measured by direction
// (5 degree cells, its nearest tenth of vertices, the least of each cell's neighbours); every hair vertex in a
// direction the hat covers moves in to 3 mm inside it. Hair below the hat's edge stays as it is.
import { NodeIO } from '@gltf-transform/core';
import { ALL_EXTENSIONS } from '@gltf-transform/extensions';
import { MeshoptDecoder, MeshoptEncoder } from 'meshoptimizer';

const [inPath, outPath, hats = 'Cap,Sfera'] = process.argv.slice(2);
const GAP = 0.003, STEP = Math.PI / 36, AZ = 72, EL = 36;
await MeshoptDecoder.ready;
await MeshoptEncoder.ready;
const io = new NodeIO().registerExtensions(ALL_EXTENSIONS)
  .registerDependencies({ 'meshopt.decoder': MeshoptDecoder, 'meshopt.encoder': MeshoptEncoder });
const doc = await io.read(inPath);
const root = doc.getRoot();
const meshes = root.listNodes().filter((n) => n.getMesh() && n.getSkin());

// 4x4 column-major helpers
const mul = (a, b) => { const o = new Array(16).fill(0); for (let c = 0; c < 4; c++) for (let r = 0; r < 4; r++) for (let k = 0; k < 4; k++) o[c * 4 + r] += a[k * 4 + r] * b[c * 4 + k]; return o; };
const apply = (m, p) => [0, 1, 2].map((r) => m[r] * p[0] + m[4 + r] * p[1] + m[8 + r] * p[2] + m[12 + r]);
function invert(m) {  // affine
  const [a, b, c, d, e, f, g, h, i] = [m[0], m[4], m[8], m[1], m[5], m[9], m[2], m[6], m[10]];
  const det = a * (e * i - f * h) - b * (d * i - f * g) + c * (d * h - e * g);
  const r = [(e * i - f * h), -(b * i - c * h), (b * f - c * e), -(d * i - f * g), (a * i - c * g), -(a * f - c * d), (d * h - e * g), -(a * h - b * g), (a * e - b * d)].map((x) => x / det);
  const o = [r[0], r[3], r[6], 0, r[1], r[4], r[7], 0, r[2], r[5], r[8], 0, 0, 0, 0, 1];
  [0, 1, 2].forEach((k) => { o[12 + k] = -(o[k] * m[12] + o[4 + k] * m[13] + o[8 + k] * m[14]); });
  return o;
}
/** Mesh space to bind-pose world (every joint at rest gives the same joint * inverse-bind). */
const bindMatrix = (n) => { const s = n.getSkin(); return mul(s.listJoints()[0].getWorldMatrix(), s.getInverseBindMatrices().getElement(0, [])); };
const world = (n) => { const m = bindMatrix(n), pos = n.getMesh().listPrimitives()[0].getAttribute('POSITION'), out = []; for (let i = 0, e = []; i < pos.getCount(); i++) out.push(apply(m, pos.getElement(i, e))); return out; };
const head = root.listSkins()[0].listJoints().find((j) => j.getName() === 'head').getWorldTranslation();

for (const hatName of hats.split(',')) {
  const hat = meshes.find((n) => n.getName() === hatName);
  const hair = meshes.filter((n) => n.getName().endsWith(`_${hatName}`));
  if (!hat || !hair.length) continue;
  const hp = world(hat);
  // the middle of the skull: under the crown's top, 8 cm above the head joint
  const top = Math.max(...hp.map((p) => p[1])), crown = hp.filter((p) => p[1] > top - 0.03);
  const C = [crown.reduce((s, p) => s + p[0], 0) / crown.length, head[1] + 0.08, crown.reduce((s, p) => s + p[2], 0) / crown.length];
  const dir = (p) => { const d = [p[0] - C[0], p[1] - C[1], p[2] - C[2]], r = Math.hypot(...d); return { d, r, az: Math.floor((Math.atan2(d[2], d[0]) + Math.PI) / STEP) % AZ, el: Math.floor((Math.asin(d[1] / r) + Math.PI / 2) / STEP) }; };
  const cells = new Map();
  for (const p of hp) { const q = dir(p), k = q.az * EL + q.el; (cells.get(k) ?? cells.set(k, []).get(k)).push(q.r); }
  const wall = new Map([...cells].map(([k, rs]) => { rs.sort((x, y) => x - y); return [k, rs[Math.floor(rs.length * 0.1)]]; }));
  const inner = (az, el) => {  // the least wall around a direction; none where the hat has no wall in every neighbour row
    let best = Infinity, rows = 0;
    for (let de = -1; de <= 1; de++) { let row = false;
      for (let da = -1; da <= 1; da++) { const w = wall.get(((az + da + AZ) % AZ) * EL + el + de); if (w != null) { best = Math.min(best, w); row = true; } }
      if (row) rows++; }
    return rows >= 2 ? best : null;
  };
  for (const n of hair) {
    const m = bindMatrix(n), inv = invert(m), pos = n.getMesh().listPrimitives()[0].getAttribute('POSITION');
    let moved = 0, most = 0;
    for (let i = 0, e = []; i < pos.getCount(); i++) {
      const p = apply(m, pos.getElement(i, e)), q = dir(p), w = inner(q.az, q.el);
      if (w == null || q.r <= w - GAP) continue;
      const k = (w - GAP) / q.r;
      pos.setElement(i, apply(inv, q.d.map((x, j) => C[j] + x * k)));
      moved++; most = Math.max(most, q.r - w + GAP);
    }
    console.log('HAIR', n.getName(), moved, 'of', pos.getCount(), 'verts moved in, at most', (most * 100).toFixed(1), 'cm');
  }
}
await io.write(outPath, doc);
console.log('wrote', outPath);
