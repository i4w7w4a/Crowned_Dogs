// node tools/anim/retarget_ual.mjs GAME.glb UAL.glb OUT.glb name=Clip[+Clip...][@from-to][~k][^Clip:k] ...
// Puts clips from Quaternius' Universal Animation Library 2 (CC0, UE-style bone names, T rest pose) on a Persona
// (61-bone UE skeleton, A rest pose). Every bone keeps its world turn: the source pose is first bent into the
// Persona's rest shape bone by bone (each bone swung to point where the Persona's bone points, the hand also
// twisted to the thumb), then each frame's world rotation is carried over. The pelvis moves by the source's
// offset scaled to the Persona's hip height. `name` replaces a clip of that name; `+` joins clips one after
// another, `@from-to` keeps that many seconds of the joined clip, `~k` tames the body and `^Clip:k` closes the
// right hand (both below). Run from the repo root.
import { NodeIO } from '@gltf-transform/core';
import { ALL_EXTENSIONS } from '@gltf-transform/extensions';
import { MeshoptDecoder, MeshoptEncoder } from 'meshoptimizer';

const FPS = 30;
const [gameFile, ualFile, outFile, ...wanted] = process.argv.slice(2);
await MeshoptDecoder.ready; await MeshoptEncoder.ready;
const io = new NodeIO().registerExtensions(ALL_EXTENSIONS)
  .registerDependencies({ 'meshopt.decoder': MeshoptDecoder, 'meshopt.encoder': MeshoptEncoder });
const game = await io.read(gameFile), ual = await io.read(ualFile);

const qmul = (a, b) => [a[3] * b[0] + a[0] * b[3] + a[1] * b[2] - a[2] * b[1], a[3] * b[1] - a[0] * b[2] + a[1] * b[3] + a[2] * b[0],
  a[3] * b[2] + a[0] * b[1] - a[1] * b[0] + a[2] * b[3], a[3] * b[3] - a[0] * b[0] - a[1] * b[1] - a[2] * b[2]];
const qinv = (q) => [-q[0], -q[1], -q[2], q[3]];
const qnorm = (q) => { const l = Math.hypot(...q) || 1; return q.map((v) => v / l); };
const rot = (q, v) => qmul(qmul(q, [...v, 0]), qinv(q)).slice(0, 3);
const sub = (a, b) => a.map((v, i) => v - b[i]), add = (a, b) => a.map((v, i) => v + b[i]);
const dot = (a, b) => a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
const cross = (a, b) => [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
const unit = (v) => { const l = Math.hypot(...v) || 1; return v.map((x) => x / l); };
const axisAngle = (axis, angle) => { const s = Math.sin(angle / 2); return [axis[0] * s, axis[1] * s, axis[2] * s, Math.cos(angle / 2)]; };
const swing = (from, to) => { from = unit(from); to = unit(to); const c = cross(from, to), d = dot(from, to);
  if (d < -0.9999) return axisAngle(unit(Math.abs(from[0]) < 0.9 ? cross(from, [1, 0, 0]) : cross(from, [0, 1, 0])), Math.PI);
  return qnorm([c[0], c[1], c[2], 1 + d]); };
const slerp = (a, b, t) => { let d = a[0] * b[0] + a[1] * b[1] + a[2] * b[2] + a[3] * b[3];
  if (d < 0) { b = b.map((v) => -v); d = -d; }
  if (d > 0.9995) return qnorm(a.map((v, i) => v + (b[i] - v) * t));
  const th = Math.acos(d), s = Math.sin(th); return a.map((v, i) => (v * Math.sin((1 - t) * th) + b[i] * Math.sin(t * th)) / s); };

// One skeleton: nodes by lower-case name, parents, rest local TRS.
const skeleton = (doc) => {
  const joints = doc.getRoot().listSkins()[0].listJoints(), by = new Map(joints.map((j) => [j.getName().toLowerCase(), j]));
  let top = joints[0]; while (top.getParentNode() && joints.includes(top.getParentNode())) top = top.getParentNode();
  const above = top.getParentNode(), base = above ? above.getWorldMatrix() : null;
  const baseQ = above ? [...above.getRotation()] : [0, 0, 0, 1], baseP = base ? [base[12], base[13], base[14]] : [0, 0, 0];
  return { joints, by, top, baseQ, baseP };
};
// World turns and positions of every joint for a local pose (name -> {t, r}).
const pose = (sk, local) => {
  const W = new Map();
  const walk = (j, pq, pp) => { const n = j.getName().toLowerCase(), l = local(n, j);
    const q = qmul(pq, l.r), p = add(pp, rot(pq, l.t)); W.set(n, { q, p });
    for (const c of j.listChildren()) if (sk.joints.includes(c)) walk(c, q, p); };
  walk(sk.top, sk.baseQ, sk.baseP); return W;
};
const restLocal = (n, j) => ({ t: [...j.getTranslation()], r: [...j.getRotation()] });
const S = skeleton(ual), T = skeleton(game);
const SR = pose(S, restLocal), TR = pose(T, restLocal);

// Where each bone points, and which bone takes its alignment from its parent.
const AIM = { pelvis: 'spine_01', spine_01: 'spine_02', spine_02: 'spine_03', spine_03: 'neck_01', neck_01: 'head',
  clavicle_l: 'upperarm_l', upperarm_l: 'lowerarm_l', lowerarm_l: 'hand_l', hand_l: 'middle_01_l',
  clavicle_r: 'upperarm_r', upperarm_r: 'lowerarm_r', lowerarm_r: 'hand_r', hand_r: 'middle_01_r',
  thigh_l: 'calf_l', calf_l: 'foot_l', foot_l: 'ball_l', thigh_r: 'calf_r', calf_r: 'foot_r', foot_r: 'ball_r' };
for (const s of ['l', 'r']) for (const f of ['index', 'middle', 'ring', 'pinky', 'thumb'])
  for (const k of [1, 2]) AIM[`${f}_0${k}_${s}`] = `${f}_0${k + 1}_${s}`;
const TWIST = { hand_l: 'thumb_01_l', hand_r: 'thumb_01_r' };
const align = new Map();
const alignOf = (n) => {
  if (align.has(n)) return align.get(n);
  let q = [0, 0, 0, 1];
  const aim = AIM[n];
  if (aim && SR.has(aim) && TR.has(aim)) {
    const dS = sub(SR.get(aim).p, SR.get(n).p), dT = sub(TR.get(aim).p, TR.get(n).p);
    q = swing(dS, dT);
    const tw = TWIST[n];
    if (tw) { const a = unit(dT), sS = rot(q, sub(SR.get(tw).p, SR.get(n).p)), sT = sub(TR.get(tw).p, TR.get(n).p);
      const pS = unit(sub(sS, a.map((v) => v * dot(sS, a)))), pT = unit(sub(sT, a.map((v) => v * dot(sT, a))));
      q = qmul(axisAngle(a, Math.atan2(dot(cross(pS, pT), a), dot(pS, pT))), q); }
  } else {
    const parent = T.by.get(n)?.getParentNode()?.getName().toLowerCase();
    if (parent && T.by.has(parent)) q = alignOf(parent);
  }
  align.set(n, q); return q;
};
// At = As * Rs^-1 * Q^-1 * Rt: the Persona bone points where the source bone points.
const offset = new Map();
for (const n of T.by.keys()) if (S.by.has(n) && n !== 'root')
  offset.set(n, qmul(qmul(qinv(SR.get(n).q), qinv(alignOf(n))), TR.get(n).q));
const hip = TR.get('pelvis').p[1] / SR.get('pelvis').p[1];

// Sampling the source clip.
const clipOf = (name) => { const a = ual.getRoot().listAnimations().find((x) => x.getName() === name);
  if (!a) throw new Error(`no clip ${name} in ${ualFile}`);
  const ch = new Map(); let end = 0;
  for (const c of a.listChannels()) { const s = c.getSampler(), times = s.getInput().getArray(), vals = s.getOutput().getArray();
    end = Math.max(end, times[times.length - 1]);
    ch.set(`${c.getTargetNode().getName().toLowerCase()}.${c.getTargetPath()}`, { times, vals, step: s.getInterpolation() === 'STEP' }); }
  return { ch, end }; };
const sample = (c, key, t, size, fallback) => { const s = c.ch.get(key); if (!s) return fallback;
  const { times, vals } = s; let i = 0; while (i < times.length - 1 && times[i + 1] <= t) i++;
  const at = (k) => Array.from(vals.subarray(k * size, k * size + size));
  if (i >= times.length - 1 || s.step) return at(i);
  const f = (t - times[i]) / (times[i + 1] - times[i]); if (f <= 0) return at(i);
  return size === 4 ? slerp(at(i), at(i + 1), f) : at(i).map((v, k) => v + (at(i + 1)[k] - v) * f); };

// The Persona's local pose (bone name -> { r, t }) for one moment of a source clip.
const frameOf = (c, time) => {
  const SW = pose(S, (n, j) => ({ t: sample(c, `${n}.translation`, time, 3, [...j.getTranslation()]),
    r: sample(c, `${n}.rotation`, time, 4, [...j.getRotation()]) }));
  const out = new Map();
  const walk = (j, pq, pp) => { const n = j.getName().toLowerCase(), lq = [...j.getRotation()], lt = [...j.getTranslation()];
    const q = offset.has(n) ? qnorm(qmul(SW.get(n).q, offset.get(n))) : qmul(pq, lq);
    let t = lt;
    if (n === 'pelvis') { const p = add(TR.get(n).p, sub(SW.get(n).p, SR.get(n).p).map((v) => v * hip));
      t = rot(qinv(pq), sub(p, pp)); }
    out.set(j.getName(), { r: qnorm(qmul(qinv(pq), q)), t });
    const p = add(pp, rot(pq, t));
    for (const c of j.listChildren()) if (T.joints.includes(c)) walk(c, q, p); };
  walk(T.top, T.baseQ, T.baseP); return out;
};

const DAMP = /^(pelvis|spine_0\d|thigh_[lr]|calf_[lr]|foot_[lr]|ball_[lr])$/;
const GRIP = /^(thumb|index|middle|ring|pinky)_0\d_r$/;
const retarget = (spec) => {
  const [name, rest] = spec.split('='); const [main, grip] = rest.split('^');
  const [body, damp] = main.split('~'), [list, cut] = body.split('@');
  const parts = list.split('+').map(clipOf), frames = [];
  let offsetT = 0;
  parts.forEach((c, pi) => { const n = Math.round(c.end * FPS);
    for (let f = pi ? 1 : 0; f <= n; f++) frames.push({ c, t: f / FPS, at: offsetT + f / FPS });
    offsetT += n / FPS; });
  const [from, to] = cut ? cut.split('-').map(Number) : [0, offsetT];
  const kept = frames.filter((f) => f.at >= from - 1e-6 && f.at <= to + 1e-6);
  const tracks = new Map(T.joints.map((j) => [j.getName(), { r: [], t: [] }]));
  for (const f of kept) for (const [bone, { r, t }] of frameOf(f.c, f.t)) { const tr = tracks.get(bone); tr.r.push(...r); tr.t.push(...t); }
  // `^Clip:k` bends the right hand's fingers k of the way to their pose in Clip's first frame (a fist): the
  // throw opens the hand wide, and the grenade in it needs fingers round it.
  if (grip) { const [from, k] = grip.split(':'), fist = frameOf(clipOf(from), 0);
    for (const [bone, tr] of tracks) { if (!GRIP.test(bone)) continue;
      for (let i = 0; i < tr.r.length; i += 4) tr.r.splice(i, 4, ...slerp(tr.r.slice(i, i + 4), fist.get(bone).r, Number(k))); } }
  // Keep each quaternion track on one hemisphere so the mixer never spins the long way round.
  for (const tr of tracks.values()) for (let i = 4; i < tr.r.length; i += 4)
    if (tr.r[i] * tr.r[i - 4] + tr.r[i + 1] * tr.r[i - 3] + tr.r[i + 2] * tr.r[i - 2] + tr.r[i + 3] * tr.r[i - 1] < 0)
      for (let k = 0; k < 4; k++) tr.r[i + k] = -tr.r[i + k];
  // `~k` keeps only k of the hips', trunk's and legs' travel from the first frame: the library's strikes lunge
  // to the floor and its throw bows to the knees, cartoon-big for the arena. The arms keep their full swing.
  if (damp) { const k = Number(damp);
    for (const [bone, tr] of tracks) { if (!DAMP.test(bone)) continue;
      for (let i = 4; i < tr.r.length; i += 4) tr.r.splice(i, 4, ...slerp(tr.r.slice(0, 4), tr.r.slice(i, i + 4), k));
      if (bone === 'pelvis') for (let i = 3; i < tr.t.length; i++) tr.t[i] = tr.t[i % 3] + (tr.t[i] - tr.t[i % 3]) * k; } }
  const root = game.getRoot(), buffer = root.listBuffers()[0];
  for (const old of root.listAnimations().filter((a) => a.getName() === name)) {
    const used = old.listSamplers().flatMap((s) => [s.getInput(), s.getOutput()]).filter(Boolean);
    old.dispose();
    // Accessors may be shared with other clips; only the ones nothing else reads go.
    for (const a of new Set(used)) if (!a.isDisposed() && a.listParents().every((p) => p === root)) a.dispose();
  }
  const anim = game.createAnimation(name);
  const times = game.createAccessor().setType('SCALAR').setBuffer(buffer)
    .setArray(new Float32Array(kept.map((f) => f.at - kept[0].at)));
  for (const j of T.joints) { const tr = tracks.get(j.getName());
    for (const [path, vals, type] of [['rotation', tr.r, 'VEC4'], ['translation', tr.t, 'VEC3'], ['scale', null, 'VEC3']]) {
      const out = game.createAccessor().setType(type).setBuffer(buffer);
      const s = game.createAnimationSampler();
      if (vals) { out.setArray(new Float32Array(vals)); s.setInput(times).setInterpolation('LINEAR'); }
      else { out.setArray(new Float32Array([1, 1, 1, 1, 1, 1]));
        s.setInput(game.createAccessor().setType('SCALAR').setBuffer(buffer).setArray(new Float32Array([0, (kept.length - 1) / FPS])))
          .setInterpolation('STEP'); }
      s.setOutput(out); anim.addSampler(s);
      anim.addChannel(game.createAnimationChannel().setTargetNode(j).setTargetPath(path).setSampler(s)); } }
  console.log(`${name}: ${list} ${(kept.length - 1) / FPS}s, ${kept.length} frames`);
};
for (const spec of wanted) retarget(spec);
await io.write(outFile, game);
