// Joins armour batch 2 (dress_gear.py's export) into a game Persona GLB.
//
// usage: node merge_gear.mjs GAME.glb GEAR.glb OUT.glb [SUIT_RECOLOURS_DIR] [ONLY=Name,Name]
//
// - Each piece of GEAR.glb joins GAME bound to GAME's own joints (matched by name), keeping its own inverse bind
//   matrices; a GAME mesh of the same name (the old full-length Suit) goes. Both files must share the rest pose.
// - GEAR's body brings its masks: every `_name` attribute is copied onto GAME's body by nearest bind position.
// - SUIT_RECOLOURS_DIR/M_<Name>.color.1024.webp makes a <Name> node per file: the Suit's geometry and skin with
//   a copy of its material wearing that colour map (Solonchak, Ten).
// - Hair under a hat (FemHair*_Cap, _Sfera) gets the offset GAME's free hair has against GEAR's (the hair was
//   moved in the game model after the fitting).
// - ONLY=Plita4: only those pieces replace the game's; every other GEAR piece just lends its `_name` attributes to
//   the game mesh of its name (and the Suit's to its recolours), so later fixes in the game model stay.
// Then meshopt as the game models have it.
import fs from 'node:fs';
import path from 'node:path';
import { NodeIO } from '@gltf-transform/core';
import { ALL_EXTENSIONS } from '@gltf-transform/extensions';
import { mergeDocuments, meshopt, prune, unpartition } from '@gltf-transform/functions';
import { MeshoptDecoder, MeshoptEncoder } from 'meshoptimizer';

const args = process.argv.slice(2);
const only = args.find((a) => a.startsWith('ONLY='))?.slice(5).split(',');
const [gamePath, gearPath, outPath, recolours] = args.filter((a) => !a.startsWith('ONLY='));
await MeshoptDecoder.ready;
await MeshoptEncoder.ready;
const io = new NodeIO().registerExtensions(ALL_EXTENSIONS)
  .registerDependencies({ 'meshopt.decoder': MeshoptDecoder, 'meshopt.encoder': MeshoptEncoder });
const doc = await io.read(gamePath);
const gear = await io.read(gearPath);
const root = doc.getRoot();
const scene = root.listScenes()[0];
const skin = root.listSkins()[0];
const joints = new Map(skin.listJoints().map((j, i) => [j.getName(), i]));
const named = (name) => root.listNodes().find((n) => n.getName() === name && n.getMesh());
const BODIES = ['MaleBody', 'FemBody'];
const gameBody = BODIES.map(named).find(Boolean);
if (!gameBody) throw new Error('no body in the game model');
// pieces wearing a material the game already has (hair, the suit) take the game's: its tuned blending and tiers
const own = new Map(root.listMaterials().map((m) => [m.getName(), m]));

// 4x4 column-major helpers
const mul = (a, b) => { const o = new Array(16).fill(0); for (let c = 0; c < 4; c++) for (let r = 0; r < 4; r++) for (let k = 0; k < 4; k++) o[c * 4 + r] += a[k * 4 + r] * b[c * 4 + k]; return o; };
const apply = (m, p) => [0, 1, 2].map((r) => m[r] * p[0] + m[4 + r] * p[1] + m[8 + r] * p[2] + m[12 + r]);
/** Bind-pose world positions of a skinned node (every joint at rest gives the same joint * inverse-bind). */
function bindPositions(node) {
  const s = node.getSkin();
  const ibm = s.getInverseBindMatrices();
  const m = mul(s.listJoints()[0].getWorldMatrix(), ibm.getElement(0, []));
  const pos = node.getMesh().listPrimitives()[0].getAttribute('POSITION');
  const out = new Float64Array(pos.getCount() * 3), e = [];
  for (let i = 0; i < pos.getCount(); i++) out.set(apply(m, pos.getElement(i, e)), i * 3);
  return out;
}
/** For each point of `to`, the index of the nearest point of `from` (grid hash, 1 cm cells). */
function nearest(from, to) {
  const cell = 0.01, grid = new Map(), key = (x, y, z) => `${x},${y},${z}`;
  for (let i = 0; i < from.length / 3; i++) {
    const k = key(...[0, 1, 2].map((a) => Math.floor(from[i * 3 + a] / cell)));
    (grid.get(k) ?? grid.set(k, []).get(k)).push(i);
  }
  const out = new Uint32Array(to.length / 3);
  let far = 0;
  for (let i = 0; i < out.length; i++) {
    const c = [0, 1, 2].map((a) => Math.floor(to[i * 3 + a] / cell));
    let best = -1, bd = Infinity;
    for (let r = 1; best < 0 || r <= 2; r++) {
      for (let x = -r; x <= r; x++) for (let y = -r; y <= r; y++) for (let z = -r; z <= r; z++) {
        for (const j of grid.get(key(c[0] + x, c[1] + y, c[2] + z)) ?? []) {
          const d = (from[j * 3] - to[i * 3]) ** 2 + (from[j * 3 + 1] - to[i * 3 + 1]) ** 2 + (from[j * 3 + 2] - to[i * 3 + 2]) ** 2;
          if (d < bd) { bd = d; best = j; }
        }
      }
      if (r > 30) break;
    }
    out[i] = best; far = Math.max(far, Math.sqrt(bd));
  }
  return { index: out, far };
}
const centre = (p) => [0, 1, 2].map((a) => { let lo = Infinity, hi = -Infinity; for (let i = a; i < p.length; i += 3) { lo = Math.min(lo, p[i]); hi = Math.max(hi, p[i]); } return (lo + hi) / 2; });

// ---- the game hair's offset against the fitting's hair (before the merge moves nodes about)
const gearHair = new Map(), shift = new Map();
for (const n of gear.getRoot().listNodes()) if (n.getMesh() && /^FemHair(Cards|Cap)$/.test(n.getName())) gearHair.set(n.getName(), centre(bindPositions(n)));
for (const [name, c] of gearHair) {
  const g = named(name);
  if (!g) continue;
  const d = centre(bindPositions(g)).map((v, i) => v - c[i]);
  shift.set(name, d);
  console.log(`HAIR ${name}: game sits ${d.map((v) => (v * 1000).toFixed(1)).join(', ')} mm from the fitting`);
}
// the cards are the same mesh in both files, the cap volume is thinned differently: the hair moved as one piece
if (shift.has('FemHairCards')) shift.set('FemHairCap', shift.get('FemHairCards'));

const map = mergeDocuments(doc, gear);
const theirScene = map.get(gear.getRoot().listScenes()[0]);
const theirs = [];
theirScene.traverse((n) => theirs.push(n));
const parent = gameBody.getParentNode();
let worst = 0, theirBody = null;
for (const node of theirs) {
  if (!node.getMesh()) continue;
  const s = node.getSkin();
  const list = s.listJoints();
  const bound = list.map((j) => {
    const i = joints.get(j.getName());
    if (i === undefined) throw new Error(`joint ${j.getName()} not in the game skin`);
    const a = j.getWorldMatrix(), b = skin.listJoints()[i].getWorldMatrix();
    for (let e = 0; e < 16; e++) worst = Math.max(worst, Math.abs(a[e] - b[e]));
    return skin.listJoints()[i];
  });
  for (const j of list) s.removeJoint(j);
  for (const j of bound) s.addJoint(j);
  s.setSkeleton(skin.getSkeleton());
  node.detach();
  (parent ?? scene).addChild(node);
  const name = node.getName();
  if (BODIES.includes(name)) { theirBody = node; continue; }
  if (gearHair.has(name)) { const m = node.getMesh(); node.dispose(); m.dispose(); continue; } // only the hair offset's reference
  if (only && !only.includes(name)) {
    const targets = [name, ...(name === 'Suit' ? ['Solonchak', 'Ten'] : [])].map(named).filter((n) => n && n !== node);
    const tp = node.getMesh().listPrimitives()[0];
    for (const target of targets) {
      const near = nearest(bindPositions(node), bindPositions(target));
      for (const semantic of tp.listSemantics().filter((k) => k.startsWith('_'))) {
        const src = tp.getAttribute(semantic), e = [];
        const values = Float32Array.from(near.index, (i) => src.getElement(i, e)[0]);
        for (const gp of target.getMesh().listPrimitives())
          gp.setAttribute(semantic, doc.createAccessor(semantic).setType('SCALAR').setArray(values).setBuffer(root.listBuffers()[0]));
        console.log(`LENT ${semantic} to ${target.getName()}: ${values.filter((v) => v >= 0.5).length} of ${values.length}, farthest ${(near.far * 1000).toFixed(1)} mm`);
      }
    }
    const m = node.getMesh(); node.dispose(); m.dispose(); continue;
  }
  for (const p of node.getMesh().listPrimitives()) {
    const m = own.get(p.getMaterial()?.getName());
    if (m) p.setMaterial(m);
  }
  const old = root.listNodes().find((n) => n !== node && n.getName() === name && n.getMesh());
  if (old) { const m = old.getMesh(); old.dispose(); m.dispose(); console.log(`REPLACED ${name}`); }
  const hair = name.match(/^(FemHair(?:Cards|Cap))_/);
  if (hair && shift.has(hair[1])) {
    // the offset is in bind world space: move the vertices by it through the bind transform's inverse
    const t = shift.get(hair[1]), inv = invert(mul(bound[0].getWorldMatrix(), s.getInverseBindMatrices().getElement(0, [])));
    const dl = [0, 1, 2].map((r) => inv[r] * t[0] + inv[4 + r] * t[1] + inv[8 + r] * t[2]);
    const pos = node.getMesh().listPrimitives()[0].getAttribute('POSITION');
    for (let i = 0; i < pos.getCount(); i++) pos.setElement(i, pos.getElement(i, []).map((v, k) => v + dl[k]));
    console.log(`HAIR ${name}: moved with ${hair[1]}`);
  }
  console.log(`PIECE ${name} (${node.getMesh().listPrimitives().map((p) => p.getMaterial()?.getName()).join(', ')}), ${list.length} joints`);
}
function invert(m) {
  // affine 4x4 inverse
  const [a, b, c, d, e, f, g, h, i] = [m[0], m[4], m[8], m[1], m[5], m[9], m[2], m[6], m[10]];
  const det = a * (e * i - f * h) - b * (d * i - f * g) + c * (d * h - e * g);
  const r = [(e * i - f * h) / det, (c * h - b * i) / det, (b * f - c * e) / det,
    (f * g - d * i) / det, (a * i - c * g) / det, (c * d - a * f) / det,
    (d * h - e * g) / det, (b * g - a * h) / det, (a * e - b * d) / det];
  const t = [m[12], m[13], m[14]];
  const tr = [0, 1, 2].map((k) => -(r[k * 3] * t[0] + r[k * 3 + 1] * t[1] + r[k * 3 + 2] * t[2]));
  return [r[0], r[3], r[6], 0, r[1], r[4], r[7], 0, r[2], r[5], r[8], 0, tr[0], tr[1], tr[2], 1];
}
console.log(`joint rest matrices differ by at most ${worst.toExponential(2)}`);
if (worst > 1e-3) throw new Error('rest poses differ');

// ---- body masks
if (!theirBody) throw new Error('no body in the gear file');
const near = nearest(bindPositions(theirBody), bindPositions(gameBody));
console.log(`BODY masks matched, farthest ${(near.far * 1000).toFixed(1)} mm`);
const tp = theirBody.getMesh().listPrimitives()[0];
for (const gp of gameBody.getMesh().listPrimitives()) {
  for (const semantic of tp.listSemantics()) {
    if (!semantic.startsWith('_')) continue;
    const src = tp.getAttribute(semantic), e = [];
    const values = new Float32Array(near.index.length);
    let hidden = 0;
    for (let i = 0; i < values.length; i++) { values[i] = src.getElement(near.index[i], e)[0]; if (values[i] >= 0.5) hidden++; }
    const acc = doc.createAccessor(semantic).setType('SCALAR').setArray(values).setBuffer(root.listBuffers()[0]);
    gp.setAttribute(semantic, acc);
    console.log(`BODY ${semantic}: ${hidden} of ${values.length} hidden`);
  }
}
const tm = theirBody.getMesh();
theirBody.dispose(); tm.dispose();

// ---- recoloured field suits
if (recolours) {
  const suit = named('Suit');
  for (const file of fs.readdirSync(recolours).filter((f) => /^M_\w+\.color\.1024\.webp$/.test(f))) {
    const name = file.split('.')[0].slice(2);
    const old = named(name);  // a game model merged before already wears this recolour
    if (old) { const m = old.getMesh(); old.dispose(); m.dispose(); }
    const mat = suit.getMesh().listPrimitives()[0].getMaterial().clone().setName(`M_${name}`);
    const tex = doc.createTexture(`M_${name}.color`).setImage(fs.readFileSync(path.join(recolours, file))).setMimeType('image/webp');
    mat.setBaseColorTexture(tex);
    const mesh = doc.createMesh(name);
    for (const p of suit.getMesh().listPrimitives()) mesh.addPrimitive(p.clone().setMaterial(mat));
    const node = doc.createNode(name).setMesh(mesh).setSkin(suit.getSkin());
    suit.getParentNode() ? suit.getParentNode().addChild(node) : scene.addChild(node);
    console.log(`SUIT ${name} from ${file}`);
  }
}
for (const n of theirs) if (!n.isDisposed() && !n.getMesh() && !n.listChildren().length) n.dispose();
theirScene.dispose();
await doc.transform(prune({ keepAttributes: true }), unpartition(), meshopt({ encoder: MeshoptEncoder, level: 'high' }));
await io.write(outPath, doc);
for (const n of root.listNodes()) if (n.getMesh()) console.log(`  node ${n.getName()}`);
console.log(`WROTE ${outPath}`);
