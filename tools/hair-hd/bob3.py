"""02 Bob: side part on her left, long side fringe swept to her right over the outer eye and down to the jaw,
left side tucked behind the ear, round full back ending at the top of the gaiter with the ends turned under."""
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
rng = np.random.default_rng(5)
PIECES = int(os.environ.get("PIECES", 480))
COLOR = tuple(float(x) for x in os.environ.get("HAIRCOL", "0.25,0.135,0.07").split(","))
t0 = time.time()
head = hg.Head()

PART_X = 0.026          # side part on her left
PART_END = 0.040        # from the hairline back to the crown (ly)
CROWN = Vector((0.010, 0.070, 1.680))
# left side: swept back over the ear top (ear: y 0.017..0.063, top z 1.59), tucked behind it, hanging to the jaw
EAR_FRONT_L = head.on((0.090, 0.016, 1.600), 0.006)
TEMPLE_L = head.on((0.082, -0.008, 1.608), 0.008)
EAR_TOP_L = head.on((0.084, 0.060, 1.604), 0.006)
BEHIND_L = head.on((0.071, 0.080, 1.556), 0.006)
# fringe: from the part across the forehead (w1) to the outer right eye / cheekbone (w2), then down to the jaw
W1_R, W1_P = head.on((-0.045, -0.090, 1.612), 0.010), head.on((-0.005, -0.098, 1.632), 0.012)
W2_R, W2_P = head.on((-0.086, -0.052, 1.565), 0.012), head.on((-0.079, -0.068, 1.572), 0.010)


def density(az, el, p):
    # a thin sideburn on the tucked side: no dark angular patch in front of the ear
    m = smooth(62, 72, az) * smooth(95, 86, az) * smooth(12, 2, el)
    return hg.scalp_density(az, el, p, fade=5.0 + 7.0 * m) * (1 - 0.55 * m)


def V(*a):
    return Vector(a)


def label(p):
    ly = p.y - C.y
    az, el = hg.azel(p)
    if p.x > PART_X and ly < PART_END and az < 115:
        if az > 68 and el < 9:
            # sideburn: mostly tucked with the rest, a few wisps left hanging in front of the ear
            return "S"
        return "L"
    return "R"


def zstop(az):
    # chin length at the front and sides, top of the gaiter at the back; the tucked left side rests on the
    # gaiter top (lower, the ends would slide forward along it under the ear)
    a = abs(az)
    z = 1.468 + 0.028 * smooth(95, 165, a)
    return max(z, 1.498 * smooth(70, 95, az) + z * (1 - smooth(70, 95, az))) if az > 0 else z


def field(p, lab):
    """Minimum distance of the hair from the scalp: lift along the part, a full crown; the bell below
    the crown is made by envelope() after the children are made."""
    az, el = hg.azel(p)
    if lab in ("L", "S", "W") and az < 112:
        return 0.004 + 0.005 * smooth(15, 50, el)
    ly = p.y - C.y
    part = smooth(0.035, 0.0, abs(p.x - PART_X)) * smooth(0.07, 0.0, ly)
    return 0.005 + 0.010 * smooth(-10, 35, el) + 0.010 * part


def guide_ctx(p, n):
    az, el = hg.azel(p)
    ly = p.y - C.y
    lab = label(p)
    top = smooth(15, 45, el)
    ctx = {"az": az, "el": el, "lab": lab, "aim": None, "top": top,
           "rot": 0.18 * noise.noise(p * 30.0) + rng.normal(0, 0.04),
           "stick": 0.75, "zstop": zstop(az) + rng.normal(0, 0.003)}
    j = V(*rng.normal(0, 0.004, 3))
    if lab == "W":
        ctx["comb"] = V(0, 0.25, -1).normalized()
        ctx["lift"] = 4
        ctx["stick"] = 0.5
        ctx["zstop"] = 1.500 + rng.normal(0, 0.008)
        return ctx, 0.12
    if lab in ("L", "S"):
        hi = smooth(1.60, 1.67, p.z)                   # roots high on the side lie over the lower ones
        ctx["aim"] = EAR_TOP_L + j + V(0, 0.010 * hi - 0.004, 0.008 * hi)
        ctx["aims"] = [BEHIND_L + j * 0.6 + V(0, 0.006 * hi, 0.004 * hi)]
        if lab == "S":
            ctx["aims"].insert(0, ctx["aim"])
            ctx["aim"] = EAR_FRONT_L + j * 0.5
        elif az > 100:                                  # already above the ear: straight behind it
            ctx["aim"] = ctx["aims"].pop(0)
        elif az < 85 and ly < 0.03:                     # the front comes down over the temple first
            ctx["aims"].insert(0, ctx["aim"])
            ctx["aim"] = TEMPLE_L + j + V(0, 0.010 * hi, 0.010 * hi)
        ctx["pass"] = True
        ctx["extra"] = 0.3
        ctx["zstop"] = 1.500 + rng.normal(0, 0.004)
        ctx["lift"] = 12 * top
        ctx["stick"] = 0.8
    elif ly < 0.0 and el > 12:
        t = smooth(-0.065, PART_X, p.x)           # 1 at the part, 0 at her right temple
        f = smooth(0.0, -0.06, ly)                # 1 at the hairline
        ctx["aim"] = W1_R.lerp(W1_P, t).lerp(head.on(p + V(-0.035, 0.0, -0.01), 0.012), 1 - f) + j
        ctx["aims"] = [W2_R.lerp(W2_P, t).lerp(W2_R, 1 - f) + V(*rng.normal(0, 0.005, 3))]
        ctx["extra"] = 0.3
        ctx["zstop"] = 1.475 + 0.006 * f + rng.normal(0, 0.005)
        ctx["lift"] = 32 * t + 10
        ctx["stick"] = 0.35
    else:
        radial = p - CROWN
        radial.z = 0
        side = V(-1.0 if p.x < PART_X else 1.0, 0.25, 0)
        comb = radial.normalized() if radial.length > 0.012 else side.normalized()
        if ly < PART_END and p.x < PART_X:
            comb = comb.lerp(side.normalized(), smooth(0.06, 0.015, ly))   # top in front of the crown goes to her right
        if p.x > PART_X:
            # her left side behind the tuck: back and down behind the ear, never forward over it
            comb.y = max(comb.y, 0.6 * smooth(160, 120, az), 0.35)
        ctx["comb"] = comb.normalized()
        ctx["lift"] = 6 + 30 * top
        ctx["stick"] = 0.75 - 0.3 * top
    return ctx, 0.34


def want(p, s, prev, n, ctx):
    if s == 0.0:
        d = ctx["aim"] - p if ctx["aim"] is not None else ctx["comb"]
        ct = (d - n * d.dot(n)).normalized()
        a = math.radians(ctx["lift"])
        return (ct * math.cos(a) + n * math.sin(a)).normalized()
    if "left" in ctx:
        w = prev * 0.25 + DOWN * 0.75
    elif ctx["aim"] is not None:
        w = (ctx["aim"] - p).normalized()
        w = (Matrix.Rotation(ctx["rot"], 3, n) @ w).normalized()
        w = w * 0.9 + DOWN * 0.1
    else:
        c = Matrix.Rotation(ctx["rot"], 3, n) @ ctx["comb"]
        ct = c - n * c.dot(n)
        g = 0.12 + 0.88 * smooth(0.02, 0.22, s)
        w = ct.normalized() * (1 - g) + DOWN * g
    zs = ctx["zstop"]
    if p.z < zs + 0.035 and ctx["lab"] == "R":
        # the ends turn under a little toward the neck
        inward = V(-p.x, 0.07 - p.y, 0)
        if inward.length > 1e-4:
            w = w + inward.normalized() * 0.22 * smooth(zs + 0.03, zs, p.z)
    return w.normalized()


A_X, A_Y = 0.0, 0.02      # vertical axis through the cranium centre
TH = np.radians(np.arange(-180, 181, 2.0))
ZS = np.arange(1.40, 1.721, 0.004)


def head_radius_table():
    """Horizontal radius of the head (with ears) around the axis, per azimuth and height."""
    R = np.zeros((len(TH), len(ZS)))
    for i, th in enumerate(TH):
        d = Vector((math.sin(th), -math.cos(th), 0.0))
        for j, z in enumerate(ZS):
            o = Vector((A_X, A_Y, z)) + d * 0.3
            loc, _, _, _ = head.bvh.ray_cast(o, -d, 0.3)
            if loc is not None:
                R[i, j] = math.hypot(loc.x - A_X, loc.y - A_Y)
    # the ears do not shape the bob: the hair goes over (or behind) them with its own volume
    ear = (np.abs(np.degrees(TH))[:, None] > 55) & (np.abs(np.degrees(TH))[:, None] < 135) & (ZS[None, :] < 1.592)
    R[ear] = 0.0
    # hair falls straight past the narrowing jaw/neck: below 1.62 keep the widest radius seen above
    top = np.searchsorted(ZS, 1.62)
    for j in range(top - 1, -1, -1):
        R[:, j] = np.maximum(R[:, j], R[:, j + 1])
    return R


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
    """Push the hair out horizontally onto a bell around the head: full at the sides and back, straight
    down past the jaw, the last few cm turned under. depth[c] puts lower-root hair under the top layer."""
    out = np.array(strands, float)
    C_, N, _ = out.shape
    seg = np.linalg.norm(np.diff(out, axis=1), axis=2)
    tip = np.concatenate([np.cumsum(seg[:, ::-1], axis=1)[:, ::-1], np.zeros((C_, 1))], axis=1)
    for it in range(iters):
        dx, dy, z = out[..., 0] - A_X, out[..., 1] - A_Y, out[..., 2]
        th = np.arctan2(dx, -dy)
        r = np.hypot(dx, dy) + 1e-9
        deg = np.degrees(th)
        ad = np.abs(deg)
        # skull + volume: thin at the crown, full from the ear tops down, the last few cm turned under
        hem = 1.468 + 0.028 * np.clip((ad - 95) / 70, 0, 1)
        u = np.clip((z - hem) / (1.64 - hem), 0, 1)
        vol = 0.016 + 0.020 * np.clip((ad - 40) / 25, 0, 1) - 0.010 * np.clip((ad - 120) / 50, 0, 1)
        vol = vol * np.where(th > 0, 0.85, 1.0) * (0.35 + 0.65 * np.clip((1.68 - z) / 0.10, 0, 1))
        turn = (0.003 + 0.017 * np.clip((ad - 95) / 60, 0, 1)) * np.clip(1 - u / 0.3, 0, 1) ** 2
        R = bilinear(RH, th, z) + vol - depth[:, None] - turn
        side = np.where(th > 0, np.clip((deg - 118) / 20, 0, 1), np.clip((ad - 30) / 25, 0, 1))
        w = side * np.clip((1.69 - z) / 0.05, 0, 1)
        w[:, 0] = 0
        # outward everywhere; at the ends both ways, so they really turn under
        hemw = np.clip(1 - tip / 0.04, 0, 1)
        d = R - r
        push = np.where(d > 0, d, d * hemw * 0.8) * w
        out[..., 0] += dx / r * push
        out[..., 1] += dy / r * push
        if smooth_k and it < iters - 1:
            out = hg.relax(out, iters=1, k=smooth_k)
    return out


def shell(p, s, ctx):
    return 0.0015 + field(p, ctx["lab"]) * smooth(0.0, 0.03, s * 0.34)


groots, gn = head.sample(GUIDES, density, seed=13)
# the sideburn is small: give its tucked and hanging parts their own guides
ex, en = head.sample(6000, density, seed=17)
exl = [label(Vector(r)) for r in ex]
sel = [i for i, l in enumerate(exl) if l == "S"][:60] + [i for i, l in enumerate(exl) if l == "W"][:24]
print("sideburn guides", len(sel))
groots, gn = np.concatenate([groots, ex[sel]]), np.concatenate([gn, en[sel]])
N = 40
guides, glab = [], []
for r, n in zip(groots, gn):
    p = Vector(r)
    ctx, L = guide_ctx(p, Vector(n))
    pts = hg.grow(head, p, n, want, L, 0.004, shell, stiff=0.72, ctx=ctx)
    guides.append(hg.resample(pts, N))
    glab.append(ctx["lab"])
guides = hg.relax(np.array(guides), iters=6)
print("guides", len(guides), round(time.time() - t0, 1))

if os.environ.get("GUIDES_ONLY"):
    RH = head_radius_table()
    gel = np.array([hg.azel(Vector(r))[1] for r in groots])
    guides = envelope(guides, 0.020 * (1 - np.clip((gel + 20) / 55, 0, 1)) * rng.uniform(0.4, 1.0, len(groots)))
    hg.make_curves("Guides", guides, r_root=2.5e-4, r_tip=1.5e-4, material=hg.hair_material(color=(0.05, 0.03, 0.02)))
    hg.setup_render(samples=16, res=(600, 660))
    hg.render_views(out + "/bobG", views=VIEWS)
    hg.save_grid([hg.load_px(f"{out}/bobG-{v}.png") for v in VIEWS], f"{out}/bobG-sheet.png", cols=4)
    raise SystemExit
roots, cn = head.sample(KIDS, density, seed=29)
clab = [label(Vector(r)) for r in roots]
clab = [l if l in glab else "S" for l in clab]
print("labels", {l: clab.count(l) for l in set(clab)})
kids, near = hg.children(guides, groots, glab, roots, clab, k=4)
RH = head_radius_table()
rel = np.array([hg.azel(Vector(r))[1] for r in roots])
depth = 0.020 * (1 - np.clip((rel + 20) / 55, 0, 1)) * rng.uniform(0.4, 1.0, len(roots))
kids = envelope(kids, depth)
print("envelope", round(time.time() - t0, 1))
fr = np.array([(r[1] - C.y) < 0.0 and r[0] < PART_X and hg.azel(Vector(r))[1] > 12 for r in roots])
kids, keep, owner = hg.leader_clump(kids, roots, clab, PIECES, rng, strength=(0.55, 0.92), power=np.where(fr, 1.2, 1.6),
                             fan=0.003, jag=(0.9, 1.0), cw_scale=np.where(fr, 1.2, 1.0), ret_owner=True)
keep = np.where(fr, keep * rng.uniform(0.8, 1.0, len(roots)), keep)
kids = hg.trim(kids, keep)
kids = envelope(kids, depth, iters=3, smooth_k=0.2)
# layered pieces: each piece sits a little in or out of its neighbours, so the cut lines read
pr = rng.normal(0, 1, len(roots))[owner]
hz = kids - np.array([0.0, 0.02, 0.0])
hz[..., 2] = 0
hz /= np.linalg.norm(hz, axis=2, keepdims=True) + 1e-9
kids = kids + hz * (pr[:, None, None] * 0.004 * np.linspace(0, 1, kids.shape[1])[None, :, None] ** 0.8)
amp = np.where(rng.random(len(roots)) < 0.02, 0.009, 0.00012)
kids = hg.frizz(kids, amp, rng, power=2.0, freq=(1, 3))
kids = hg.pushout(head, kids, frac=0.6)
kids = hg.relax(kids, iters=3, k=0.4)
print("kids", kids.shape, round(time.time() - t0, 1))

mat = hg.hair_material(color=COLOR, coat=0.0, rough=float(os.environ.get("ROUGH", 0.3)), radial=0.35, grad=(0.6, 1.1), rnd=0.3)
if os.environ.get("DBG"):
    cl = np.array(clab)
    m = ((kids[..., 0] > 0.05) & (kids[..., 1] < 0.015) & (kids[..., 2] < 1.575)).any(axis=1)
    for l in "RLSW":
        mm = m & (cl == l)
        print("veil", l, mm.sum())
        if mm.sum():
            pts = kids[mm]
            box = (pts[..., 0] > 0.05) & (pts[..., 1] < 0.015) & (pts[..., 2] < 1.575)
            q = pts[box]
            print("  pts x", np.percentile(q[:, 0], [5, 50, 95]).round(3), "y", np.percentile(q[:, 1], [5, 50, 95]).round(3),
                  "z", np.percentile(q[:, 2], [5, 50, 95]).round(3), "idx", np.percentile(np.nonzero(box)[1], [5, 50, 95]))
        if mm.sum():
            rr = roots[mm]
            az = np.degrees(np.arctan2(rr[:, 0], -(rr[:, 1] - 0.02)))
            el = np.degrees(np.arctan2(rr[:, 2] - 1.585, np.hypot(rr[:, 0], rr[:, 1] - 0.02)))
            print("  root az", np.percentile(az, [5, 50, 95]).round(0), "el", np.percentile(el, [5, 50, 95]).round(0),
                  "x", np.percentile(rr[:, 0], [5, 50, 95]).round(3), "ly", np.percentile(rr[:, 1] - 0.02, [5, 50, 95]).round(3))
    raise SystemExit
if os.environ.get("ONLY"):
    sel = np.array([l in os.environ["ONLY"].split(",") for l in clab])
    kids = kids[sel]
hg.make_curves("Hair_bob", kids, r_root=6e-5, r_tip=2.5e-5, material=mat)
head.scalp_cap(density)
hg.setup_render(samples=int(os.environ.get("SPP", 96)))
hg.render_views(out + "/bob", views=VIEWS)
hg.sheet("bob", out)
if SAVE:
    import bpy
    bpy.ops.wm.save_as_mainfile(filepath=SAVE, compress=True)
print("done", round(time.time() - t0, 1))
