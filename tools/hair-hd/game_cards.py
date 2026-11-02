"""Game version of one HD hairstyle: hair cards and a scalp cap from the strands.

usage: blender -b 3d/hair-hd/<groom>.blend --python game_cards.py -- STYLE OUT.blend TEXTURE.png [HEADS.blend]
HEADS.blend holds the game's current heads at rest (tools/beards/prepare.py writes output/beards/heads-rest.blend
for the male ones); without it the groom file's own heads are used.
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
heads_p = args[3] if len(args) > 3 else None
# clumps and segments per groom object: (object, mesh suffix, clumps, segments, width clip)
PLAN = {
    'pixie': [('Hair_pixie', 'cards', 460, 6, (0.004, 0.016))],
    'bob': [('Hair_bob', 'cards', 520, 10, (0.0045, 0.02))],
    'braid': [('Hair_braid_scalp', 'cards', 440, 10, (0.0045, 0.02)),
              ('Hair_braid_plait', 'plait', 90, 18, (0.003, 0.012))],
    # male: the HD grooms in male_short/slick/medium/ponytail.blend
    'crop': [('Hair_short', 'cards', 480, 4, (0.0045, 0.014))],
    'slick': [('Hair_slick', 'cards', 420, 8, (0.006, 0.02))],
    'medium': [('Hair_medium', 'cards', 520, 10, (0.0045, 0.018))],
    'ponytail': [('Hair_ponytail', 'cards', 440, 8, (0.006, 0.02)),
                 ('Hair_ponytail_tail', 'tail', 80, 14, (0.003, 0.012))],
}[style]
FACES = [f'Head_{"m" if style in ("crop", "slick", "medium", "ponytail") else "f"}{i}' for i in range(1, 5)]
# combed flat back over the skull: one layer, roots narrowed and faded in at the thin hairline, painted on the head under it
SLEEK = style in ('slick', 'ponytail')
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
hv, hf = world_mesh(head)
zt = hv[:, 2].max(); top = hv[hv[:, 2] > zt - 0.06]
C = np.array([top[:, 0].mean(), top[:, 1].mean(), zt - 0.095])

out_objs = []
roots = []
fade_paths = []
for obj_name, suffix, K, SEG, (wmin, wmax) in PLAN:
    S = strands(bpy.data.objects[obj_name])
    R, L = resample(S, SEG)
    # the crop's faded sides are a few millimetres of stubble: as strips they read as scattered dots, so
    # only the top gets strips and the male head's own baked buzz shows the fade
    keep = L > (0.014 if style == 'crop' else 0.004)
    R, L = R[keep], L[keep]
    # the ponytail's groom has a few thousand strands that run through the skull or hang over the face (hidden
    # or fine in HD): mixed into clumps they made strips across the forehead
    if style == 'ponytail' and suffix == 'cards':
        bvh = BVHTree.FromPolygons([tuple(v) for v in hv], hf)
        depth = np.zeros(len(R))
        for i, st in enumerate(R):
            for q in st[::2]:
                d = unit(q - C)
                hit = bvh.ray_cast(tuple(C + d * 0.4), tuple(-d))
                if hit[0] is not None:
                    depth[i] = max(depth[i], np.linalg.norm(np.array(hit[0]) - C) - np.linalg.norm(q - C))
        face = (np.abs(R[..., 0] - C[0]) < 0.06) & (R[..., 1] < C[1] - 0.05) & (R[..., 2] < zt - 0.085)
        # and those that leave the front hairline downwards: as a strip one ran down over the temple
        d0 = unit(R[:, SEG // 4] - R[:, 0])
        down = (R[:, 0, 1] < C[1] - 0.05) & (np.abs(R[:, 0, 0] - C[0]) < 0.05) & (d0[:, 2] < -0.2)
        bad = (depth > 0.006) | (face.sum(1) >= 3) | down
        print('DROPPED through the skull or over the face', int(bad.sum()), 'of', len(R))
        R, L = R[~bad], L[~bad]
    roots.append(R[:, 0])
    feat = np.concatenate([R[:, 0], R[:, SEG // 3] * 0.8, R[:, 2 * SEG // 3] * 0.6, R[:, -1] * 0.4], 1)
    lab = kmeans(feat, K)
    # roots per 8 mm round each root: low at the hairline
    from mathutils.kdtree import KDTree
    kd = KDTree(len(R)); [kd.insert(tuple(r), i) for i, r in enumerate(R[:, 0])]; kd.balance()
    rdens = np.array([len(kd.find_range(tuple(r), 0.008)) for r in R[:, 0]]); rmed = np.median(rdens)
    t = np.linspace(0, 1, SEG + 1)
    sleek = SLEEK and suffix == 'cards'
    centres = {}
    for j in range(K):
        grp = R[lab == j]
        if len(grp) >= 4:
            centres[j] = smooth(np.median(grp, axis=0))
            centres[j][0] = np.median(grp[:, 0], axis=0)
    arcs = {j: np.concatenate([[0], np.cumsum(np.linalg.norm(np.diff(c, axis=0), axis=1))]) for j, c in centres.items()}
    open_end = set()
    if sleek:
        # the strips' paths every 2 mm past their first 8 mm, with their directions
        pts, tng, own = [], [], []
        for j, c in centres.items():
            a = np.arange(0.008, arcs[j][-1], 0.002)
            g = np.gradient(c, axis=0)
            pts.append(np.stack([np.interp(a, arcs[j], c[:, k]) for k in range(3)], 1)); own += [j] * len(a)
            tng.append(unit(np.stack([np.interp(a, arcs[j], g[:, k]) for k in range(3)], 1)))
        pts, tng, own = np.concatenate(pts), np.concatenate(tng), np.array(own)

        def tree(sel):
            kt = KDTree(len(sel))
            for i in sel:
                kt.insert(tuple(pts[i]), int(i))
            kt.balance()
            return kt
        kc = tree(np.arange(len(pts)))
        # the ponytail's clumps mix strands that sweep across each other at the temples and round the parting:
        # their strips lay across the flow of the strips under them and showed as chopped pieces there (Art,
        # 2026-11-02). On the front half of the head a strip that runs over 14 degrees off its neighbours' flow
        # is dropped; those under it cover the place.
        if style == 'ponytail':
            across = {}
            for j, c in centres.items():
                if c[0, 1] >= C[1]:
                    continue
                angs = []
                for i in np.flatnonzero(own == j):
                    nb = [m for _, m, _ in kc.find_range(tuple(pts[i]), 0.006) if own[m] != j]
                    if len(nb) >= 3:
                        f = tng[nb] * np.sign(tng[nb] @ tng[i])[:, None]
                        angs.append(np.degrees(np.arccos(min(1.0, abs(unit(f.mean(0)) @ tng[i])))))
                if angs and np.median(angs) > 14:
                    across[j] = np.median(angs)
            print('DROPPED across the flow', len(across), 'strips')
            for j, v in across.items():
                print('  %3d deg, root %s' % (v, np.round(centres.pop(j)[0], 3)))
            kc = tree(np.flatnonzero(~np.isin(own, list(across))))
        # strips whose root end lies open on the front of the head, with no other strip passing over it within
        # 6 mm (the hairline, the ponytail's parting): those fade in from the root
        open_end = {j for j, c in centres.items() if c[0, 1] < C[1] - 0.03
                    and all(own[i] == j for _, i, _ in kc.find_range(tuple(c[0]), 0.006))}
        # the ponytail's long strips pass close by each other's roots at the hairline without covering their
        # ends, which still stood as steps at the temples: all its strips on the front half of the head fade in
        if style == 'ponytail':
            open_end |= {j for j, c in centres.items() if c[0, 1] < C[1]}
    axis = smooth(np.median(R, axis=0), 2)   # a tail's strips lie round its own axis, not the skull's centre
    verts, faces, uvs, normals, alphas = [], [], [], [], []
    strips = edges = 0
    for j in range(K):
        grp = R[lab == j]
        if j not in centres:
            continue
        centre = centres[j]
        # braid: as strips the loose face strand turned into a dark block over the eye (Art,
        # 2026-11-01), so no strip hangs in front of the face below the forehead
        if style == 'braid' and suffix == 'cards':
            front = (np.abs(centre[:, 0] - C[0]) < 0.06) & (centre[:, 1] < C[1] - 0.05) & (centre[:, 2] < zt - 0.075)
            if front.sum() >= 2:
                continue
        tan = unit(np.gradient(centre, axis=0))
        out = unit(centre - C)
        if suffix == 'tail':
            ra = centre - axis
            ra = ra - tan * np.einsum('kc,kc->k', ra, tan)[:, None]
            out = unit(np.where(np.linalg.norm(ra, axis=1, keepdims=True) > 0.002, ra, out))
        across = unit(np.cross(tan, out))
        nrm = unit(np.cross(across, tan))
        off = grp - centre[None]
        wa = np.percentile(np.abs(np.einsum('skc,kc->sk', off, across)), 80, axis=0)
        wn = np.percentile(np.abs(np.einsum('skc,kc->sk', off, nrm)), 80, axis=0)
        taper = 0.8 if suffix == 'tail' else {'slick': 0.1, 'ponytail': 0.1, 'medium': 0.75}.get(style, 0.35)   # the medium's fringe and the tail end in points
        half = np.clip(smooth(wa, 2) * 1.3, wmin, wmax) * (1 - taper * t ** 2)
        # blunt strip roots stacked along the hairline read as dark steps: they start narrow
        # (the slick lies flat and every strip root shows on the back of the head: narrow roots and a
        # second layer there stacked into rows of scales, so there only the hairline's strips narrow)
        if not sleek or np.percentile(rdens[lab == j], 20) < 0.55 * rmed:
            # (over a segment and a half; the ponytail's long strips over 12 mm, or the strips meeting at
            # its hairline's centre, where the hair parts both ways, left bare spikes between them)
            arc = np.concatenate([[0], np.cumsum(np.linalg.norm(np.diff(centre, axis=0), axis=1))])
            ramp = arc / 0.012 if style == 'ponytail' else t * SEG / 1.5
            half = half * (0.3 + 0.7 * np.clip(ramp, 0, 1) ** 0.7)
        thick = np.clip(smooth(wn, 2), 0, 0.01)
        n = unit(0.7 * nrm + 0.3 * out) if suffix not in ('plait', 'tail') else nrm
        vtip = 0.8 if sleek else 1.0   # the slick's strips end bluntly, short of the texture's tips
        # the slick's and the ponytail's hairline strips fade in from the root (vertex alpha: the strip shader in
        # look.rs starts each strand of the texture at its own point along it), so their blunt ends stop drawing
        # steps on the forehead (Art, 2026-11-02); the head shader paints the hairline under them (scalp_mask.py)
        arc = np.concatenate([[0], np.cumsum(np.linalg.norm(np.diff(centre, axis=0), axis=1))])
        edge = j in open_end
        if edge:
            fade_paths.append(np.stack([np.interp(np.arange(0, 0.0121, 0.002), arc, centre[:, k]) for k in range(3)], 1))
        # rows along the strip: its SEG + 1, and on a hairline strip more over its first 14 mm, where the fade
        # runs (a segment is some 2 cm: across one, the fade was a blunt cut a millimetre in)
        rows = np.arange(SEG + 1.0)
        if edge:
            rows = np.union1d(rows, np.interp(np.arange(0.002, 0.0141, 0.002), arc, rows))
        at = lambda X: np.stack([np.interp(rows, np.arange(SEG + 1), x) for x in np.reshape(X, (SEG + 1, -1)).T], 1)
        r_centre, r_nrm, r_across, r_n = at(centre), unit(at(nrm)), unit(at(across)), unit(at(n))
        r_half, r_thick, r_t = at(half)[:, 0], at(thick)[:, 0], rows / SEG
        # 0.1 at the root, where even the strongest strands stay under the 0.15 cut, to full 14 mm along
        r_alpha = np.clip(0.1 + 0.9 * (at(arc)[:, 0] / 0.014) ** 1.3, 0, 1) if edge else np.ones(len(rows))
        for layer in ((0,) if sleek else (0, 1)):
            k = 1.0 if layer == 0 else 0.65
            shift = 0.0 if layer == 0 else rng.uniform(-0.35, 0.35)
            lift = (0.0 if layer == 0 else 0.6) * r_thick + 0.0006 * layer
            u0 = rng.uniform(0, 0.5)
            base = len(verts)
            for i in range(len(rows)):
                c = r_centre[i] + r_nrm[i] * lift[i] + r_across[i] * shift * r_half[i]
                for side in (-1, 1):
                    verts.append(tuple(c + r_across[i] * side * r_half[i] * k))
                    uvs.append((u0 + (0.5 + 0.5 * side) * 0.5, r_t[i] * vtip))
                    normals.append(tuple(r_n[i]))
                    alphas.append(r_alpha[i])
            for i in range(len(rows) - 1):
                a = base + 2 * i
                faces.append((a, a + 1, a + 3, a + 2))
            strips += 1
        edges += edge
    name = f'Hair_{style}_{suffix}'
    me = bpy.data.meshes.new(name)
    me.from_pydata(verts, [], faces)
    me.shade_smooth()
    uv = me.uv_layers.new(name='UVMap')
    for poly in me.polygons:
        for li in poly.loop_indices:
            uv.data[li].uv = uvs[me.loops[li].vertex_index]
    me.normals_split_custom_set_from_vertices(normals)
    if sleek:
        col = me.color_attributes.new(name='Color', type='FLOAT_COLOR', domain='POINT')
        col.data.foreach_set('color', np.array([(1, 1, 1, a) for a in alphas], np.float32).ravel())
    me.update()
    ob = bpy.data.objects.new(name, me)
    out_objs.append(ob)
    if suffix == 'cards':
        # the roots and their first steps for the hairline mask (scalp_mask.py), as directions from the skull's centre
        np.savez(out_p[:-len('.blend')] + '_roots.npz', roots=R[:, 0], dirs=R[:, 1] - R[:, 0], centre=C)
    print('CARDS', name, 'strands', len(R), 'strips', strips, 'fading in', edges, 'tris', 2 * len(faces))

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
# the slick's and the ponytail's hairline was a layer of the fade's dashes over the whole cap: its sparse end
# scattered dots on the forehead and its edge read as a brown band, so the head shader paints it now (scalp_mask.py)
dense = dens >= 0.5 * np.median(dens)
# and it stays out from under the first 12 mm of the front strips that fade in, where its flat edge showed through
if fade_paths:
    ke = np.concatenate(fade_paths); kt = KDTree(len(ke))
    for i, r in enumerate(ke):
        kt.insert(tuple(r), i)
    kt.balance()
    dense &= np.array([kt.find(tuple(v))[2] > 0.007 for v in cv])
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
# move out by that much plus a millimetre, fading out over 3 cm away from the cap. That clears the
# largest skull and left the hair standing off the others (m1 under the ponytail, Art 2026-11-02),
# so each face also gets a shape key of its own fit, out or in to a millimetre over its own skull
# (look.rs picks the chosen face's); the base shape stays the fit that clears all four.
from mathutils.kdtree import KDTree
if heads_p:
    with bpy.data.libraries.load(heads_p) as (src, dst):
        assert all(f in src.objects for f in FACES), heads_p
        dst.objects = list(FACES)
    face_objs = dict(zip(FACES, dst.objects))   # loaded as Head_m1.001 and so on beside the groom's own
else:
    face_objs = {f: bpy.data.objects[f] for f in FACES}
heads = []
for face in FACES:
    o = face_objs[face]
    M = np.array(o.matrix_world)
    V = np.array([v.co for v in o.data.vertices]) @ M[:3, :3].T + M[:3, 3]
    heads.append((face, BVHTree.FromPolygons([tuple(v) for v in V], [tuple(p.vertices) for p in o.data.polygons])))


def outer(bvh, d):
    hit = bvh.ray_cast(tuple(C + d * 0.4), tuple(-d))
    return None if hit[0] is None else np.linalg.norm(np.array(hit[0]) - C)


dirs = unit(cv - C); rc = np.linalg.norm(cv - C, axis=1)
delta = np.zeros(len(cv))
fits = {}
for face, bvh in heads:
    r = np.array([outer(bvh, d) or 0.0 for d in dirs])
    need = np.where(r > 0, r + 0.001 - rc, 0.0)
    need[need > 0.014] = 0  # an ear standing out, not the skull
    fits[face] = np.maximum(need, -0.003)
    print('CLEARANCE', face, 'cap points under skin', int((r > rc + 0.0002).sum()), 'max mm', round(1000 * need.max(), 1),
          'own fit in, max mm', round(-1000 * fits[face].min(), 1))
    delta = np.maximum(delta, need)
kd = KDTree(len(cv))
for i, v in enumerate(cv):
    kd.insert(tuple(v), i)
kd.balance()


def near_cap(P):
    """The 8 nearest cap points of each vertex, their weights, and the fade with distance from the cap."""
    idx = np.zeros((len(P), 8), int); w = np.zeros((len(P), 8)); fade = np.zeros(len(P))
    for i, p in enumerate(P):
        near = kd.find_n(tuple(p), 8)
        idx[i] = [n[1] for n in near]; w[i] = [1 / (n[2] + 0.002) for n in near]
        fade[i] = np.clip(1 - (near[0][2] - 0.01) / 0.03, 0, 1)
    return idx, w / w.sum(1, keepdims=True), fade


def push(P, nb, dl):
    idx, w, fade = nb
    return P + unit(P - C) * ((w * dl[idx]).sum(1) * fade)[:, None]


for o in out_objs:
    P = np.array([v.co for v in o.data.vertices])
    cap_like = o.name.endswith('_cap')
    nb = None if cap_like else near_cap(P)
    fit = (lambda dl: P + dirs * dl[:, None]) if cap_like else (lambda dl: push(P, nb, dl))
    o.data.vertices.foreach_set('co', fit(delta).astype(np.float32).ravel())
    o.data.update()
    o.shape_key_add(name='Basis')
    for face in FACES:
        o.shape_key_add(name=face, from_mix=False).data.foreach_set('co', fit(fits[face]).astype(np.float32).ravel())
print('PUSHED out, max mm', round(1000 * delta.max(), 1), '; a shape key per face')

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
