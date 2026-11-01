"""04 Shag (the in-game "long" female cut): wispy pointed fringe to the brows, choppy layers falling from a
rounded crown over the ears to the jaw, tapered pointed nape, piecey ends flicking out, a few flyaways."""
import sys, os, math, time
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import numpy as np
from mathutils import Vector, Matrix, noise
import hairgen as hg
from hairgen import smooth, C, DOWN

args = sys.argv[sys.argv.index("--") + 1:]
out = args[0]
GUIDES = int(args[1]) if len(args) > 1 else 1500
KIDS = int(args[2]) if len(args) > 2 else 60000
VIEWS = args[3].split(",") if len(args) > 3 else ["front", "back", "left", "right"]
SAVE = args[4] if len(args) > 4 else None
rng = np.random.default_rng(9)
PIECES = int(os.environ.get("PIECES", 450))
COLOR = tuple(float(x) for x in os.environ.get("HAIRCOL", "0.06,0.032,0.016").split(","))
t0 = time.time()
head = hg.Head()

WHORL = Vector((0.012, 0.072, 1.680))
PART_X = 0.008           # the fringe opens a little left of centre
FR_LY = -0.030           # fringe roots: in front of this (ly)


def V(*a):
    return Vector(a)


def label(p):
    az, el = hg.azel(p)
    if p.y - C.y < FR_LY and abs(az) < 40 and el > 12:
        return "F"
    return "H"


def zstop(az, el):
    """Jaw at the sides, a little shorter beside the face, nape down to the gaiter; the crown is cut
    shorter so its ends show as a layer over the longer hair."""
    a = abs(az)
    z = 1.494 + 0.026 * smooth(70, 35, a) + 0.010 * smooth(120, 150, a) * smooth(175, 150, a)
    return z + 0.045 * smooth(20, 60, el) * rng.uniform(0.2, 1.0)


def field(p):
    """Minimum distance of the hair from the scalp while it grows: a full rounded crown; the bell below it
    is made by envelope() after the children are made."""
    az, el = hg.azel(p)
    face = smooth(30, 75, abs(az))
    return 0.005 * (1 - face) + (0.006 + 0.014 * smooth(-10, 40, el)) * face


A_X, A_Y = 0.0, 0.02      # vertical axis through the cranium centre
TH = np.radians(np.arange(-180, 181, 2.0))
ZS = np.arange(1.40, 1.721, 0.004)


def head_radius_table():
    """Horizontal radius of the head around the axis, per azimuth and height. The ears are ignored (the hair
    covers them); at the sides the hair falls straight past the jaw, at the back it follows the skull in
    to the nape (the taper)."""
    R = np.zeros((len(TH), len(ZS)))
    B = np.zeros((len(TH), len(ZS)))
    for i, th in enumerate(TH):
        d = Vector((math.sin(th), -math.cos(th), 0.0))
        for j, z in enumerate(ZS):
            o = Vector((A_X, A_Y, z)) + d * 0.3
            loc, _, _, _ = head.bvh.ray_cast(o, -d, 0.3)
            if loc is not None:
                R[i, j] = math.hypot(loc.x - A_X, loc.y - A_Y)
            loc, _, _, _ = head.body.ray_cast(o, -d, 0.3)
            if loc is not None:
                B[i, j] = math.hypot(loc.x - A_X, loc.y - A_Y) + 0.004
    ad = np.abs(np.degrees(TH))
    ear = (ad[:, None] > 55) & (ad[:, None] < 135) & (ZS[None, :] < 1.592)
    R[ear] = 0.0
    cyl = R.copy()
    top = np.searchsorted(ZS, 1.62)
    for j in range(top - 1, -1, -1):
        cyl[:, j] = np.maximum(cyl[:, j], cyl[:, j + 1])
    b = np.clip((ad - 118) / 30, 0, 1)[:, None]
    raw = np.maximum(np.maximum(R, B), cyl * 0.62)
    return cyl * (1 - b) + raw * b


def bilinear(T, th, z):
    fi = (np.degrees(th) + 180.0) / 2.0
    fj = (z - ZS[0]) / (ZS[1] - ZS[0])
    i0 = np.clip(np.floor(fi).astype(int), 0, len(TH) - 2)
    j0 = np.clip(np.floor(fj).astype(int), 0, len(ZS) - 2)
    a = np.clip(fi - i0, 0, 1)
    b = np.clip(fj - j0, 0, 1)
    return (T[i0, j0] * (1 - a) * (1 - b) + T[i0 + 1, j0] * a * (1 - b)
            + T[i0, j0 + 1] * (1 - a) * b + T[i0 + 1, j0 + 1] * a * b)


def envelope(strands, depth, iters=8, smooth_k=0.3):
    """Push the hair out horizontally onto a rounded bell: full over the ears, straight past the jaw,
    tapering in at the nape. depth[c] puts lower-root hair under the top layer."""
    out = np.array(strands, float)
    for it in range(iters):
        dx, dy, z = out[..., 0] - A_X, out[..., 1] - A_Y, out[..., 2]
        th = np.arctan2(dx, -dy)
        r = np.hypot(dx, dy) + 1e-9
        ad = np.abs(np.degrees(th))
        vol = 0.018 + 0.014 * np.clip((ad - 35) / 25, 0, 1) - 0.006 * np.clip((ad - 120) / 50, 0, 1)
        vol = vol * (0.5 + 0.5 * np.clip((1.69 - z) / 0.09, 0, 1))
        vol = vol * (1 - 0.55 * np.clip((ad - 120) / 40, 0, 1) * np.clip((1.565 - z) / 0.055, 0, 1))
        R = bilinear(RH, th, z) + vol - depth[:, None]
        w = np.clip((ad - 28) / 25, 0, 1) * np.clip((1.70 - z) / 0.05, 0, 1)
        w[:, 0] = 0
        # the back is pulled in under the occiput, so the nape tapers to a point
        taper = np.clip((ad - 115) / 35, 0, 1) * np.clip((1.60 - z) / 0.06, 0, 1)
        push = np.maximum(R - r, 0.0) * w + np.minimum(R - r, 0.0) * taper * 0.6
        out[..., 0] += dx / r * push
        out[..., 1] += dy / r * push
        if smooth_k and it < iters - 1:
            out = hg.relax(out, iters=1, k=smooth_k)
    return out


def guide_ctx(p, n):
    az, el = hg.azel(p)
    ly = p.y - C.y
    lab = label(p)
    top = smooth(15, 45, el)
    ctx = {"az": az, "el": el, "lab": lab, "aim": None, "top": top,
           "rot": 0.5 * noise.noise(p * 34.0) + rng.normal(0, 0.14),
           "flick": (Vector(noise.noise_vector(p * 45.0)) + V(p.x, ly, 0).normalized() * 0.8).normalized(),
           "fl": rng.uniform(0.3, 0.9), "stick": 0.55, "layer": rng.uniform(0.8, 1.0)}
    j = V(*rng.normal(0, 0.004, 3))
    if lab == "F":
        sx = 1.0 if p.x > PART_X else -1.0
        x1 = p.x * 1.15 + sx * 0.004
        ctx["aim"] = head.on((x1, -0.12, 1.640), 0.012) + j
        ctx["aims"] = [head.on((p.x * 1.35 + sx * 0.010, -0.12, 1.596), 0.006) + j * 1.5]
        ctx["pass"] = True
        ctx["extra"] = 0.03
        ctx["zstop"] = 1.600 + rng.normal(0, 0.007)
        ctx["lift"] = 24 + 10 * top
        ctx["stick"] = 0.65
        ctx["fl"] = rng.uniform(0.0, 0.25)
    else:
        radial = p - WHORL
        radial.z = 0
        comb = radial.normalized() if radial.length > 0.012 else V(0, 1, 0)
        side = V(1.0 if p.x > 0 else -1.0, 0.15, 0)
        # beside the face the hair is combed out to the side rather than forward onto the cheek
        comb = comb.lerp(side.normalized(), 0.55 * smooth(25, 55, abs(az)) * smooth(110, 80, abs(az)))
        ctx["comb"] = comb.normalized()
        ctx["lift"] = 8 + 30 * top
        if el > 35 and rng.random() < 0.18:
            ctx["lift"] = rng.uniform(50, 70)          # a few pieces stand up off the crown before falling
        ctx["zstop"] = zstop(az, el) + rng.normal(0, 0.004)
    return ctx, 0.34


def want(p, s, prev, n, ctx):
    if s == 0.0:
        d = ctx["aim"] - p if ctx["aim"] is not None else ctx["comb"]
        ct = (d - n * d.dot(n)).normalized()
        a = math.radians(ctx["lift"])
        return (ct * math.cos(a) + n * math.sin(a)).normalized()
    if "left" in ctx:
        w = prev * 0.3 + DOWN * 0.7
    elif ctx["aim"] is not None:
        w = (ctx["aim"] - p).normalized()
        w = (Matrix.Rotation(ctx["rot"], 3, n) @ w).normalized()
        w = w * 0.9 + DOWN * 0.1
    else:
        c = Matrix.Rotation(ctx["rot"] * (0.4 + s), 3, n) @ ctx["comb"]
        ct = c - n * c.dot(n)
        g = 0.10 + 0.90 * smooth(0.02, 0.20, s)
        w = ct.normalized() * (1 - g) + DOWN * g
    zs = ctx["zstop"]
    if ctx["lab"] == "H" and p.z < zs + 0.03:
        # choppy ends flick out a little
        w = w + ctx["flick"] * ctx["fl"] * smooth(zs + 0.03, zs, p.z)
    return w.normalized()


def shell(p, s, ctx):
    return 0.0015 + field(p) * ctx["layer"] * smooth(0.0, 0.03, s * 0.34)


groots, gn = head.sample(GUIDES, hg.scalp_density, seed=19)
N = 36
guides, glab = [], []
for r, n in zip(groots, gn):
    p = Vector(r)
    ctx, L = guide_ctx(p, Vector(n))
    pts = hg.grow(head, p, n, want, L, 0.004, shell, stiff=0.7, ctx=ctx)
    guides.append(hg.resample(pts, N))
    glab.append(ctx["lab"])
guides = hg.relax(np.array(guides), iters=6)
print("guides", len(guides), {l: glab.count(l) for l in set(glab)}, round(time.time() - t0, 1))

RH = head_radius_table()
if os.environ.get("GUIDES_ONLY"):
    gel = np.array([hg.azel(Vector(r))[1] for r in groots])
    guides = envelope(guides, 0.020 * (1 - np.clip((gel + 20) / 55, 0, 1)) * rng.uniform(0.4, 1.0, len(groots)))
    hg.make_curves("Guides", guides, r_root=2.5e-4, r_tip=1.5e-4, material=hg.hair_material(color=(0.05, 0.03, 0.02)))
    hg.setup_render(samples=16, res=(600, 660))
    hg.render_views(out + "/shagG", views=VIEWS)
    hg.save_grid([hg.load_px(f"{out}/shagG-{v}.png") for v in VIEWS], f"{out}/shagG-sheet.png", cols=4)
    raise SystemExit
roots, cn = head.sample(KIDS, hg.scalp_density, seed=31)
clab = np.array([label(Vector(r)) for r in roots])
m = ~((clab == "F") & (rng.random(len(roots)) < 0.35))     # a thinner, wispy fringe
roots, cn, clab = roots[m], cn[m], clab[m].tolist()
kids, near = hg.children(guides, groots, glab, roots, clab, k=4)
rel = np.array([hg.azel(Vector(r))[1] for r in roots])
depth = 0.012 * (1 - np.clip((rel + 20) / 55, 0, 1)) * rng.uniform(0.4, 1.0, len(roots))
kids = envelope(kids, depth)
# piecey: tousled pieces on the crown splay apart, the fringe gathers into a few pointed wisps
ae = np.array([hg.azel(Vector(r)) for r in roots])
fan = 0.008 + 0.016 * np.clip((ae[:, 1] - 5.0) / 40.0, 0, 1)
fr = np.array(clab) == "F"
kids, keep, owner = hg.leader_clump(kids, roots, clab, PIECES, rng, strength=(0.8, 0.98), power=np.where(fr, 0.8, 0.9), fan=fan,
                                    jag=(0.6, 1.0), cw_scale=np.where(fr, 1.25, 1.0), ret_owner=True)
keep = np.where(fr, keep * rng.uniform(0.55, 1.0, len(roots)), keep)
kids = hg.trim(kids, keep)
kids = envelope(kids, depth, iters=3, smooth_k=0.2)
# layered pieces: each piece sits a little in or out of its neighbours, so the cut lines read
pr = rng.normal(0, 1, len(roots))[owner]
hz = kids - np.array([A_X, A_Y, 0.0])
hz[..., 2] = 0
hz /= np.linalg.norm(hz, axis=2, keepdims=True) + 1e-9
kids = kids + hz * (pr[:, None, None] * 0.010 * np.linspace(0, 1, kids.shape[1])[None, :, None] ** 0.5)
u = rng.random(len(roots))
amp = np.where(u < 0.04, 0.011, 0.00014)
kids = hg.frizz(kids, amp, rng, power=2.0, freq=(1, 3))
stray = (u > 0.993) & (ae[:, 1] > 10)          # a few long flyaways lifting off the crown
kids = hg.frizz(kids, np.where(stray, 0.022, 0.0), rng, power=1.2, freq=(0.4, 1.0))
kids = hg.pushout(head, kids, frac=0.6)
kids = hg.relax(kids, iters=3, k=0.4)
print("kids", kids.shape, round(time.time() - t0, 1))

mat = hg.hair_material(color=COLOR, coat=0.0, rough=float(os.environ.get("ROUGH", 0.32)), radial=0.35, grad=(0.6, 1.08), rnd=0.22)
hg.make_curves("Hair_shag", kids, r_root=6e-5, r_tip=2.2e-5, material=mat)
head.scalp_cap(hg.scalp_density)
hg.setup_render(samples=int(os.environ.get("SPP", 96)))
hg.render_views(out + "/shag", views=VIEWS)
hg.save_grid([hg.load_px(f"{out}/shag-{v}.png") for v in VIEWS], f"{out}/shag-sheet.png", cols=4)
if SAVE:
    import bpy
    bpy.ops.wm.save_as_mainfile(filepath=SAVE, compress=True)
print("done", round(time.time() - t0, 1))
