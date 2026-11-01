// node morph_suit.mjs IN.glb OUT.glb (run from a folder with @gltf-transform/core, extensions and meshoptimizer, like
// the other armour steps): gives each suit (Suit, Ryvok, Solonchak, Ten) a morph target per hard vest ("vest",
// "plita4") that pulls the jacket in under the vest's outside, fading out over a few rings past its edge.
// The game turns it on while that vest is worn, so the suit no longer needs a hole cut where it poked through.
import { NodeIO } from '@gltf-transform/core';
import { ALL_EXTENSIONS } from '@gltf-transform/extensions';
import { MeshoptDecoder, MeshoptEncoder } from 'meshoptimizer';
const io = async () => { await MeshoptDecoder.ready; await MeshoptEncoder.ready;
  return new NodeIO().registerExtensions(ALL_EXTENSIONS).registerDependencies({ 'meshopt.decoder': MeshoptDecoder, 'meshopt.encoder': MeshoptEncoder }); };
const [inp, out] = process.argv.slice(2);
const IO = await io(), doc = await IO.read(inp);
const mul = (a, b) => { const o = new Array(16).fill(0); for (let c = 0; c < 4; c++) for (let r = 0; r < 4; r++) for (let k = 0; k < 4; k++) o[c * 4 + r] += a[k * 4 + r] * b[c * 4 + k]; return o; };
const apply = (m, p, w = 1) => [0, 1, 2].map((r) => m[r] * p[0] + m[4 + r] * p[1] + m[8 + r] * p[2] + m[12 + r] * w);
const sub = (a, b) => [a[0] - b[0], a[1] - b[1], a[2] - b[2]], dot = (a, b) => a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
const cross = (a, b) => [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
const inv3 = (m) => { const [a, b, c, d, e, f, g, h, i] = [m[0], m[4], m[8], m[1], m[5], m[9], m[2], m[6], m[10]];  // rows of the 3x3
  const A = e * i - f * h, B = f * g - d * i, C = d * h - e * g, det = a * A + b * B + c * C;
  return [[A, c * h - b * i, b * f - c * e], [B, a * i - c * g, c * d - a * f], [C, b * g - a * h, a * e - b * d]].map((r) => r.map((x) => x / det)); };
const ARMS = ['upperarm', 'lowerarm', 'hand', 'thumb', 'index', 'middle', 'ring', 'pinky', 'neck', 'head'];  // sleeves and collar move on their own
const rest = (node) => { const skin = node.getSkin(), ibm = skin.getInverseBindMatrices(), joints = skin.listJoints();
  const JP = joints.map((j, i) => mul(j.getWorldMatrix(), ibm.getElement(i, [])));
  const arm = joints.map((j) => ARMS.some((k) => j.getName().toLowerCase().includes(k)));
  const p = node.getMesh().listPrimitives()[0], P = p.getAttribute('POSITION'), N = p.getAttribute('NORMAL'), J = p.getAttribute('JOINTS_0'), W = p.getAttribute('WEIGHTS_0');
  const pos = [], nor = [], back = [], arms = [];
  for (let i = 0; i < P.getCount(); i++) { const x = P.getElement(i, []), n = N.getElement(i, []), j = J.getElement(i, []), w = W.getElement(i, []);
    const M = new Array(16).fill(0); let aw = 0;
    for (let k = 0; k < 4; k++) if (w[k] > 0) { for (let q = 0; q < 16; q++) M[q] += w[k] * JP[j[k]][q]; if (arm[j[k]]) aw += w[k]; }
    const b = apply(M, n, 0), l = Math.hypot(...b) || 1;
    pos.push(apply(M, x)); nor.push(b.map((c) => c / l)); back.push(inv3(M)); arms.push(aw); }
  return { p, pos, nor, back, arms, ix: p.getIndices().getArray() }; };
const nodeOf = (name) => doc.getRoot().listNodes().find((n) => n.getName() === name && n.getMesh());
const front = (name) => { const V = rest(nodeOf(name)), T = [];
  for (let q = 0; q < V.ix.length; q += 3) { const v = [0, 1, 2].map((k) => V.pos[V.ix[q + k]]), n = cross(sub(v[1], v[0]), sub(v[2], v[0])), l = Math.hypot(...n) || 1;
    T.push({ v, n: n.map((x) => x / l), lo: [0, 1, 2].map((k) => Math.min(v[0][k], v[1][k], v[2][k])), hi: [0, 1, 2].map((k) => Math.max(v[0][k], v[1][k], v[2][k])) }); }
  return T; };
// nearest front-facing vest face along o + t d (t in 0..len)
const cast = (T, o, d, len) => { let best = Infinity;
  const end = o.map((c, k) => c + d[k] * len), blo = o.map((c, k) => Math.min(c, end[k])), bhi = o.map((c, k) => Math.max(c, end[k]));
  for (const { v, n, lo, hi } of T) { if (lo[0] > bhi[0] || hi[0] < blo[0] || lo[1] > bhi[1] || hi[1] < blo[1] || lo[2] > bhi[2] || hi[2] < blo[2]) continue;
    if (dot(n, d) > -0.2) continue;  // the vest's outside faces back along the ray
    const e1 = sub(v[1], v[0]), e2 = sub(v[2], v[0]), h = cross(d, e2), a = dot(e1, h); if (Math.abs(a) < 1e-12) continue;
    const f = 1 / a, s = sub(o, v[0]), u = f * dot(s, h); if (u < 0 || u > 1) continue;
    const q = cross(s, e1), w = f * dot(d, q); if (w < 0 || u + w > 1) continue; const t = f * dot(e2, q); if (t > 0 && t < best) best = t; }
  return best; };
const VESTS = [['vest', 'Vest'], ['plita4', 'Plita4']].filter(([, m]) => nodeOf(m)).map(([k, m]) => [k, front(m)]);
const UP = 0.1, IN = 0.01, RINGS = [0.75, 0.5, 0.25];
const done = new Map();
for (const name of ['Suit', 'Ryvok', 'Solonchak', 'Ten']) {
  const node = nodeOf(name); if (!node) continue;
  const prim = node.getMesh().listPrimitives()[0], key = prim.getAttribute('POSITION');
  if (!done.has(key)) {
    const S = rest(node), n = S.pos.length, weld = new Int32Array(n), seen = new Map();
    for (let i = 0; i < n; i++) { const k = S.pos[i].map((x) => Math.round(x * 1e4)).join(); if (!seen.has(k)) seen.set(k, i); weld[i] = seen.get(k); }
    const nb = Array.from({ length: n }, () => new Set());
    for (let q = 0; q < S.ix.length; q += 3) for (const a of [0, 1, 2]) for (const b of [0, 1, 2]) if (a !== b) nb[weld[S.ix[q + a]]].add(weld[S.ix[q + b]]);
    // decimated suits have flipped normals here and there: average over neighbours, then point away from the spine
    let sn = S.nor.map((x) => x.slice());
    for (let it = 0; it < 3; it++) sn = sn.map((x, i) => { if (weld[i] !== i) return x; const a = x.slice(); for (const j of nb[i]) for (let c = 0; c < 3; c++) a[c] += sn[j][c]; return a; });
    const ax = [0, 2].map((k) => S.pos.reduce((s, p) => s + p[k], 0) / n);
    sn = sn.map((x, i) => { const r = [S.pos[i][0] - ax[0], 0, S.pos[i][2] - ax[1]], l = Math.hypot(...x) || 1, v = x.map((c) => c / l);
      return dot(v, r) < 0 ? v.map((c) => -c) : v; });
    const targets = [];
    for (const [tname, T] of VESTS) {
      const depth = new Float64Array(n);  // how far in along -normal, per welded vertex
      for (let i = 0; i < n; i++) { if (weld[i] !== i || S.arms[i] >= 0.3) continue;
        const p = S.pos[i], d = sn[i].map((c) => -c), t = cast(T, p.map((c, k) => c - d[k] * UP), d, UP + 0.06);
        if (t === Infinity) continue; const out = UP - t;  // suit above the vest's outside by this much (negative: below)
        if (out > -IN && out < 0.08) depth[i] = out + IN; }
      // points the old cut hid but no ray settled (gaps between pouches, folds): take their neighbours' depth
      const cut = prim.getAttribute('_' + tname.toUpperCase());
      for (let it = 0; it < 6 && cut; it++) for (let i = 0; i < n; i++) if (weld[i] === i && depth[i] === 0 && cut.getScalar(i) >= 0.5) {
        let m = 0; for (const j of nb[i]) m = Math.max(m, depth[j]); depth[i] = m; }
      // fade past the vest's edge so the jacket tucks in instead of stepping
      let ring = [...depth].map((x, i) => (x > 0 ? i : -1)).filter((i) => i >= 0);
      for (const f of RINGS) { const next = [];
        for (const i of ring) for (const j of nb[i]) if (depth[j] === 0 && S.arms[j] < 0.3) { depth[j] = -1; next.push(j); }
        for (const j of next) { let m = 0; for (const k of nb[j]) if (depth[k] > 0) m = Math.max(m, depth[k]); depth[j] = m * f; }
        ring = next; }
      for (let it = 0; it < 2; it++) { const sm = depth.slice(); for (let i = 0; i < n; i++) if (weld[i] === i && depth[i] > 0) { let s = depth[i], c = 1; for (const j of nb[i]) { s += depth[j]; c++; } sm[i] = Math.max(depth[i] * 0.5, s / c); } depth.set(sm); }
      const arr = new Float32Array(n * 3); let moved = 0, max = 0;
      for (let i = 0; i < n; i++) { const dd = depth[weld[i]]; if (dd <= 0) continue; moved++; max = Math.max(max, dd);
        const w = sn[weld[i]].map((c) => -c * dd), B = S.back[i], l = [0, 1, 2].map((r) => B[r][0] * w[0] + B[r][1] * w[1] + B[r][2] * w[2]);
        arr.set(l, i * 3); }
      console.log(name, tname, 'moved', moved, 'of', n, 'max', max.toFixed(3));
      const acc = doc.createAccessor(`${name}_${tname}`).setType('VEC3').setArray(arr).setBuffer(key.getBuffer());
      targets.push([tname, doc.createPrimitiveTarget(tname).setAttribute('POSITION', acc)]);
    }
    done.set(key, targets);
  }
  const targets = done.get(key);
  for (const [, t] of targets) prim.addTarget(t);
  node.getMesh().setWeights(targets.map(() => 0)).setExtras({ ...node.getMesh().getExtras(), targetNames: targets.map(([k]) => k) });
}
await IO.write(out, doc);
