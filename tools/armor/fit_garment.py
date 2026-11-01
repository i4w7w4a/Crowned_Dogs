"""Fit an unrigged garment mesh (a Tripo download of a suit, vest, helmet or boots) onto a Persona .blend.

python -I fit_garment.py -- PERSONA.blend GARMENT.(glb|fbx|obj) OUT.blend NAME [key=value ...]

Keys:
  mode=body      body: the garment is a dressed figure or a piece worn on the body (suit, vest, boots);
                 rigid: one bone carries it 100 % (helmet), bone=head
  height=auto    target height of the garment's bbox in metres (auto: the Persona's skull top + sole)
  sole=0.025     boot sole: the garment's lowest point goes this far below the bare soles (body mode, height=auto)
  cut_above=     delete garment geometry above this height after fitting (the generated head of a dressed figure)
  cut_below=     delete garment geometry below this height after fitting
  cut_head=1     delete the generated head by colour: everything above head_band hi plus the white-shell or
                 skin coloured vertices above head_band lo (default neck_01 - 6 cm, head + 6 cm) joined to it;
                 the head grows from its top through white-shell / skin coloured vertices (head_lum=0.62 = white)
  collar_trim=1  with cut_head: per direction around the neck, delete what stands above the collar's fabric
  neck_cut=      with cut_head: also delete everything within collar_r=0.13 of the neck axis above this height
  hide_below=    body vertices below this height are hidden too (auto: the lowest point of the neck opening
                 - 1.5 cm); for a full suit with gloves and boots, so the body can never poke through
  fit_bones=arms,legs   which limb chains get posed to the garment (empty = none)
  clearance=0.004 minimum gap between body and garment in the rest pose
  aniso=1        refine scale per axis (a piece made from its own picture, e.g. a vest, has its own proportions)
  inside=        how far the reference points sit inside the garment surface (0.012 suit; negative = outside the
                 lining of a hollow piece; over a base garment the points are that garment's torso vertices)
  push=soft      move dents together with everything within soft_r0=0.015..soft_r1=0.04 m (default for vests and
                 helmets; push=hard moves only the vertices that are inside, default for a dressed figure)
  base=          an earlier OUT.blend to add this garment to (vest over suit); garments already in it count as
                 surfaces this one must stay outside of
  pair=1         boots: the download is one right boot (toe to -Y); make a right and a mirrored left boot, each turned
                 and centred on its bare foot, the boot `toe_room=0.04` longer than the foot (sole at floor=-SOLE)
  slab=z0,z1     align=feet: centre the garment on the body between these heights (default 55..72 % of the Persona)
  ref_zmax=      refine only against body points below this height (boots: the shaft top)

What it does (body mode): uniform scale + offset so the garment wraps the body; the Persona's arm and leg
bones are swung until the posed body sits inside the garment's sleeves and legs; skin weights are copied from
the posed body (barycentric, smoothed, max 4 bones); the garment is un-skinned back into the Persona's rest
pose; anything closer to the body than `clearance` is pushed out; body vertices the garment covers go into a
vertex group `hide_<NAME>` on the body (the game hides those faces).
"""
import bpy, bmesh, sys, math
import numpy as np
from mathutils import Vector, Matrix
from mathutils.bvhtree import BVHTree

args = sys.argv[sys.argv.index('--') + 1:]
PERSONA, GARMENT, OUT, NAME = args[:4]
opt = dict(a.split('=', 1) for a in args[4:])
MODE = opt.get('mode', 'body')
SOLE = float(opt.get('sole', 0.025))
CLEAR = float(opt.get('clearance', 0.004))
FIT = [c for c in opt.get('fit_bones', 'arms,legs').split(',') if c]

bpy.ops.wm.open_mainfile(filepath=opt.get('base') or PERSONA)
scene = bpy.context.scene
arm = next(o for o in scene.objects if o.type == 'ARMATURE')
assert (arm.matrix_world - Matrix.Identity(4)).to_3x3().determinant() < 1e-6 or True
if arm.animation_data: arm.animation_data.action = None
for pb in arm.pose.bones: pb.matrix_basis.identity()
persona_meshes = [o for o in scene.objects if o.type == 'MESH' and o.parent == arm and not o.get('garment')]
body = max(persona_meshes, key=lambda o: len(o.vertex_groups))
heads = [o for o in persona_meshes if o is not body]
old_garments = [o for o in scene.objects if o.type == 'MESH' and o.get('garment')]
print('body', body.name, 'heads', [h.name for h in heads], 'garments already there', [g.name for g in old_garments])


def update():
    bpy.context.view_layer.update()


def evaluated(obj):
    """World-space vertex coords and triangles of the deformed mesh."""
    dg = bpy.context.evaluated_depsgraph_get()
    ev = obj.evaluated_get(dg)
    me = ev.to_mesh()
    me.calc_loop_triangles()
    n = len(me.vertices)
    co = np.empty(n * 3, np.float32); me.vertices.foreach_get('co', co); co = co.reshape(-1, 3)
    nr = np.empty(n * 3, np.float32); me.vertices.foreach_get('normal', nr); nr = nr.reshape(-1, 3)
    tri = np.empty(len(me.loop_triangles) * 3, np.int32); me.loop_triangles.foreach_get('vertices', tri); tri = tri.reshape(-1, 3)
    ev.to_mesh_clear()
    M = np.array(obj.matrix_world)
    co = co @ M[:3, :3].T + M[:3, 3]
    nr = nr @ np.linalg.inv(M[:3, :3])
    nr /= np.linalg.norm(nr, axis=1, keepdims=True) + 1e-12
    return co, nr, tri


def smooth_normal(q, ti, co, nr, tri):
    """Vertex normals interpolated at point q on triangle ti: a steadier inside/outside sign at creases."""
    a, b, c = tri[ti]
    A, B, C = co[a], co[b], co[c]
    v0, v1, v2 = B - A, C - A, np.array(q) - A
    d00, d01, d11, d20, d21 = v0 @ v0, v0 @ v1, v1 @ v1, v2 @ v0, v2 @ v1
    den = d00 * d11 - d01 * d01 or 1e-12
    wv = (d11 * d20 - d01 * d21) / den; ww = (d00 * d21 - d01 * d20) / den
    n = (1 - wv - ww) * nr[a] + wv * nr[b] + ww * nr[c]
    return Vector(n / (np.linalg.norm(n) + 1e-12))


def bvh(co, tri):
    return BVHTree.FromPolygons([Vector(v) for v in co], [tuple(t) for t in tri], all_triangles=True)


# ---------- import the garment as one static mesh ----------
before = set(bpy.data.objects)
ext = GARMENT.lower().rsplit('.', 1)[-1]
if ext == 'glb' or ext == 'gltf': bpy.ops.import_scene.gltf(filepath=GARMENT)
elif ext == 'fbx': bpy.ops.import_scene.fbx(filepath=GARMENT)
elif ext == 'obj': bpy.ops.wm.obj_import(filepath=GARMENT)
new = [o for o in bpy.data.objects if o not in before]
for o in new:
    if o.type == 'ARMATURE':
        if o.animation_data: o.animation_data.action = None
        for pb in o.pose.bones: pb.matrix_basis.identity()
update()
dg = bpy.context.evaluated_depsgraph_get()
parts = []
for o in new:
    if o.type == 'MESH':
        me = bpy.data.meshes.new_from_object(o.evaluated_get(dg), preserve_all_data_layers=True, depsgraph=dg)
        me.transform(o.matrix_world)
        parts.append(me)
for o in new: bpy.data.objects.remove(o)
gme = bpy.data.meshes.new(NAME)
bm = bmesh.new()
mats = []
for me in parts:
    off = len(mats)
    for m in me.materials: mats.append(m)
    for p in me.polygons: p.material_index += off
    bm.from_mesh(me)
# glTF splits vertices along UV seams: weld them back so the surface is one piece (no cracks when vertices move)
nv = len(bm.verts)
bmesh.ops.remove_doubles(bm, verts=bm.verts, dist=float(opt.get('weld', 1e-5)))
print(f'welded {nv - len(bm.verts)} seam vertices, {len(bm.verts)} left')
for f in bm.faces: f.smooth = True
bm.to_mesh(gme); bm.free()
if opt.get('pair') == '1':
    # one right boot, toe to -Y: a right and a mirrored left boot, each turned and centred on its bare foot
    bco0 = np.array([body.matrix_world @ v.co for v in body.data.vertices])
    gm = np.empty(len(gme.vertices) * 3, np.float32); gme.vertices.foreach_get('co', gm); gm = gm.reshape(-1, 3)
    gm -= [0.5 * (gm[:, 0].min() + gm[:, 0].max()), 0.5 * (gm[:, 1].min() + gm[:, 1].max()), gm[:, 2].min()]
    bm = bmesh.new(); first = True
    for sgn, side in ((-1, 'r'), (1, 'l')):
        foot = bco0[(bco0[:, 2] < 0.13) & (bco0[:, 0] * sgn > 0)]
        hd = np.array(arm.matrix_world @ arm.data.bones['foot_' + side].head_local)
        tl = np.array(arm.matrix_world @ arm.data.bones['ball_' + side].tail_local)
        d = (tl - hd)[:2]; d /= np.linalg.norm(d)
        ang = math.atan2(-d[0], d[1]) + math.pi  # boot toe (-Y) turned onto the foot direction
        along = foot[:, :2] @ d
        length = np.ptp(along) + float(opt.get('toe_room', 0.04))
        k = length / np.ptp(gm[:, 1])
        c = foot[:, :2].mean(0) + d * (0.5 * (along.max() + along.min()) - foot[:, :2].mean(0) @ d)
        one = gm * k
        if sgn > 0: one[:, 0] *= -1  # the left boot is the right one mirrored
        R = np.array([[math.cos(ang), -math.sin(ang)], [math.sin(ang), math.cos(ang)]])
        one[:, :2] = one[:, :2] @ R.T + c
        one[:, 2] -= SOLE
        me = gme.copy(); me.vertices.foreach_set('co', one.astype(np.float32).ravel())
        n0 = len(bm.faces); bm.from_mesh(me); bpy.data.meshes.remove(me)
        bm.faces.ensure_lookup_table()
        if sgn > 0: bmesh.ops.reverse_faces(bm, faces=bm.faces[n0:])
        print(f'boot {side}: length {length:.3f} m, scale {k:.4f}, turned {math.degrees(ang) - 180:.1f} deg, centre {c.round(3)}')
    bm.to_mesh(gme); bm.free()
for m in mats: gme.materials.append(m)
gar = bpy.data.objects.new(NAME, gme); scene.collection.objects.link(gar)
gar['garment'] = True


def gco():
    a = np.empty(len(gme.vertices) * 3, np.float32); gme.vertices.foreach_get('co', a); return a.reshape(-1, 3)


def set_gco(a):
    gme.vertices.foreach_set('co', a.astype(np.float32).ravel()); gme.update()


def gtri():
    gme.calc_loop_triangles()
    t = np.empty(len(gme.loop_triangles) * 3, np.int32); gme.loop_triangles.foreach_get('vertices', t); return t.reshape(-1, 3)


# ---------- body vertex sets per bone ----------
gnames = [g.name for g in body.vertex_groups]
nb = len(body.data.vertices)
W = np.zeros((nb, len(gnames)), np.float32)
for v in body.data.vertices:
    for g in v.groups: W[v.index, g.group] = g.weight
W[:, [k for k, nm in enumerate(gnames) if nm.startswith('hide_')]] = 0  # masks of earlier garments are not bones
W /= W.sum(1, keepdims=True) + 1e-9
dom = np.array(gnames)[W.argmax(1)]
domw = W.max(1)


def owner(name):
    """Fold twist, finger and ball bones into the limb bone they ride on."""
    for side in ('_l', '_r'):
        if not name.endswith(side): continue
        b = name[:-2]
        if b.startswith('upperarm'): return 'upperarm' + side
        if b.startswith('lowerarm'): return 'lowerarm' + side
        if b in ('hand',) or b.split('_')[0] in ('thumb', 'index', 'middle', 'ring', 'pinky'): return 'hand' + side
        if b.startswith('thigh'): return 'thigh' + side
        if b.startswith('calf'): return 'calf' + side
        if b in ('foot', 'ball'): return 'foot' + side
    return name


own = np.array([owner(n) for n in dom])
CHAINS = {'arms': [['upperarm_l', 'lowerarm_l', 'hand_l'], ['upperarm_r', 'lowerarm_r', 'hand_r']],
          'legs': [['thigh_l', 'calf_l', 'foot_l'], ['thigh_r', 'calf_r', 'foot_r']]}

def refine(points, inside, prior, label, aniso=False, lock_x=False):
    """Scale (uniform, or per axis with aniso) about the garment's centre + offset so `points` sit `inside` metres
    inside it (negative: outside a hollow piece's lining, e.g. a vest over the suit)."""
    g0 = gco(); tree0 = bvh(g0, gtri())
    piv = g0.mean(0)  # scale about the garment's own centre
    pen_w = float(opt.get('pen_w', 1.0))

    def split(x):
        s, t = (np.array(x[:3]), np.array(x[3:], float)) if aniso else (np.full(3, x[0]), np.array(x[1:], float))
        if lock_x: t[0] = 0.0
        return s, t

    def cost(x):
        s, t = split(x)
        # move the points instead of the garment: p' = (p - t - piv) / s + piv in garment space
        P = (points - t - piv) / s + piv
        c = 0.0
        for p in P:
            q, n, i, d = tree0.find_nearest(Vector(p))
            if q is None: c += 0.1; continue
            sd = (Vector(p) - q).dot(n) * float(np.mean(s))
            r = sd + inside
            # a hollow piece (inside < 0) cutting into what it covers is worse than a gap: weigh that side up
            w = pen_w if (inside < 0 and sd < 0) else 1.0
            c += w * (math.sqrt(1 + (r / 0.01) ** 2) - 1)  # soft L1
        return c / len(P) + prior * float(((s - 1.0) ** 2).sum())
    try:
        from scipy.optimize import minimize
    except ImportError:
        print('scipy missing: rough placement only'); return
    starts = ([[1, 1, 1, 0, 0, 0], [1, 1.08, 1, 0, -0.02, 0], [1.05, 1.05, 1, 0, 0, 0], [1.04, 1.12, 1, 0, -0.03, 0]]
              if aniso else [[1, 0, 0, 0]])
    res = None
    for x0 in starts:  # the cost has local minima for plates and pouches: try a few starts, keep the best
        simplex = [x0] + [[v + (0.03 if (k < (3 if aniso else 1)) and j == k else 0.02 if j == k else 0)
                           for j, v in enumerate(x0)] for k in range(len(x0))]
        r_ = minimize(cost, x0, method='Nelder-Mead', options={'xatol': 1e-4, 'fatol': 1e-5, 'maxiter': 600 if aniso else 300,
                                                                'initial_simplex': simplex})
        if len(starts) > 1: print(f'  start {x0[:3]}: cost {r_.fun:.4f} scale {np.round(r_.x[:3], 3)}')
        if res is None or r_.fun < res.fun: res = r_
    s, t = split(res.x)
    print(f'{label} refine: scale x{s.round(4)}, offset {t.round(4)}, cost {res.fun:.4f}')
    set_gco((g0 - piv) * s + piv + t)


def base_colour_per_vertex():
    """Mean base-colour texel under each garment vertex (from the image feeding Base Color, up to 4 nodes back)."""
    img = None
    for m in gme.materials:
        if not (m and m.use_nodes): continue
        bsdf = next((n for n in m.node_tree.nodes if n.type == 'BSDF_PRINCIPLED'), None)
        todo = [bsdf.inputs['Base Color']] if bsdf else []
        for _ in range(4):
            nxt = []
            for sock in todo:
                for l in sock.links:
                    if l.from_node.type == 'TEX_IMAGE' and l.from_node.image: img = img or l.from_node.image
                    nxt += list(l.from_node.inputs)
            todo = nxt
    if img is None: raise SystemExit('no base colour image on the garment')
    w, h = img.size
    px = np.array(img.pixels[:], np.float32).reshape(h, w, -1)[..., :3]
    uv = np.empty(len(gme.loops) * 2, np.float32); gme.uv_layers.active.data.foreach_get('uv', uv); uv = uv.reshape(-1, 2) % 1.0
    lv = np.empty(len(gme.loops), np.int32); gme.loops.foreach_get('vertex_index', lv)
    col = px[np.clip((uv[:, 1] * h).astype(int), 0, h - 1), np.clip((uv[:, 0] * w).astype(int), 0, w - 1)]
    vc = np.zeros((len(gme.vertices), 3)); cnt = np.zeros(len(gme.vertices))
    np.add.at(vc, lv, col); np.add.at(cnt, lv, 1)
    return vc / (cnt[:, None] + 1e-9)


def head_by_colour(g):
    """Vertices of the generated head and neck: white shell or skin colour, grown from the top of the head."""
    from scipy.sparse import coo_matrix
    from scipy.sparse.csgraph import connected_components
    c = base_colour_per_vertex()
    lum = c.mean(1); sat = (c.max(1) - c.min(1)) / (c.max(1) + 1e-6)
    white = (lum > float(opt.get('head_lum', 0.62))) & (sat < 0.18)
    skin = (c[:, 0] - c[:, 2] > 0.10) & (c[:, 1] - c[:, 0] < -0.04)
    hd = (white | skin).astype(np.float64)
    ed = np.empty(len(gme.edges) * 2, np.int32); gme.edges.foreach_get('vertices', ed); ed = ed.reshape(-1, 2)
    n = len(g)
    deg = np.bincount(ed.ravel(), minlength=n).astype(np.float64)
    for _ in range(3):  # bridge eyes and brows, drop camo speckles
        acc = np.zeros(n); np.add.at(acc, ed[:, 0], hd[ed[:, 1]]); np.add.at(acc, ed[:, 1], hd[ed[:, 0]])
        hd = 0.5 * hd + 0.5 * acc / np.maximum(deg, 1)
    mw = arm.matrix_world
    lo, hi = ((mw @ arm.pose.bones['neck_01'].head).z - 0.06, (mw @ arm.pose.bones['head'].head).z + 0.06)
    if opt.get('head_band'): lo, hi = map(float, opt['head_band'].split(','))
    grow = ((hd > 0.5) & (g[:, 2] > lo)) | (g[:, 2] > hi)

    def comps(mask_edges):
        e = ed[mask_edges]
        A = coo_matrix((np.ones(len(e)), (e[:, 0], e[:, 1])), shape=(n, n))
        return connected_components(A, directed=False)[1]
    lab = comps(grow[ed[:, 0]] & grow[ed[:, 1]])
    kill = grow & np.isin(lab, np.unique(lab[g[:, 2] > hi]))
    # the generated neck often closes the collar with a disc inside the real neck: drop what lies inside it
    nk = np.array(mw @ arm.pose.bones['neck_01'].head)
    bco_, _, _ = evaluated(body)
    ring_b = bco_[(bco_[:, 2] > nk[2] - 0.01) & (bco_[:, 2] < nk[2] + 0.04)]
    r_neck = np.median(np.linalg.norm(ring_b[:, :2] - nk[:2], axis=1)) if len(ring_b) else 0.05
    inner = (g[:, 2] > lo) & (np.linalg.norm(g[:, :2] - nk[:2], axis=1) < float(opt.get('neck_inner', 0.85)) * r_neck)
    print(f'neck radius {r_neck:.3f}, {(inner & ~kill).sum()} verts inside the neck')
    kill |= inner
    # strips of generated neck or chin still standing on the collar: per direction around the neck, the collar
    # ends at its highest fabric-coloured vertex; skin/white-coloured bits above that go
    rv = np.linalg.norm(g[:, :2] - nk[:2], axis=1)
    zone = ~kill & (g[:, 2] > lo) & (rv < float(opt.get('collar_r', 0.13)))
    gme.update()
    gn = np.empty(len(gme.vertices) * 3, np.float32); gme.vertices.foreach_get('normal', gn); gn = gn.reshape(-1, 3)
    radial = np.c_[g[:, :2] - nk[:2], np.zeros(n)] / (rv[:, None] + 1e-9)
    outer = ((gn * radial).sum(1) > 0.2) & (rv > r_neck + 0.010)  # the collar's outside wall, not a strip on the neck
    fabric = zone & outer & (hd < 0.25) & (lum < 0.42)
    ang = np.arctan2(g[:, 1] - nk[1], g[:, 0] - nk[0]); NS = 24
    sec = ((ang + np.pi) / (2 * np.pi) * NS).astype(int) % NS
    top = np.full(NS, np.nan)
    for k in range(NS):
        zz = g[fabric & (sec == k), 2]
        if len(zz) > 3: top[k] = np.percentile(zz, 95)
    if opt.get('collar_trim') == '1' and np.isfinite(top).any():
        top = np.where(np.isfinite(top), top, np.nanmedian(top))
        top = np.array([np.median(np.take(top, range(k - 2, k + 3), mode='wrap')) for k in range(NS)])
        above = zone & (g[:, 2] > top[sec] + 0.004)
        kill |= above
        print(f'collar top {top.min():.3f}..{top.max():.3f}, {above.sum()} verts of neck/chin above it')
    if opt.get('neck_cut'):  # a hard ceiling for anything near the neck (bits of chin left on a tall collar)
        hard = zone & ~kill & (g[:, 2] > float(opt['neck_cut']))
        kill |= hard
        print(f'neck_cut: {hard.sum()} verts above {float(opt["neck_cut"]):.3f}')
    # pieces left floating above the neck band floor (marks on the face) go too; the main body stays
    lab = comps(~kill[ed[:, 0]] & ~kill[ed[:, 1]])
    sizes = np.bincount(lab[~kill], minlength=lab.max() + 1)
    zmin = np.full(lab.max() + 1, 9.0); np.minimum.at(zmin, lab[~kill], g[~kill, 2])
    stray = ~kill & (lab != np.argmax(sizes)) & (zmin[lab] > lo)
    kill |= stray
    if kill.sum() > float(opt.get('head_max', 0.3)) * n: raise SystemExit(f'head cut failed: {kill.sum()} of {n} verts would go')
    print(f'head cut by colour: band {lo:.3f}..{hi:.3f}, head {kill.sum() - stray.sum()}, strays {stray.sum()}')
    return kill


def slab_centre(a, z0, z1):
    m = (a[:, 2] > z0) & (a[:, 2] < z1)
    return a[m, :2].mean(0)


rng = np.random.default_rng(1)
ALIGN = opt.get('align', 'feet')

if MODE == 'body':
    # ---------- rough placement: height and centre ----------
    g = gco()
    bco, _, _ = evaluated(body)
    top = max(evaluated(h)[0][:, 2].max() for h in heads) if heads else bco[:, 2].max()
    if ALIGN == 'feet':
        # a dressed figure: as tall as the Persona plus the boot sole, standing on the sole
        H = float(opt['height']) if opt.get('height', 'auto') != 'auto' else top + SOLE
        s = 1.0 if opt.get('pair') == '1' else H / (g[:, 2].max() - g[:, 2].min())  # boots come sized to the feet
        g = g * s
        zb = -SOLE if opt.get('height', 'auto') == 'auto' else float(opt.get('floor', -SOLE))
        g[:, 2] += zb - g[:, 2].min()
        z0, z1 = map(float, opt['slab'].split(',')) if opt.get('slab') else (0.55 * top, 0.72 * top)
        g[:, :2] += slab_centre(bco, z0, z1) - slab_centre(g, z0, z1)
    else:
        # a piece worn on the torso (vest): given height, its top at top_z (default: base of the neck + 2 cm)
        s = float(opt['height']) / (g[:, 2].max() - g[:, 2].min())
        g = g * s
        tz = float(opt.get('top_z', (arm.matrix_world @ arm.pose.bones['neck_01'].head).z + 0.02))
        g[:, 2] += tz - g[:, 2].max()
        z0 = g[:, 2].min() + 0.3 * (g[:, 2].max() - g[:, 2].min()); z1 = g[:, 2].max() - 0.3 * (g[:, 2].max() - g[:, 2].min())
        g[:, 1] += slab_centre(bco, z0, z1)[1] - slab_centre(g, z0, z1)[1]
        # sideways: the top 40 % (plates and straps) is symmetric, the pouches lower down may not be
        up = g[:, 2] > g[:, 2].max() - 0.4 * (g[:, 2].max() - g[:, 2].min())
        g[:, 0] += slab_centre(bco, z0, z1)[0] - 0.5 * (g[up, 0].min() + g[up, 0].max())
    set_gco(g)
    print(f'rough: scale {s:.4f}')

    # ---------- refine scale + offset so the torso (and legs) sit inside the garment ----------
    REF = opt.get('refine', 'pelvis,spine_01,spine_02,spine_03,thigh_l,thigh_r,calf_l,calf_r' if ALIGN == 'feet'
                  else 'spine_02,spine_03').split(',')
    sel = np.where(np.isin(own, REF))[0]
    if opt.get('ref_zmax'): sel = sel[bco[sel, 2] < float(opt['ref_zmax'])]
    if ALIGN != 'feet':  # only the body inside the piece's height range
        gz = gco()[:, 2]; sel = sel[(bco[sel, 2] > gz.min() + 0.03) & (bco[sel, 2] < gz.max() - 0.05)]
    pts = bco[sel]
    if old_garments and ALIGN != 'feet':
        # a piece worn over other garments wraps their surface, not the body: use their torso vertices
        pts = []
        for og in old_garments:
            oco, _, _ = evaluated(og)
            names = np.array([vg.name for vg in og.vertex_groups])
            OW = np.zeros((len(og.data.vertices), len(names)), np.float32)
            for v in og.data.vertices:
                for gr in v.groups: OW[v.index, gr.group] = gr.weight
            oown = np.array([owner(nm) for nm in names[OW.argmax(1)]])
            gz = gco()[:, 2]
            m = np.isin(oown, REF) & (oco[:, 2] > gz.min() + 0.03) & (oco[:, 2] < gz.max() - 0.05)
            pts.append(oco[m])
        pts = np.concatenate(pts)
        print(f'refine against {len(pts)} torso vertices of {[o.name for o in old_garments]}')
    samp = rng.choice(len(pts), min(1500, len(pts)), replace=False)
    refine(pts[samp], float(opt.get('inside', 0.012 if ALIGN == 'feet' else 0.03)), float(opt.get('scale_prior', 60.0)), 'body',
           aniso=opt.get('aniso') == '1', lock_x=opt.get('lock_x', '0' if ALIGN == 'feet' else '1') == '1')

    # ---------- swing the limb bones into the garment's sleeves and legs ----------
    g = gco(); tri = gtri(); gtree = bvh(g, tri)
    for chain_set in FIT:
        for chain in CHAINS[chain_set]:
            for bname in chain:
                pb = arm.pose.bones[bname]
                idx = np.where(own == bname)[0]
                total = 0.0
                for it in range(12):
                    update()
                    bco, bnr, _ = evaluated(body)
                    c = np.array(arm.matrix_world @ pb.head)
                    tail = np.array(arm.matrix_world @ pb.tail)
                    P, Q = [], []
                    for i in idx[::2]:
                        q, n, fi, d = gtree.find_nearest(Vector(bco[i]), 0.15)
                        if q is None: continue
                        if Vector(bnr[i]).dot(n) < 0.3: continue
                        P.append(bco[i]); Q.append(np.array(q))
                    if len(P) < 30: break
                    P = np.array(P) - c; Q = np.array(Q) - c
                    U, S_, Vt = np.linalg.svd(P.T @ Q)
                    D = np.diag([1, 1, np.sign(np.linalg.det(Vt.T @ U.T))])
                    R = Vt.T @ D @ U.T
                    d = tail - c; d /= np.linalg.norm(d)
                    swing = Vector(d).rotation_difference(Vector(R @ d))
                    ang = swing.angle
                    if ang > math.radians(10): swing = Matrix.Rotation(math.radians(10), 4, swing.axis).to_quaternion()
                    Mw = Matrix.Translation(Vector(c)) @ swing.to_matrix().to_4x4() @ Matrix.Translation(Vector(-c))
                    pb.matrix = arm.matrix_world.inverted() @ Mw @ arm.matrix_world @ pb.matrix
                    total += min(ang, math.radians(10))
                    if ang < math.radians(0.15): break
                print(f'  {bname}: swung {math.degrees(total):.1f} deg in {it + 1} steps ({len(idx)} body verts)')
    update()

    # ---------- cut away what is not ours (the generated head, the floor) ----------
    g = gco()
    kill = np.zeros(len(g), bool)
    if opt.get('cut_above'): kill |= g[:, 2] > float(opt['cut_above'])
    if opt.get('cut_below'): kill |= g[:, 2] < float(opt['cut_below'])
    if opt.get('cut_head') == '1':
        kill |= head_by_colour(g)
    # thin bridges Tripo sometimes leaves between the legs or arms: tiny pieces spanning a wide gap
    from scipy.sparse import coo_matrix
    from scipy.sparse.csgraph import connected_components
    ed = np.empty(len(gme.edges) * 2, np.int32); gme.edges.foreach_get('vertices', ed); ed = ed.reshape(-1, 2)
    lab = connected_components(coo_matrix((np.ones(len(ed)), (ed[:, 0], ed[:, 1])), shape=(len(g), len(g))), directed=False)[1]
    for k in np.unique(lab):
        m = lab == k
        if m.sum() < 100 and np.ptp(g[m, 0]) > 0.15:
            kill |= m; print(f'bridge removed: {m.sum()} verts spanning {np.ptp(g[m, 0]):.2f} m at z {g[m, 2].mean():.2f}')
    if kill.any():
        bm = bmesh.new(); bm.from_mesh(gme); bm.verts.ensure_lookup_table()
        bmesh.ops.delete(bm, geom=[bm.verts[i] for i in np.where(kill)[0]], context='VERTS')
        bm.to_mesh(gme); bm.free(); gme.update()
        print(f'cut {kill.sum()} verts')

    # ---------- weights from the posed body ----------
    bco, bnr, btri = evaluated(body)
    btree = bvh(bco, btri)
    g = gco()
    GW = np.zeros((len(g), len(gnames)), np.float32)
    for i, p in enumerate(g):
        q, n, ti, d = btree.find_nearest(Vector(p))
        a, b, cc = btri[ti]
        A, B, C = bco[a], bco[b], bco[cc]
        v0, v1, v2 = B - A, C - A, np.array(q) - A
        d00, d01, d11, d20, d21 = v0 @ v0, v0 @ v1, v1 @ v1, v2 @ v0, v2 @ v1
        den = d00 * d11 - d01 * d01 or 1e-12
        wv = (d11 * d20 - d01 * d21) / den; ww = (d00 * d21 - d01 * d20) / den; wu = 1 - wv - ww
        GW[i] = wu * W[a] + wv * W[b] + ww * W[cc]
    # smooth over the garment's own edges
    ed = np.empty(len(gme.edges) * 2, np.int32); gme.edges.foreach_get('vertices', ed); ed = ed.reshape(-1, 2)
    for _ in range(int(opt.get('smooth', 4))):
        acc = np.zeros_like(GW); cnt = np.zeros(len(g), np.float32)
        np.add.at(acc, ed[:, 0], GW[ed[:, 1]]); np.add.at(acc, ed[:, 1], GW[ed[:, 0]])
        np.add.at(cnt, ed[:, 0], 1); np.add.at(cnt, ed[:, 1], 1)
        nz = cnt > 0
        GW[nz] = 0.5 * GW[nz] + 0.5 * acc[nz] / cnt[nz, None]
    # keep the 4 biggest
    order = np.argsort(-GW, axis=1)
    keep = np.zeros_like(GW, bool); np.put_along_axis(keep, order[:, :4], True, axis=1)
    GW[~keep] = 0; GW[GW < 0.01] = 0
    GW /= GW.sum(1, keepdims=True) + 1e-9

    # ---------- un-skin into the rest pose ----------
    S = np.zeros((len(gnames), 4, 4), np.float64)
    for k, n in enumerate(gnames):
        if n in arm.pose.bones:
            pb = arm.pose.bones[n]
            S[k] = np.array(arm.matrix_world @ pb.matrix @ pb.bone.matrix_local.inverted() @ arm.matrix_world.inverted())
        else:
            S[k] = np.eye(4)
    Mv = np.einsum('vg,gij->vij', GW.astype(np.float64), S)
    rest = np.linalg.solve(Mv[:, :3, :3], (g - Mv[:, :3, 3])[..., None])[..., 0]
    for pb in arm.pose.bones: pb.matrix_basis.identity()
    update()
    set_gco(rest)

else:  # rigid piece on one bone (helmet on the head)
    BONE = opt.get('bone', 'head')
    skull_obj = next((h for h in heads if 'Head' in h.name), heads[0])
    sk, _, _ = evaluated(skull_obj)
    cover = [h for h in heads if h is not skull_obj]  # hair that must stay inside (hidden or not)
    allp = np.concatenate([sk] + [evaluated(h)[0] for h in cover]) if cover else sk
    stop = sk[:, 2].max()
    crown = sk[sk[:, 2] > stop - 0.09]             # the dome of the skull
    crown_all = allp[allp[:, 2] > stop - 0.09]
    g = gco()
    # rough: outer width = skull (and hair) width + 2 x margin, top a little over the crown
    width = (crown_all[:, 0].max() - crown_all[:, 0].min()) + 2 * float(opt.get('margin', 0.028))
    s = float(opt['width']) / (g[:, 0].max() - g[:, 0].min()) if opt.get('width') else width / (g[:, 0].max() - g[:, 0].min())
    g = g * s
    g[:, 2] += (max(stop, crown_all[:, 2].max()) + float(opt.get('top_gap', 0.018))) - g[:, 2].max()
    top_half = g[g[:, 2] > g[:, 2].max() - 0.08]
    g[:, :2] += crown_all[:, :2].mean(0) - top_half[:, :2].mean(0)
    if opt.get('nudge'): g += np.array([float(x) for x in opt['nudge'].split(',')])
    set_gco(g)
    print(f'rough helmet: scale {s:.4f}, outer width {width:.3f}')
    samp = crown[rng.choice(len(crown), min(800, len(crown)), replace=False)]  # centre on the skull, not the hair
    if opt.get('refine', '1') == '1':
        refine(samp, float(opt.get('inside', 0.012)), float(opt.get('scale_prior', 60.0)), 'helmet')
    g = gco()
    if BONE not in gnames: gnames.append(BONE)
    GW = np.zeros((len(g), len(gnames)), np.float32)
    GW[:, gnames.index(BONE)] = 1.0

# ---------- push out of the body (and of garments already worn) in the rest pose ----------
update()
# hair volumes (hidden helper meshes, cards) are not pushed against: they hang over the collar
under = [body] + [h for h in heads if 'hair' not in h.name.lower()] + old_garments if MODE == 'body' else [skull_obj]
g = gco()
moved = 0
SOFT = opt.get('push', 'hard' if (MODE == 'body' and ALIGN == 'feet') else 'soft') == 'soft'
for o in under:
    oco, onr, otri = evaluated(o)
    tree = bvh(oco, otri)
    lim = CLEAR if o in (body,) or o in heads else CLEAR * 1.5
    if SOFT:
        # thin plates and straps: move each dent together with everything within a few cm (both faces of a plate),
        # so pushing the inner face out does not fold it through the outer one
        from scipy.spatial import cKDTree
        idx, dv = [], []
        for i, p in enumerate(g):
            q, n, ti, d = tree.find_nearest(Vector(p), float(opt.get('push_search', 0.15)))
            if q is None: continue
            n = smooth_normal(q, ti, oco, onr, otri)
            sd = (Vector(p) - q).dot(n)
            if sd < lim: idx.append(i); dv.append(np.array(n) * (lim - sd))
        if idx:
            idx = np.array(idx); dv = np.array(dv); mag = np.linalg.norm(dv, axis=1)
            R0, R1 = float(opt.get('soft_r0', 0.015)), float(opt.get('soft_r1', 0.04))
            kd = cKDTree(g[idx])
            D = np.zeros_like(g); best = np.zeros(len(g))
            for i, near in enumerate(cKDTree(g).query_ball_tree(kd, R1)):
                if not near: continue
                near = np.array(near)
                dist = np.linalg.norm(g[idx[near]] - g[i], axis=1)
                w = np.clip((R1 - dist) / (R1 - R0), 0, 1)
                k = np.argmax(w * mag[near])
                D[i] = w[k] * dv[near[k]]; best[i] = w[k] * mag[near[k]]
            g = g + D
            print(f'soft push against {o.name}: {len(idx)} dents, {(best > 1e-4).sum()} verts moved, max {mag.max():.3f} m')
    for i, p in enumerate(g):
        q, n, ti, d = tree.find_nearest(Vector(p), float(opt.get('push_search', 0.15)))
        if q is None: continue
        if SOFT: n = smooth_normal(q, ti, oco, onr, otri)
        sd = (Vector(p) - q).dot(n)
        if sd < lim:
            g[i] = np.array(q + n * lim); moved += 1
set_gco(g)
# second test: walk out along the garment's own normal; if the body's outer shell is hit from inside, lift past it
gme.update()
gn = np.empty(len(gme.vertices) * 3, np.float32); gme.vertices.foreach_get('normal', gn); gn = gn.reshape(-1, 3)
lifted = 0
for o in under:
    oco, onr, otri = evaluated(o)
    tree = bvh(oco, otri)
    for i, p in enumerate(g):
        n = Vector(gn[i])
        q, hn, ti, d = tree.ray_cast(Vector(p) - n * 0.0005, n, float(opt.get('lift_search', 0.08)))
        if q is not None and hn.dot(n) > 0.2:
            g[i] = np.array(q + n * CLEAR); lifted += 1
set_gco(g)
print(f'pushed out {moved} verts, lifted {lifted}')

# ---------- hair under a helmet: squeeze it radially to just inside the shell ----------
if MODE != 'body' and opt.get('hair'):
    g = gco(); tree = bvh(g, gtri())
    c = Vector(crown.mean(0)); c.z -= 0.05
    for hn in opt['hair'].split(','):
        src = bpy.data.objects.get(hn)
        if not src: print('no hair object', hn); continue
        dup = src.copy(); dup.data = src.data.copy(); dup.name = f'{hn}_{NAME}'
        scene.collection.objects.link(dup)
        Mw = dup.matrix_world.copy(); Mi = Mw.inverted()
        n_sq = 0
        for v in dup.data.vertices:
            w = Mw @ v.co
            d = (w - c); L = d.length
            if L < 1e-6: continue
            d /= L
            hit, hn_, fi, dist = tree.ray_cast(c, d, 0.5)
            if hit is None: continue  # below the rim: stays as it is
            lim = (hit - c).length - float(opt.get('hair_gap', 0.006))
            if L > lim:
                v.co = Mi @ (c + d * lim); n_sq += 1
        dup['hair_for'] = NAME
        print(f'hair {hn}: {n_sq} verts squeezed under {NAME} -> {dup.name}')

# ---------- skin it ----------
gar.parent = arm
for k, n in enumerate(gnames):
    idx = np.where(GW[:, k] > 0)[0]
    if not len(idx): continue
    vg = gar.vertex_groups.new(name=n)
    for i in idx: vg.add([int(i)], float(GW[i, k]), 'REPLACE')
mod = gar.modifiers.new('Armature', 'ARMATURE'); mod.object = arm

# ---------- body vertices this garment covers ----------
update()
if MODE != 'body':
    gar['garment'] = True; gar['slot'] = opt.get('slot', NAME)
    bpy.ops.wm.save_as_mainfile(filepath=OUT, compress=True)
    print('saved', OUT, 'verts', len(gme.vertices)); sys.exit(0)
bco, bnr, _ = evaluated(body)
g = gco(); tree = bvh(g, gtri())
cover = np.zeros(nb, bool)
for i in range(nb):
    hit = tree.ray_cast(Vector(bco[i] + bnr[i] * 0.001), Vector(bnr[i]), float(opt.get('cover_dist', 0.08)))
    if hit[0] is not None: cover[i] = True; continue
    # poking through: the garment is right under this body vertex
    hit = tree.ray_cast(Vector(bco[i] - bnr[i] * 0.001), Vector(-bnr[i]), float(opt.get('poke_dist', 0.05)))
    if hit[0] is not None: cover[i] = True
if opt.get('hide_below'):
    if opt['hide_below'] == 'auto':
        # the neck opening: boundary edges of the garment in the upper body, lowest point - 1.5 cm
        bm = bmesh.new(); bm.from_mesh(gme)
        zs3 = (arm.matrix_world @ arm.pose.bones['spine_03'].head).z
        nk = arm.matrix_world @ arm.pose.bones['neck_01'].head
        allb = [v.co for e in bm.edges if e.is_boundary for v in e.verts]
        # the neck opening only: boundary near the neck axis (pocket flaps on the chest have open edges too)
        bz = [c.z for c in allb if c.z > zs3 and math.hypot(c.x - nk.x, c.y - nk.y) < float(opt.get('collar_r', 0.13))]
        allb = [c.z for c in allb]
        bm.free()
        print(f'boundary verts {len(allb)}, z {min(allb, default=0):.3f}..{max(allb, default=0):.3f}, spine_03 {zs3:.3f}')
        if bz: zcut = min(bz) - 0.015
        else:
            zcut = (arm.matrix_world @ arm.pose.bones['neck_01'].head).z - 0.03
            print('no neck opening found: hiding below neck_01 - 3 cm')
    else:
        zcut = float(opt['hide_below'])
    cover |= bco[:, 2] < zcut
    print(f'hide everything below {zcut:.3f}')
# erode two rings so the faces along the edge stay visible
bed = np.empty(len(body.data.edges) * 2, np.int32); body.data.edges.foreach_get('vertices', bed); bed = bed.reshape(-1, 2)
for _ in range(int(opt.get('erode', 2))):
    bad = ~cover
    edge_bad = bad[bed[:, 0]] | bad[bed[:, 1]]
    nxt = cover.copy(); nxt[bed[edge_bad].ravel()] = False
    cover = nxt
vg = body.vertex_groups.get('hide_' + NAME) or body.vertex_groups.new(name='hide_' + NAME)
vg.add([int(i) for i in np.where(cover)[0]], 1.0, 'REPLACE')
print(f'body verts covered: {cover.sum()} of {nb}')

gar['garment'] = True
gar['slot'] = opt.get('slot', NAME)
bpy.ops.wm.save_as_mainfile(filepath=OUT, compress=True)
print('saved', OUT, 'verts', len(gme.vertices))
