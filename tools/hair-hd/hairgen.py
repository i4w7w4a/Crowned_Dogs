"""Procedural groom helpers: scalp sampling, guide growth, children, render."""
import bpy, math, os
import numpy as np
from mathutils import Vector, Matrix
from mathutils.bvhtree import BVHTree
from mathutils.kdtree import KDTree

# PERSONA=male: the male base scene (base_male.blend, heads m1-m4); he stands 12 cm taller
MALE = os.environ.get("PERSONA") == "male"
C = Vector((0.003, -0.008, 1.700)) if MALE else Vector((0.0, 0.02, 1.585))   # cranium centre (world, rest pose)
DOWN = Vector((0, 0, -1))


def smooth(a, b, x):
    t = min(max((x - a) / (b - a), 0.0), 1.0)
    return t * t * (3 - 2 * t)


def lerp_table(table, x):
    if x <= table[0][0]:
        return table[0][1]
    for (x0, y0), (x1, y1) in zip(table, table[1:]):
        if x <= x1:
            return y0 + (y1 - y0) * (x - x0) / (x1 - x0)
    return table[-1][1]


def azel(p):
    d = p - C
    return math.degrees(math.atan2(d.x, -d.y)), math.degrees(math.atan2(d.z, math.hypot(d.x, d.y)))


# Hairline elevation by |azimuth| (0 = face, 90 = ear, 180 = nape)
HAIRLINE = [(0, 44), (25, 42), (45, 34), (60, 22), (70, 8), (75, -14), (84, -14), (87, 5), (100, 7),
            (112, 5), (118, -22), (135, -36), (150, -42), (180, -44)]


def mesh_world(obj, dg):
    e = obj.evaluated_get(dg)
    m = e.to_mesh()
    m.calc_loop_triangles()
    mw = obj.matrix_world
    verts = [mw @ v.co for v in m.vertices]
    tris = [tuple(t.vertices) for t in m.loop_triangles]
    e.to_mesh_clear()
    return verts, tris


class Head:
    def __init__(self, head=None, body=None):
        head = head or ("Head_m1" if MALE else "Head_f1")
        body = body or (("MaleBody", "Buff", "Suit") if MALE else ("FemBody", "Buff", "Suit"))
        dg = bpy.context.evaluated_depsgraph_get()
        self.verts, self.tris = mesh_world(bpy.data.objects[head], dg)
        self.bvh = BVHTree.FromPolygons(self.verts, self.tris)
        bv, bt = [], []
        for n in body:
            v, t = mesh_world(bpy.data.objects[n], dg)
            o = len(bv)
            bv += v
            bt += [(a + o, b + o, c + o) for a, b, c in t]
        self.body = BVHTree.FromPolygons(bv, bt)

    def scalp_cap(self, density, name="Scalp_cap", color=(0.10, 0.065, 0.045), off=0.0005, thresh=0.25, fade=(0.45, 0.95)):
        """Hair-coloured skin under the groom so the scalp never shows through as bare skin.
        It fades out toward the hairline (per-vertex density), so its triangle edges never show."""
        V_ = np.array(self.verts)
        T = np.array(self.tris)
        a, b, c = V_[T[:, 0]], V_[T[:, 1]], V_[T[:, 2]]
        fn = np.cross(b - a, c - a)
        cen = (a + b + c) / 3
        keep = []
        for i in range(len(T)):
            p = Vector(cen[i])
            if Vector(fn[i]).dot(p - C) <= 0:
                continue
            az, el = azel(p)
            if density(az, el, p) > thresh:
                keep.append(i)
        vn = np.zeros_like(V_)
        for k in range(3):
            np.add.at(vn, T[:, k], fn)
        vn /= np.maximum(np.linalg.norm(vn, axis=1, keepdims=True), 1e-12)
        used = np.unique(T[keep].ravel())
        remap = {int(v): i for i, v in enumerate(used)}
        verts = [tuple(V_[v] + vn[v] * off) for v in used]
        faces = [tuple(remap[int(v)] for v in T[i]) for i in keep]
        me = bpy.data.meshes.new(name)
        me.from_pydata(verts, [], faces)
        me.update()
        for poly in me.polygons:
            poly.use_smooth = True
        dens = []
        for v in used:
            az, el = azel(Vector(V_[v]))
            dens.append(smooth(fade[0], fade[1], density(az, el, Vector(V_[v]))))
        at = me.attributes.new("dens", 'FLOAT', 'POINT')
        at.data.foreach_set("value", np.array(dens, np.float32))
        m = bpy.data.materials.new(name)
        m.use_nodes = True
        nt = m.node_tree
        nt.nodes.clear()
        bsdf = nt.nodes.new("ShaderNodeBsdfPrincipled")
        attr = nt.nodes.new("ShaderNodeAttribute")
        attr.attribute_name = "dens"
        mix = nt.nodes.new("ShaderNodeMixShader")
        nt.links.new(attr.outputs["Fac"], mix.inputs[0])
        nt.links.new(nt.nodes.new("ShaderNodeBsdfTransparent").outputs[0], mix.inputs[1])
        nt.links.new(bsdf.outputs[0], mix.inputs[2])
        nt.links.new(mix.outputs[0], nt.nodes.new("ShaderNodeOutputMaterial").inputs[0])
        bsdf.inputs["Base Color"].default_value = (*color, 1)
        bsdf.inputs["Roughness"].default_value = 0.85
        me.materials.append(m)
        ob = bpy.data.objects.new(name, me)
        bpy.context.scene.collection.objects.link(ob)
        arm = bpy.data.objects.get("Armature")
        if arm:
            ob.parent = arm
            ob.parent_type = 'BONE'
            ob.parent_bone = "head"
            bpy.context.view_layer.update()
            ob.matrix_world = Matrix.Identity(4)
        return ob

    def on(self, v, off):
        """Point on the head surface nearest to v, pushed out along the normal by off."""
        loc, n, _, _ = self.bvh.find_nearest(Vector(v), 1.0)
        return loc + n * off

    def sample(self, count, density, seed=1):
        """Area-weighted roots on the scalp. density(az, el, p) -> 0..1."""
        rng = np.random.default_rng(seed)
        V = np.array(self.verts)
        T = np.array(self.tris)
        a, b, c = V[T[:, 0]], V[T[:, 1]], V[T[:, 2]]
        cr = np.cross(b - a, c - a)
        area = np.linalg.norm(cr, axis=1) / 2
        nrm = cr / np.maximum(np.linalg.norm(cr, axis=1, keepdims=True), 1e-12)
        cen = (a + b + c) / 3
        w = np.zeros(len(T))
        for i in range(len(T)):
            p = Vector(cen[i])
            d = (p - C).normalized()
            if Vector(nrm[i]).dot(d) < 0.15:
                continue
            az, el = azel(p)
            w[i] = area[i] * density(az, el, p)
        w /= w.sum()
        pick = rng.choice(len(T), size=count, p=w)
        u = rng.random(count)
        v = rng.random(count)
        flip = u + v > 1
        u[flip] = 1 - u[flip]
        v[flip] = 1 - v[flip]
        roots = a[pick] + (b[pick] - a[pick]) * u[:, None] + (c[pick] - a[pick]) * v[:, None]
        return roots, nrm[pick]


def scalp_density(az, el, p, fade=5.0):
    lo = lerp_table(HAIRLINE, abs(az))
    return smooth(lo, lo + fade, el)


def grow(head, root, nrm, want, length, ds, shell, stiff=0.6, hug=0.012, body=True, ctx=None):
    """Integrate one strand. want(p, s, prev, n_head, ctx) -> unit dir; shell(p, s, ctx) -> min offset."""
    p = Vector(root)
    n0 = Vector(nrm)
    prev = want(p, 0.0, n0, n0, ctx)
    pts = [p.copy()]
    steps = max(2, int(length / ds))
    for i in range(steps):
        s = (i + 1) / steps
        loc, n, _, dist = head.bvh.find_nearest(p, 0.2)
        n = n if loc is not None else n0
        w = want(p, s, prev, n, ctx)
        d = (prev * stiff + w * (1 - stiff)).normalized()
        sh = shell(p, s, ctx)
        if loc is not None:
            off = (p - loc).dot(n)
            if off < sh + hug and d.dot(n) < 0:
                d = (d - n * d.dot(n) * (1 - max(0.0, off - sh) / hug)).normalized()
            st = ctx.get("stick", 0.0) * (1 - ctx.get("free", 0.0) * smooth(0.6, 0.95, s)) if ctx else 0.0
            if st and off > sh and dist < 0.04:
                # styled hair is pulled back onto its shell instead of flying off the curvature
                d = (d - n * st * smooth(0.0, 0.012, off - sh)).normalized()
        q = p + d * ds
        for _ in range(2):
            loc, n2, _, dist = head.bvh.find_nearest(q, 0.2)
            if loc is None:
                break
            off = (q - loc).dot(n2)
            if off < sh:
                q = q + n2 * (sh - off)
            else:
                break
        if body:
            loc, n2, _, dist = head.body.find_nearest(q, 0.1)
            if loc is not None:
                off = (q - loc).dot(n2)
                if off < 0.003:
                    q = q + n2 * (0.003 - off)
        d = (q - p).normalized()
        q = p + d * ds
        prev = d
        p = q
        pts.append(q.copy())
        if ctx and ctx.get("zstop") is not None and p.z < ctx["zstop"]:
            break
        if ctx and ctx.get("aim") is not None and "extra" in ctx:
            a = ctx["aim"] - p
            if "left" not in ctx and (a.length < 0.014 or (ctx.get("pass") and a.length < 0.04 and a.dot(prev) < 0)):
                if ctx.get("aims"):
                    ctx["aim"] = ctx["aims"].pop(0)
                else:
                    ctx["left"] = ctx["extra"]
            if "left" in ctx:
                ctx["left"] -= ds
                if ctx["left"] <= 0:
                    break
    return np.array(pts)


def resample(pts, n):
    seg = np.linalg.norm(np.diff(pts, axis=0), axis=1)
    t = np.concatenate([[0], np.cumsum(seg)])
    t /= t[-1]
    u = np.linspace(0, 1, n)
    return np.stack([np.interp(u, t, pts[:, k]) for k in range(3)], axis=1)


def children(guides, groots, labels_g, roots, labels_c, k=3):
    """Interpolate guide shapes onto child roots (same label only). Returns (C,N,3), nearest guide idx."""
    G = np.asarray(guides)
    rel = G - G[:, :1]
    out = np.zeros((len(roots), G.shape[1], 3))
    near = np.zeros(len(roots), int)
    trees = {}
    for lab in set(labels_g):
        idx = [i for i, l in enumerate(labels_g) if l == lab]
        t = KDTree(len(idx))
        for j, i in enumerate(idx):
            t.insert(groots[i], j)
        t.balance()
        trees[lab] = (t, idx)
    for c, r in enumerate(roots):
        t, idx = trees[labels_c[c]]
        found = t.find_n(r, k)
        ids = [idx[f[1]] for f in found]
        ds = np.array([f[2] for f in found])
        w = 1.0 / (ds + 1e-4) ** 2
        w /= w.sum()
        out[c] = r + np.tensordot(w, rel[ids], axes=1)
        near[c] = ids[0]
    return out, near


def clump(strands, roots, guides, near, strength, power=1.5, spread=1.0):
    G = np.asarray(guides)
    N = strands.shape[1]
    s = np.linspace(0, 1, N)[None, :, None]
    cw = np.asarray(strength).reshape(-1, 1, 1) * s ** power
    target = G[near] + (roots - G[near][:, 0])[:, None, :] * (1 - s) * spread
    return strands + (target - strands) * cw


def frizz(strands, amp, rng, power=2.0, freq=(2, 6)):
    C_, N, _ = strands.shape
    s = np.linspace(0, 1, N)[None, :]
    out = strands.copy()
    amp = np.broadcast_to(np.asarray(amp, float), (C_,)).reshape(-1, 1)
    for k in range(3):
        ph = rng.random((C_, 1)) * 6.283
        f = rng.uniform(freq[0], freq[1], (C_, 1))
        ph2 = rng.random((C_, 1)) * 6.283
        f2 = rng.uniform(freq[1], freq[1] * 2.5, (C_, 1))
        out[:, :, k] += amp * s ** power * (np.sin(f * s * 6.283 + ph) * 0.7 + np.sin(f2 * s * 6.283 + ph2) * 0.3)
    return out


def relax(strands, iters=4, k=0.5):
    """Laplacian smoothing along each strand (root fixed): removes the step jitter from collision pushes."""
    out = np.array(strands, float)
    for _ in range(iters):
        mid = (out[:, :-2] + out[:, 2:]) * 0.5
        out[:, 1:-1] += (mid - out[:, 1:-1]) * k
        out[:, -1] += ((2 * out[:, -2] - out[:, -3]) - out[:, -1]) * k * 0.5
    return out


def trim(strands, keep):
    """Cut each strand to fraction keep[c] of its length, resampled to the same point count."""
    C_, N, _ = strands.shape
    u = np.linspace(0, 1, N)[None, :] * np.asarray(keep).reshape(-1, 1) * (N - 1)
    i0 = np.clip(np.floor(u).astype(int), 0, N - 2)
    f = (u - i0)[..., None]
    rows = np.arange(C_)[:, None]
    return strands[rows, i0] * (1 - f) + strands[rows, i0 + 1] * f


def pushout(head, strands, frac=0.7, minoff=0.0012, body=False):
    C_, N, _ = strands.shape
    M = max(2, int(N * frac))
    for c in range(C_):
        row = strands[c]
        for j in range(1, M):
            q = Vector(row[j])
            loc, n, _, _ = head.bvh.find_nearest(q, 0.05)
            if loc is not None:
                off = (q - loc).dot(n)
                if off < minoff:
                    row[j] = q + n * (minoff - off)
        if body:
            for j in range(1, N):
                q = Vector(row[j])
                loc, n, _, _ = head.body.find_nearest(q, 0.05)
                if loc is not None:
                    off = (q - loc).dot(n)
                    if off < 0.002:
                        row[j] = q + n * (0.002 - off)
    return strands


def hair_material(name="Hair ash brown", melanin=0.5, redness=0.28, rough=0.3, radial=0.35, coat=0.2, random_color=0.12, tint=(1, 1, 1), color=None, grad=None, rnd=0.0):
    """color: COLOR parametrization. grad=(root, tip) brightness factors along the strand, rnd: per-strand +-."""
    m = bpy.data.materials.get(name) or bpy.data.materials.new(name)
    m.use_nodes = True
    nt = m.node_tree
    nt.nodes.clear()
    out = nt.nodes.new("ShaderNodeOutputMaterial")
    h = nt.nodes.new("ShaderNodeBsdfHairPrincipled")
    try:
        h.model = 'CHIANG'
    except Exception:
        pass
    h.parametrization = 'COLOR' if color else 'MELANIN'
    vals = {"Color": (*color, 1.0) if color else None, "Melanin": melanin, "Melanin Redness": redness,
            "Random Color": random_color, "Roughness": rough, "Radial Roughness": radial, "Coat": coat,
            "Random Roughness": 0.15, "Tint": (*tint, 1.0)}
    for k, v in vals.items():
        sock = h.inputs.get(k)
        if sock is not None and v is not None:
            sock.default_value = v
    if color and (grad or rnd):
        info = nt.nodes.new("ShaderNodeHairInfo")
        g0, g1 = grad or (1.0, 1.0)
        mr = nt.nodes.new("ShaderNodeMapRange")
        mr.inputs[3].default_value, mr.inputs[4].default_value = g0, g1
        nt.links.new(info.outputs["Intercept"], mr.inputs[0])
        rr = nt.nodes.new("ShaderNodeMapRange")
        rr.inputs[3].default_value, rr.inputs[4].default_value = 1 - rnd, 1 + rnd
        nt.links.new(info.outputs["Random"], rr.inputs[0])
        mul = nt.nodes.new("ShaderNodeMath")
        mul.operation = 'MULTIPLY'
        nt.links.new(mr.outputs[0], mul.inputs[0])
        nt.links.new(rr.outputs[0], mul.inputs[1])
        sc = nt.nodes.new("ShaderNodeVectorMath")
        sc.operation = 'SCALE'
        sc.inputs[0].default_value = color
        nt.links.new(mul.outputs[0], sc.inputs["Scale"])
        nt.links.new(sc.outputs[0], h.inputs["Color"])
    nt.links.new(h.outputs[0], out.inputs[0])
    return m


def leader_clump(strands, roots, labels, count, rng, strength=(0.7, 0.95), power=1.0, fan=0.006, jag=(0.8, 1.0), cw_scale=1.0, ret_owner=False):
    """Gather children into `count` pointed pieces: each child is pulled toward the nearest leader child
    (same label) more and more toward the tip; leader tips get a little random fan-out; each piece
    gets one length so the ends read as layered, pointed locks. Returns strands, keep factors."""
    C_, N, _ = strands.shape
    s = np.linspace(0, 1, N)[None, :, None]
    labels = np.asarray(labels)
    lead = rng.choice(C_, size=min(count, C_), replace=False)
    owner = np.zeros(C_, int)
    for lab in set(labels.tolist()):
        li = lead[labels[lead] == lab]
        if len(li) == 0:
            continue
        t = KDTree(len(li))
        for j, i in enumerate(li):
            t.insert(roots[i], j)
        t.balance()
        for c in np.nonzero(labels == lab)[0]:
            owner[c] = li[t.find(roots[c])[1]]
    L = strands[lead].copy()
    fan = np.broadcast_to(np.asarray(fan, float), (C_,))[lead].reshape(-1, 1, 1)
    L += rng.normal(0, 1, (len(lead), 1, 3)) * fan * s[0] ** 2
    lidx = np.full(C_, -1)
    lidx[lead] = np.arange(len(lead))
    Ls = L[lidx[owner]]
    target = Ls + (roots - strands[owner][:, 0])[:, None, :] * (1 - s)
    cw = rng.uniform(*strength, (C_, 1, 1)) * np.broadcast_to(np.asarray(cw_scale, float), (C_,)).reshape(-1, 1, 1)
    cw = np.minimum(cw, 1.0) * s ** np.broadcast_to(np.asarray(power, float), (C_,)).reshape(-1, 1, 1)
    out = strands + (target - strands) * cw
    piece = rng.uniform(*jag, len(lead))
    keep = piece[lidx[owner]] * rng.uniform(0.9, 1.0, C_)
    if ret_owner:
        return out, keep, owner
    return out, keep


def make_curves(name, strands, r_root=4.5e-5, r_tip=1.2e-5, material=None, collection=None):
    old = bpy.data.objects.get(name)
    if old:
        bpy.data.objects.remove(old, do_unlink=True)
    C_, N, _ = strands.shape
    cv = bpy.data.hair_curves.new(name)
    cv.add_curves([N] * C_)
    cv.attributes["position"].data.foreach_set("vector", strands.astype(np.float32).ravel())
    s = np.linspace(0, 1, N)
    rad = np.tile(r_root + (r_tip - r_root) * s ** 1.5, C_).astype(np.float32)
    ra = cv.attributes.get("radius") or cv.attributes.new("radius", 'FLOAT', 'POINT')
    ra.data.foreach_set("value", rad)
    if material:
        cv.materials.append(material)
    ob = bpy.data.objects.new(name, cv)
    (collection or bpy.context.scene.collection).objects.link(ob)
    arm = bpy.data.objects.get("Armature")
    if arm:
        ob.parent = arm
        ob.parent_type = 'BONE'
        ob.parent_bone = "head"
        bpy.context.view_layer.update()
        ob.matrix_world = Matrix.Identity(4)
    return ob


def setup_render(samples=64, res=(900, 1000)):
    sc = bpy.context.scene
    prefs = bpy.context.preferences.addons['cycles'].preferences
    prefs.compute_device_type = 'OPTIX'
    prefs.get_devices()
    for d in prefs.devices:
        d.use = d.type == 'OPTIX'
    sc.render.engine = 'CYCLES'
    sc.cycles.device = 'GPU'
    sc.cycles.samples = samples
    sc.cycles.use_denoising = True
    sc.cycles.denoiser = 'OPENIMAGEDENOISE'
    try:
        sc.cycles.denoising_prefilter = 'ACCURATE'
        sc.cycles.denoising_quality = 'HIGH'
    except Exception:
        pass
    import os
    mul = float(os.environ.get("RESMUL", 1))
    sc.render.resolution_x, sc.render.resolution_y = int(res[0] * mul), int(res[1] * mul)
    if os.environ.get("NODENOISE"):
        sc.cycles.use_denoising = False
    if os.environ.get("BORDER"):
        x0, y0, x1, y1 = (float(v) for v in os.environ["BORDER"].split(","))
        sc.render.use_border, sc.render.use_crop_to_border = True, True
        sc.render.border_min_x, sc.render.border_min_y, sc.render.border_max_x, sc.render.border_max_y = x0, y0, x1, y1
    sc.cycles_curves.shape = 'THICK'
    sc.cycles_curves.subdivisions = 3
    sc.render.film_transparent = False
    # the reference sheet is lit softly: a weaker back-left rim keeps thin backlit hair from glowing white
    rim = bpy.data.objects.get("Rim")
    if rim is not None and "base_energy" not in rim:
        rim["base_energy"] = rim.data.energy
    if rim is not None:
        rim.data.energy = rim["base_energy"] * float(os.environ.get("RIM", 0.45))


REF = r"C:\Users\qtas5\Documents\CrownedDogs\research\hair-ref\hair-concepts-ref.webp"
# reference sheet columns (pixels, top-left origin): style -> (x0, x1); rows front/back
REF_COL = {"pixie": (0, 512), "bob": (512, 1024), "braid": (1024, 1536)}
REF_ROW = {"front": (64, 624), "back": (624, 1024)}


def load_px(p, box=None):
    im = bpy.data.images.load(p, check_existing=False)
    w, h = im.size
    a = np.empty(w * h * 4, np.float32)
    im.pixels.foreach_get(a)
    bpy.data.images.remove(im)
    a = a.reshape(h, w, 4)
    if box:
        x0, y0, x1, y1 = box
        a = a[h - y1:h - y0, x0:x1]
    return a


def fit_h(a, H):
    h, w = a.shape[:2]
    W = max(1, int(round(w * H / h)))
    ys = (np.arange(H) * h / H).astype(int)
    xs = (np.arange(W) * w / W).astype(int)
    return a[ys][:, xs]


def montage(paths, out, cols=None, scale=1, H=None, refs=()):
    """Paste images side by side (rows of cols) into one PNG. refs: (style, row) crops of the reference first."""
    imgs = []
    for style, row in refs:
        x0, x1 = REF_COL[style]
        y0, y1 = REF_ROW[row]
        imgs.append(load_px(REF, (x0, y0, x1, y1)))
    for p in paths:
        imgs.append(load_px(p)[::scale, ::scale])
    if H:
        imgs = [fit_h(a, H) for a in imgs]
    cols = cols or len(imgs)
    rows = (len(imgs) + cols - 1) // cols
    h = max(i.shape[0] for i in imgs)
    w = max(i.shape[1] for i in imgs)
    sheet = np.ones((rows * h, cols * w, 4), np.float32)
    for k, a in enumerate(imgs):
        r, c = divmod(k, cols)
        y0 = (rows - 1 - r) * h
        sheet[y0:y0 + a.shape[0], c * w:c * w + a.shape[1]] = a
    im = bpy.data.images.new("montage", cols * w, rows * h, alpha=True)
    im.pixels.foreach_set(sheet.ravel())
    im.filepath_raw = out
    im.file_format = 'PNG'
    im.save()
    bpy.data.images.remove(im)


VIEWS = {"front": (14, 3),"back": (194, 6), "left": (90, 2), "right": (-90, 2), "top": (10, 60)}


def sheet(style, out, views=("front", "back", "left", "right")):
    """Reference front | render front | reference back | render back, then the side views."""
    ref = [("front", None), ("back", None)]
    imgs = []
    for row in ("front", "back"):
        x0, x1 = REF_COL[style]
        y0, y1 = REF_ROW[row]
        imgs.append(fit_h(load_px(REF, (x0, y0, x1, y1)), 560))
        imgs.append(fit_h(load_px(f"{out}/{style}-{row}.png"), 560))
    for v in views[2:]:
        imgs.append(fit_h(load_px(f"{out}/{style}-{v}.png"), 560))
    save_grid(imgs, f"{out}/{style}-sheet.png", cols=4)


def save_grid(imgs, out, cols):
    rows = (len(imgs) + cols - 1) // cols
    h = max(i.shape[0] for i in imgs)
    w = max(i.shape[1] for i in imgs)
    sheet = np.ones((rows * h, cols * w, 4), np.float32)
    for k, a in enumerate(imgs):
        r, c = divmod(k, cols)
        y0 = (rows - 1 - r) * h
        sheet[y0:y0 + a.shape[0], c * w:c * w + a.shape[1]] = a
    im = bpy.data.images.new("grid", cols * w, rows * h, alpha=True)
    im.pixels.foreach_set(sheet.ravel())
    im.filepath_raw = out
    im.file_format = 'PNG'
    im.save()
    bpy.data.images.remove(im)


def render_views(prefix, views=("front", "back", "left", "right"), target=None, dist=0.95, lens=85):
    target = target or ((0.003, -0.01, 1.68) if MALE else (0.0, 0.018, 1.565))
    sc = bpy.context.scene
    cam = bpy.data.objects["Review_camera"]
    cam.data.type = 'PERSP'
    cam.data.lens = lens
    cam.data.sensor_fit = 'AUTO'
    T = Vector(target)
    lights = [o for o in bpy.data.objects if o.type == 'LIGHT']
    base = {o.name: o.matrix_world.copy() for o in lights}
    follow = os.environ.get("FOLLOW", "back").split(",")
    for v in views:
        az, el = VIEWS[v]
        a, e = math.radians(az), math.radians(el)
        turn = math.radians(az - VIEWS["front"][0]) if v in follow else 0.0
        R = Matrix.Translation(Vector((T.x, T.y, 0))) @ Matrix.Rotation(turn, 4, 'Z') @ Matrix.Translation(Vector((-T.x, -T.y, 0)))
        for o in lights:
            o.matrix_world = R @ base[o.name]
        # az measured from the face (-Y) toward her left (+X)
        dirv = Vector((math.sin(a) * math.cos(e), -math.cos(a) * math.cos(e), math.sin(e)))
        cam.location = T + dirv * dist
        cam.rotation_euler = (T - cam.location).to_track_quat('-Z', 'Y').to_euler()
        sc.camera = cam
        sc.render.filepath = f"{prefix}-{v}.png"
        bpy.ops.render.render(write_still=True)
    for o in lights:
        o.matrix_world = base[o.name]
