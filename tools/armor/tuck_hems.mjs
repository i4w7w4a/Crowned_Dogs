// Tucks the suits' trouser legs into the boots of a game Persona GLB.
//
// usage: node tuck_hems.mjs IN.glb OUT.glb [BAND=0.10] [FLARE=0.3]
//
// The suit legs (Suit and its recolours, which share its positions, and Ryvok) were fitted without boots, so a wide
// hem flares through the back of the shaft and sticks out above its top. Around each lower leg, the boots' surface
// is measured by angle and height about the calf axis; every suit vertex outside it (less 5 mm) moves in towards the
// axis. Above the shaft's top the allowed radius widens FLARE cm per cm up to BAND metres, so the leg blouses over the
// boot instead of creasing. Only positions change: skin, masks and the rest stay as they are.
import { NodeIO } from '@gltf-transform/core';
import { ALL_EXTENSIONS } from '@gltf-transform/extensions';
import { MeshoptDecoder, MeshoptEncoder } from 'meshoptimizer';

const [inPath, outPath, band = '0.10', flare = '0.3'] = process.argv.slice(2);
const BAND = +band, FLARE = +flare, GAP = 0.005, BINS = 36, H = 0.01;
await MeshoptDecoder.ready;
await MeshoptEncoder.ready;
const io = new NodeIO().registerExtensions(ALL_EXTENSIONS)
  .registerDependencies({ 'meshopt.decoder': MeshoptDecoder, 'meshopt.encoder': MeshoptEncoder });
const doc = await io.read(inPath);
const root = doc.getRoot();
const node = (name) => root.listNodes().find((n) => n.getName() === name && n.getMesh());

// 4x4 column-major helpers
const mul = (a, b) => { const o = new Array(16).fill(0); for (let c = 0; c < 4; c++) for (let r = 0; r < 4; r++) for (let k = 0; k < 4; k++) o[c * 4 + r] += a[k * 4 + r] * b[c * 4 + k]; return o; };
const apply = (m, p) => [0, 1, 2].map((r) => m[r] * p[0] + m[4 + r] * p[1] + m[8 + r] * p[2] + m[12 + r]);
function invert(m) {  // affine
  const [a, b, c, d, e, f, g, h, i] = [m[0], m[4], m[8], m[1], m[5], m[9], m[2], m[6], m[10]];
  const det = a * (e * i - f * h) - b * (d * i - f * g) + c * (d * h - e * g);
  const r = [(e * i - f * h), -(b * i - c * h), (b * f - c * e), -(d * i - f * g), (a * i - c * g), -(a * f - c * d), (d * h - e * g), -(a * h - b * g), (a * e - b * d)].map((x) => x / det);
  const t = [m[12], m[13], m[14]];
  const o = [r[0], r[3], r[6], 0, r[1], r[4], r[7], 0, r[2], r[5], r[8], 0, 0, 0, 0, 1];
  [0, 1, 2].forEach((k) => { o[12 + k] = -(o[k] * t[0] + o[4 + k] * t[1] + o[8 + k] * t[2]); });
  return o;
}
/** Mesh space to bind-pose world (every joint at rest gives the same joint * inverse-bind). */
const bindMatrix = (n) => { const s = n.getSkin(); return mul(s.listJoints()[0].getWorldMatrix(), s.getInverseBindMatrices().getElement(0, [])); };
const joint = (name) => root.listSkins()[0].listJoints().find((j) => j.getName() === name).getWorldTranslation();

// the calf axis of each leg, foot to knee, by height
const legs = ['l', 'r'].map((s) => ({ s, foot: joint(`foot_${s}`), knee: joint(`calf_${s}`) }));
const legOf = (p) => legs.reduce((a, b) => (Math.abs(p[0] - a.foot[0]) < Math.abs(p[0] - b.foot[0]) ? a : b));
const axis = (leg, y) => { const t = (y - leg.foot[1]) / (leg.knee[1] - leg.foot[1]); return [0, 2].map((k) => leg.foot[k] + t * (leg.knee[k] - leg.foot[k])); };
const polar = (leg, p) => { const [x, z] = axis(leg, p[1]); const dx = p[0] - x, dz = p[2] - z; return { r: Math.hypot(dx, dz), a: Math.atan2(dz, dx), dx, dz }; };
const bin = (a) => ((Math.floor((a + Math.PI) / (2 * Math.PI) * BINS) % BINS) + BINS) % BINS;

// the boots' radius by angle and height (a low percentile: the shaft's wall, not its straps), and the shaft's top
const boots = node('Boots');
const bm = bindMatrix(boots), bpos = boots.getMesh().listPrimitives()[0].getAttribute('POSITION');
for (const leg of legs) { leg.cells = new Map(); leg.top = new Array(BINS).fill(-Infinity); }
for (let i = 0, e = []; i < bpos.getCount(); i++) {
  const p = apply(bm, bpos.getElement(i, e)), leg = legOf(p), q = polar(leg, p), b = bin(q.a);
  if (p[1] < leg.foot[1] + 0.05) continue;  // the foot itself
  leg.top[b] = Math.max(leg.top[b], p[1]);
  const k = `${b},${Math.floor(p[1] / H)}`;
  (leg.cells.get(k) ?? leg.cells.set(k, []).get(k)).push(q.r);
}
for (const leg of legs) {
  leg.radius = new Map([...leg.cells].filter(([, rs]) => rs.length >= 3).map(([k, rs]) => { rs.sort((x, y) => x - y); return [k, rs[Math.floor(rs.length * 0.25)]]; }));
  // one top for the shaft (notches and the shin guard's peak aside), and its radius there per angle, smoothed round
  const tops = leg.top.filter(Number.isFinite).sort((x, y) => x - y);
  leg.shaft = tops[Math.floor(tops.length / 2)];
  const raw = leg.top.map((_, b) => { for (let h = Math.floor(leg.shaft / H); h > Math.floor(leg.shaft / H) - 8; h--) if (leg.radius.has(`${b},${h}`)) return leg.radius.get(`${b},${h}`); return null; });
  leg.rim = raw.map((_, b) => { const near = [-1, 0, 1].map((d) => raw[(b + d + BINS) % BINS]).filter((r) => r != null).sort((x, y) => x - y); return near.length ? near[Math.floor(near.length / 2)] : null; });
  console.log('leg', leg.s, 'shaft top', leg.shaft.toFixed(3), 'rim', Math.min(...leg.rim.filter(Boolean)).toFixed(3), '..', Math.max(...leg.rim.filter(Boolean)).toFixed(3));
}
/** How far from the axis a suit vertex at angle a and height y may sit, or null where no boot is measured. */
function limit(leg, a, y) {
  const f = (a + Math.PI) / (2 * Math.PI) * BINS - 0.5, b0 = ((Math.floor(f) % BINS) + BINS) % BINS, b1 = (b0 + 1) % BINS, t = f - Math.floor(f);
  const one = (b) => {
    const top = leg.shaft, rim = leg.rim[b];
    if (rim == null || y > top + BAND) return null;
    if (y > top - H) return rim - GAP + Math.max(0, y - top) * FLARE;
    const cell = leg.radius.get(`${b},${Math.floor(y / H)}`);
    return (cell ?? rim) - GAP;
  };
  const l0 = one(b0), l1 = one(b1);
  return l0 == null ? l1 : l1 == null ? l0 : l0 + t * (l1 - l0);
}

for (const name of ['Suit', 'Ryvok']) {
  const n = node(name);
  if (!n) continue;
  const m = bindMatrix(n), inv = invert(m), pos = n.getMesh().listPrimitives()[0].getAttribute('POSITION');
  let moved = 0, most = 0;
  for (let i = 0, e = []; i < pos.getCount(); i++) {
    const p = apply(m, pos.getElement(i, e)), leg = legOf(p);
    if (p[1] > leg.knee[1] - 0.05) continue;
    const q = polar(leg, p), lim = limit(leg, q.a, p[1]);
    if (lim == null || q.r <= lim) continue;
    const k = lim / q.r, [x, z] = axis(leg, p[1]);
    pos.setElement(i, apply(inv, [x + q.dx * k, p[1], z + q.dz * k]));
    moved++; most = Math.max(most, q.r - lim);
  }
  console.log('TUCK', name, moved, 'of', pos.getCount(), 'verts moved in, at most', (most * 100).toFixed(1), 'cm');
}
await io.write(outPath, doc);
console.log('wrote', outPath);
