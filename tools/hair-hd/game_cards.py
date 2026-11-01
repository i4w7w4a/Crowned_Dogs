"""Game version of one HD hairstyle: hair cards and a scalp cap from the strands.

usage: blender -b 3d/hair-hd/<groom>.blend --python game_cards.py -- STYLE OUT.blend TEXTURE.png
The HD groom has 60-75 thousand strands; a phone draws a few hundred textured strips instead.
The strands are grouped by shape (root, middle, tip) into clumps; each clump becomes one strip
that follows the clump's median strand, as wide as the clump, lying flat on the hair volume, plus
a second narrower strip over it for volume (like the old female cards.py). The groom's own dark
Scalp_cap stays under them so no skin shows between strips. Everything stays in the persona's rest
space; pack.py moves it under the head bone. Faces f2-f4 are checked against the cap.
"""
import bpy, sys
import numpy as np
from mathutils.bvhtree import BVHTree

args = sys.argv[sys.argv.index('--') + 1:]
style, out_p, tex_p = args[:3]
# clumps and segments per groom object: (object, mesh suffix, clumps, segments, width clip)
PLAN = {
    'pixie': [('Hair_pixie', 'cards', 460, 6, (0.004, 0.016))],
    'bob': [('Hair_bob', 'cards', 520, 10, (0.0045, 0.02))],
    'braid': [('Hair_braid_scalp', 'cards', 440, 10, (0.0045, 0.02)),
              ('Hair_braid_plait', 'plait', 90, 18, (0.003, 0.012))],
    # male: the HD grooms in male_short/slick/medium.blend
    'crop': [('Hair_short', 'cards', 480, 4, (0.0045, 0.014))],
    'slick': [('Hair_slick', 'cards', 420, 8, (0.006, 0.02))],
    'medium': [('Hair_medium', 'cards', 520, 10, (0.0045, 0.018))],
}[style]
FACES = [f'Head_{"m" if style in ("crop", "slick", "medium") else "f"}{i}' for i in range(1, 5)]
rng = np.random.default_rng(7)
dg = bpy.context.evaluated_depsgraph_get()


def unit(v):
    n = np.linalg.norm(v, axis=-1, keepdims=True)
    return v / np.maximum(n, 1e-9)


def smooth(v, k=1):
    for _ in range(k):
        s = v.copy(); s[1:-1] = (v[:-2] + 2 * v[1:-1] + v[2:]) / 4; v = s
    return v


def world_mesh(o):
    e = o.evaluated_get(dg); m = e.to_mesh()
    M = np.array(o.matrix_world)
    V = np.array([v.co for v in m.vertices]) @ M[:3, :3].T + M[:3, 3]
    F = [tuple(p.vertices) for p in m.polygons]
    e.to_mesh_clear()
    return V, F


def strands(o):
    c = o.evaluated_get(dg).data
    P = np.zeros(len(c.points) * 3, np.float32); c.attributes['position'].data.foreach_get('vector', P)
    M = np.array(o.matrix_world)
    P = P.reshape(-1, 3) @ M[:3, :3].T + M[:3, 3]
    n = c.curves[0].points_length
    return P.reshape(-1, n, 3).astype(np.float64)


def resample(curves, n):
    d = np.concatenate([np.zeros((len(curves), 1)), np.cumsum(np.linalg.norm(np.diff(curves, axis=1), axis=2), axis=1)], 1)
    out = np.zeros((len(curves), n + 1, 3))
    t = np.linspace(0, 1, n + 1)
    for k in range(3):
        for i in range(len(curves)):
            out[i, :, k] = np.interp(t * d[i, -1], d[i], curves[i, :, k])
    return out, d[:, -1]


def kmeans(X, k, iters=10):
    sub = X[rng.choice(len(X), min(len(X), 20000), replace=False)].astype(np.float32)
    seeds = [int(rng.integers(len(sub)))]
    dist = np.linalg.norm(sub - sub[seeds[0]], axis=1)
    for _ in range(k - 1):
        seeds.append(int(dist.argmax())); dist = np.minimum(dist, np.linalg.norm(sub - sub[seeds[-1]], axis=1))
    cent = sub[seeds].copy()

    def assign(A):
        lab = np.empty(len(A), np.int32)
        cc = (cent ** 2).sum(1)
        for s in range(0, len(A), 8000):
            a = A[s:s + 8000]
            lab[s:s + 8000] = ((a ** 2).sum(1)[:, None] - 2 * a @ cent.T + cc[None]).argmin(1)
        return lab
    for _ in range(iters):
        lab = assign(sub)
        for j in range(k):
            if (lab == j).any():
                cent[j] = sub[lab == j].mean(0)
    return assign(X.astype(np.float32))


head = bpy.data.objects[FACES[0]]
hv, _ = world_mesh(head)
zt = hv[:, 2].max(); top = hv[hv[:, 2] > zt - 0.06]
C = np.array([top[:, 0].mean(), top[:, 1].mean(), zt - 0.095])

out_objs = []
roots = []
for obj_name, suffix, K, SEG, (wmin, wmax) in PLAN:
    S = strands(bpy.data.objects[obj_name])
    R, L = resample(S, SEG)
    # the crop's faded sides are a few millimetres of stubble: as strips they read as scattered dots, so
    # only the top gets strips and the male head's own baked buzz shows the fade
    keep = L > (0.014 if style == 'crop' else 0.004)
    R, L = R[keep], L[keep]
    roots.append(R[:, 0])
    feat = np.concatenate([R[:, 0], R[:, SEG // 3] * 0.8, R[:, 2 * SEG // 3] * 0.6, R[:, -1] * 0.4], 1)
    lab = kmeans(feat, K)
    t = np.linspace(0, 1, SEG + 1)
    verts, faces, uvs, normals = [], [], [], []
    for j in range(K):
        grp = R[lab == j]
        if len(grp) < 4:
            continue
        centre = smooth(np.median(grp, axis=0))
        centre[0] = np.median(grp[:, 0], axis=0)
        # braid: as strips the loose face strand turned into a dark block over the eye (Art,
        # 2026-11-01), so no strip hangs in front of the face below the forehead
        if style == 'braid' and suffix == 'cards':
            front = (np.abs(centre[:, 0] - C[0]) < 0.06) & (centre[:, 1] < C[1] - 0.05) & (centre[:, 2] < zt - 0.075)
            if front.sum() >= 2:
                continue
        tan = unit(np.gradient(centre, axis=0))
        out = unit(centre - C)
        across = unit(np.cross(tan, out))
        nrm = unit(np.cross(across, tan))
        off = grp - centre[None]
        wa = np.percentile(np.abs(np.einsum('skc,kc->sk', off, across)), 80, axis=0)
        wn = np.percentile(np.abs(np.einsum('skc,kc->sk', off, nrm)), 80, axis=0)
        half = np.clip(smooth(wa, 2) * 1.3, wmin, wmax) * (1 - (0.1 if style == 'slick' else 0.35) * t ** 2)
        # blunt strip roots stacked along the hairline read as dark steps: they start narrow
        # (the slick lies flat and every strip root shows on the back of the head: narrow roots and a
        # second layer there stacked into rows of scales)
        if style != 'slick':
            half = half * (0.3 + 0.7 * np.clip(t * SEG / 1.5, 0, 1) ** 0.7)
        thick = np.clip(smooth(wn, 2), 0, 0.01)
        n = unit(0.7 * nrm + 0.3 * out) if suffix != 'plait' else nrm
        vtip = 0.8 if style == 'slick' else 1.0   # the slick's strips end bluntly, short of the texture's tips
        for layer in ((0,) if style == 'slick' else (0, 1)):
            k = 1.0 if layer == 0 else 0.65
            shift = 0.0 if layer == 0 else rng.uniform(-0.35, 0.35)
            lift = (0.0 if layer == 0 else 0.6) * thick + 0.0006 * layer
            u0 = rng.uniform(0, 0.5)
            base = len(verts)
            for i in range(SEG + 1):
                c = centre[i] + nrm[i] * lift[i] + across[i] * shift * half[i]
                for side in (-1, 1):
                    verts.append(tuple(c + across[i] * side * half[i] * k))
                    uvs.append((u0 + (0.5 + 0.5 * side) * 0.5, t[i] * vtip))
                    normals.append(tuple(n[i]))
            for i in range(SEG):
                a = base + 2 * i
                faces.append((a, a + 1, a + 3, a + 2))
    name = f'Hair_{style}_{suffix}'
    me = bpy.data.meshes.new(name)
    me.from_pydata(verts, [], faces)
    me.shade_smooth()
    uv = me.uv_layers.new(name='UVMap')
    for poly in me.polygons:
        for li in poly.loop_indices:
            uv.data[li].uv = uvs[me.loops[li].vertex_index]
    me.normals_split_custom_set_from_vertices(normals)
    me.update()
    ob = bpy.data.objects.new(name, me)
    out_objs.append(ob)
    print('CARDS', name, 'strands', len(R), 'strips', len(faces) // SEG, 'tris', 2 * len(faces))

# the groom's dark cap, and the braid's tie
cv, cf = world_mesh(bpy.data.objects['Scalp_cap'])
if style == 'crop':
    # the faded sides: the whole HD cap, a hair above it, textured with hair-fade.png (fade_texture.py):
    # u runs round the head and tiles every 4 cm, v climbs from sparse stubble at the hairline to a
    # solid pile under the strips, by elevation like the HD fade
    Ch = np.array([0.003, -0.008, 1.700])   # hairgen's male cranium centre: elevations match the groom
    d = cv - Ch
    az = np.degrees(np.arctan2(d[:, 0], -d[:, 1]))
    el = np.degrees(np.arctan2(d[:, 2], np.hypot(d[:, 0], d[:, 1])))
    fu = (az + 180) / 360 * 14
    fv = np.clip((el + 30) / 70, 0.02, 0.98)   # off the edges: the texture repeats, the far edge would bleed in
    fp = cv + unit(d) * 0.0004
    me = bpy.data.meshes.new('Hair_crop_fade'); me.from_pydata([tuple(v) for v in fp], [], cf); me.shade_smooth()
    uv = me.uv_layers.new(name='UVMap')
    for poly in me.polygons:
        us = [fu[me.loops[li].vertex_index] for li in poly.loop_indices]
        wrap = max(us) - min(us) > 7   # a face across the nape seam: keep its u continuous
        for li in poly.loop_indices:
            vi = me.loops[li].vertex_index
            uv.data[li].uv = (fu[vi] + (14 if wrap and fu[vi] < 7 else 0), fv[vi])
    me.normals_split_custom_set_from_vertices([tuple(n) for n in unit(d)])
    me.update()
    out_objs.append(bpy.data.objects.new('Hair_crop_fade', me))
    print('FADE', len(fp), 'verts', sum(len(f) - 2 for f in cf), 'tris')
# the HD cap reaches the thin hairline, where fine strands hid it; strips there are sparse and its
# edge showed as dark teeth at the temples. It keeps the faces where roots are at least half as
# dense as on the crown (roots within 8 mm of every corner).
from mathutils.kdtree import KDTree
rk = np.concatenate(roots); kr = KDTree(len(rk))
for i, r in enumerate(rk):
    kr.insert(tuple(r), i)
kr.balance()
dens = np.array([len(kr.find_range(tuple(v), 0.008)) for v in cv])
dense = dens >= 0.5 * np.median(dens)
cf = [f for f in cf if dense[list(f)].all()]
used = sorted({i for f in cf for i in f}); remap = {o: n for n, o in enumerate(used)}
cv = cv[used]; cf = [tuple(remap[i] for i in f) for f in cf]
me = bpy.data.meshes.new(f'Hair_{style}_cap'); me.from_pydata([tuple(v) for v in cv], [], cf); me.shade_smooth(); me.update()
cap = bpy.data.objects.new(f'Hair_{style}_cap', me); out_objs.append(cap)
print('CAP', len(cv), 'verts', sum(len(f) - 2 for f in cf), 'tris')
if 'Braid_tie' in bpy.data.objects:
    tv, tf = world_mesh(bpy.data.objects['Braid_tie'])
    me = bpy.data.meshes.new(f'Hair_{style}_tie'); me.from_pydata([tuple(v) for v in tv], [], tf); me.shade_smooth(); me.update()
    out_objs.append(bpy.data.objects.new(f'Hair_{style}_tie', me))

# faces 1-4 against the cap: one hairstyle serves all four heads, so wherever a skull stands
# out of the cap (rays from outside in, towards the skull centre), the cap and the strips over it
# move out by that much plus a millimetre, fading out over 3 cm away from the cap.
from mathutils.kdtree import KDTree
heads = []
for face in FACES:
    V, F = world_mesh(bpy.data.objects[face])
    heads.append((face, BVHTree.FromPolygons([tuple(v) for v in V], F)))


def outer(bvh, d):
    hit = bvh.ray_cast(tuple(C + d * 0.4), tuple(-d))
    return None if hit[0] is None else np.linalg.norm(np.array(hit[0]) - C)


dirs = unit(cv - C); rc = np.linalg.norm(cv - C, axis=1)
delta = np.zeros(len(cv))
for face, bvh in heads:
    r = np.array([outer(bvh, d) or 0.0 for d in dirs])
    need = np.maximum(0, r + 0.001 - rc)
    need[need > 0.014] = 0  # an ear standing out, not the skull
    print('CLEARANCE', face, 'cap points under skin', int((r > rc + 0.0002).sum()), 'max mm', round(1000 * need.max(), 1))
    delta = np.maximum(delta, need)
kd = KDTree(len(cv))
for i, v in enumerate(cv):
    kd.insert(tuple(v), i)
kd.balance()


def push(P):
    out = P.copy()
    for i, p in enumerate(P):
        near = kd.find_n(tuple(p), 8)
        w = np.array([1 / (n[2] + 0.002) for n in near]); dd = np.array([delta[n[1]] for n in near])
        fade = np.clip(1 - (near[0][2] - 0.01) / 0.03, 0, 1)
        out[i] = p + unit(p - C) * float((w * dd).sum() / w.sum()) * fade
    return out


if delta.max() > 0:
    for o in out_objs:
        P = np.array([v.co for v in o.data.vertices])
        Q = push(P) if not o.name.endswith('_cap') else P + dirs * delta[:, None]
        for v, q in zip(o.data.vertices, Q):
            v.co = tuple(q)
        o.data.update()
    print('PUSHED out, max mm', round(1000 * delta.max(), 1))

# a clean file with only the game pieces
keep = {o.name for o in out_objs}
for o in [o for o in bpy.data.objects if o.name not in keep]:
    bpy.data.objects.remove(o)
for o in out_objs:
    bpy.context.scene.collection.objects.link(o)
    o.name = o.data.name = o.name.split('.')[0]
bpy.ops.outliner.orphans_purge(do_recursive=True)
bpy.ops.wm.save_as_mainfile(filepath=out_p, compress=True)
print('SAVED', out_p)
