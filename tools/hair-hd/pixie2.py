"""01 Pixie: side part on her left, long fringe swept across to her right, lifted tousled top,
short sides (left ear open), tapered nape."""
import sys, os, math, time
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import numpy as np
from mathutils import Vector, Matrix, noise
import hairgen as hg
from hairgen import smooth, C, DOWN

args = sys.argv[sys.argv.index("--") + 1:]
out = args[0]
GUIDES = int(args[1]) if len(args) > 1 else 1500
KIDS = int(args[2]) if len(args) > 2 else 30000
VIEWS = args[3].split(",") if len(args) > 3 else ["front", "back", "left", "right"]
SAVE = args[4] if len(args) > 4 else None
rng = np.random.default_rng(7)
PIECES = int(os.environ.get("PIECES", 700))
COLOR = tuple(float(x) for x in os.environ.get("HAIRCOL", "0.156,0.089,0.05").split(","))
t0 = time.time()
head = hg.Head()

PART_X = 0.032          # side part on her left, front half of the head only
PART_END = -0.005       # part runs from the hairline back to here (ly)
TEMPLE_R = head.on((-0.102, 0.000, 1.605), 0.030)
LEFT_EAR = head.on((0.098, 0.050, 1.606), 0.020)
TEMPLE_L = head.on((0.096, -0.028, 1.582), 0.014)
# fringe waypoints: across the forehead (w1), then down to the right eye corner / cheekbone (w2)
W1_R, W1_P = head.on((-0.045, -0.09, 1.628), 0.010), head.on((0.004, -0.095, 1.636), 0.012)
W2_R, W2_P = head.on((-0.085, -0.058, 1.572), 0.007), head.on((-0.066, -0.080, 1.552), 0.005)
B1_R, B1_P = head.on((-0.05, -0.01, 1.67), 0.030), head.on((-0.01, -0.03, 1.68), 0.036)
WHORL = Vector((0.018, 0.075, 1.680))


def V(*a):
    return Vector(a)


def label(p):
    ly = p.y - C.y
    az, el = hg.azel(p)
    if ly < PART_END and p.x > PART_X and el > 10:
        return "L"
    return "R"


def swirl(p, comb):
    # around the crown whorl the hair spirals out instead of parting: front of it goes right, behind it left/down
    r = p - WHORL
    r.z = 0
    d = r.length
    if d < 1e-5:
        return comb
    sw = (r.normalized() * 0.45 + V(r.y, -r.x, 0).normalized()).normalized() + V(0, 0, -0.3)
    return comb.normalized().lerp(sw.normalized(), smooth(0.04, 0.012, d))


def stand(ctx, n, comb, lift_deg, p=None):
    """Short stiff hair: leaves the scalp at lift_deg toward comb, then levels off and bends under gravity."""
    if p is not None:
        comb = swirl(p, comb)
    ct = (comb - n * comb.dot(n)).normalized()
    a = math.radians(max(2.0, lift_deg))
    ctx["d0"] = (ct * math.cos(a) + n * math.sin(a)).normalized()
    ctx["comb"] = comb.normalized()
    ctx["stiff"] = 0.88
    ctx["stand"] = True


def guide_ctx(p, n):
    az, el = hg.azel(p)
    ly = p.y - C.y
    lab = label(p)
    top = smooth(14, 36, el)
    ctx = {"az": az, "el": el, "ly": ly, "lab": lab, "aim": None, "top": top,
           "rot": 0.55 * noise.noise(p * 38.0) + rng.normal(0, 0.12),
           "flick": (Vector(noise.noise_vector(p * 45.0)) + n * 0.5 * top).normalized(),
           "fl": rng.uniform(0.2, 0.8), "g0": 0.08, "g1": 0.35, "stick": 0.0}
    jit = 12.0 * noise.noise(p * 31.0 + V(5, 1, 2)) + rng.normal(0, 6.0)
    if lab == "L":
        # from the part the hair falls back and down over the temple to the top of the ear
        tgt = TEMPLE_L.lerp(LEFT_EAR, smooth(-0.075, -0.015, ly)) + V(*rng.normal(0, 0.006, 3))
        L = max(0.03, (tgt - p).length * 1.1 + rng.uniform(0.0, 0.015))
        stand(ctx, n, (tgt - p) + V(0, 0, -0.01), 18 + 26 * top + jit * 0.6)
    elif ly < 0.06 and el > 14:
        t = smooth(-0.055, PART_X, p.x)            # 1 at the part, 0 at her right temple
        f = smooth(0.03, -0.045, ly)              # 1 at the hairline, 0 near the crown
        ctx["fringe"] = f > 0.45
        if ctx["fringe"]:
            w1 = W1_R.lerp(W1_P, t)
            w2 = W2_R.lerp(W2_P, t)
            ctx["aim"] = w1 + V(*rng.normal(0, 0.005, 3))
            ctx["aims"] = [w2 + V(*rng.normal(0, 0.008, 3))]
            ctx["extra"] = rng.uniform(0.012, 0.03)
            ctx["g0"], ctx["g1"] = 0.05, 0.5
            ctx["stick"] = 0.7
            ctx["fl"] = 0.0
            ctx["layer"] = rng.uniform(0.85, 1.0)
            L = 0.2
        else:
            # the top sweeps up from the part and over to her right temple, pieces lifted
            tgt = TEMPLE_R + V(*rng.normal(0, 0.012, 3)) + V(0, -0.02, 0) * (1 - t)
            L = min(0.085, (tgt - p).length * 0.95 + rng.uniform(0.0, 0.02))
            stand(ctx, n, tgt - p, 44 + 8 * t + jit, p)
            ctx["fl"] = rng.uniform(0.4, 1.0)
            ctx["g0"], ctx["g1"] = 0.08, 0.4
    else:
        back = smooth(100, 150, abs(az))
        upper = smooth(-5, 22, el)
        sx = -1.0 if p.x > 0 else 1.0
        # below the occiput the back is tapered: short hair lying close and combed in toward the nape
        inw = smooth(105, 130, abs(az)) * (1 - smooth(158, 176, abs(az)))
        lowb = V(sx * 0.45 * inw, 0.2, -1.0)
        if az < 0:
            low = V(-0.25, -0.45, -1.0).lerp(lowb, back)
        else:
            low = V(0.12, 0.55, -1.0).lerp(lowb, back)
        radial = (p - WHORL)
        radial.z = 0
        crown = (radial.normalized() * 0.35 + V(0, 0.5, -0.8)).normalized()
        ctx["rot"] *= 1 - 0.6 * back
        outw = V(p.x, p.y - C.y, 0).normalized()
        ctx["flick"] = (ctx["flick"] * (1 - back) + (outw * 0.6 + V(0, 0, -1)) * back).normalized()
        nape = (0.012 + 0.008 * smooth(-45, -28, el) + (0.010 + 0.016 * smooth(140, 170, abs(az))) * smooth(-28, -6, el)
                + 0.042 * upper)
        side = (0.046 if az < -35 else 0.034) * (0.35 + 0.65 * smooth(-22, 5, el))
        L = (side + (nape - side) * back) * (1 - top) + 0.076 * top
        lift = 3 + 6 * smooth(-35, -10, el) + 24 * upper
        if back < 0.5:
            lift *= 0.7 if az < 0 else 0.45
        comb = low.normalized().lerp(crown, max(top, upper * back))
        stand(ctx, n, comb, lift + jit * smooth(-30, 0, el), p)
    return ctx, L


def want(p, s, prev, n, ctx):
    if ctx.get("stand"):
        if s == 0.0:
            return ctx["d0"]
        c = (Matrix.Rotation(ctx["rot"] * (0.3 + s), 3, n) @ ctx["comb"])
        ct = c - n * c.dot(n)
        g = ctx["g0"] + ctx["g1"] * s
        w = ct.normalized() * (1 - g) + DOWN * g
    else:
        v = prev if "left" in ctx else (ctx["aim"] - p).normalized()
        v = (Matrix.Rotation(ctx["rot"] * (0.5 + s), 3, n) @ v).normalized()
        g = ctx["g0"] + ctx["g1"] * s * s
        w = v * (1 - g) + DOWN * g
    if s > 0.6:
        w += ctx["flick"] * ctx["fl"] * smooth(0.6, 1.0, s)
    return w.normalized()


def field(p):
    az, el = hg.azel(p)
    F = 0.004 + 0.010 * smooth(-42, -15, el) + 0.018 * smooth(-15, 12, el) + 0.006 * smooth(20, 45, el)
    face = smooth(1.60, 1.645, p.z) if p.y < C.y - 0.05 else 1.0
    return F * face


def shell(p, s, ctx):
    if ctx.get("stand"):
        return 0.003
    rise = smooth(0.0, 0.016, s * ctx["L"])
    return 0.0015 + max(0.0025, field(p) * ctx["layer"] * rise)


groots, gn = head.sample(GUIDES, hg.scalp_density, seed=11)
N = 28
guides, glab = [], []
for r, n in zip(groots, gn):
    p = Vector(r)
    ctx, L = guide_ctx(p, Vector(n))
    ctx["L"] = L * rng.uniform(0.88, 1.08)
    pts = hg.grow(head, p, n, want, ctx["L"], 0.004, shell, stiff=ctx.get("stiff", 0.5), ctx=ctx)
    guides.append(hg.resample(pts, N))
    glab.append(ctx["lab"])
guides = hg.relax(np.array(guides), iters=6)
print("guides", len(guides), round(time.time() - t0, 1))

if os.environ.get("STATS"):
    rows = []
    for g, r in zip(guides, groots):
        az, el = hg.azel(Vector(r))
        offs = []
        for q in g:
            loc, n, _, _ = head.bvh.find_nearest(Vector(q), 0.2)
            offs.append((Vector(q) - loc).dot(n))
        rows.append((az, el, max(offs), offs[-1], np.linalg.norm(np.diff(g, axis=0), axis=1).sum()))
    rows = np.array(rows)
    for name, m in (("top el>30", rows[:, 1] > 30), ("crown back", (rows[:, 1] > 0) & (abs(rows[:, 0]) > 110)),
                    ("right side", (rows[:, 0] < -60) & (rows[:, 0] > -110) & (rows[:, 1] < 20)),
                    ("left side", (rows[:, 0] > 60) & (rows[:, 0] < 110) & (rows[:, 1] < 20))):
        r_ = rows[m]
        print("STAT", name, len(r_), "maxoff p50/p90", np.percentile(r_[:, 2], [50, 90]).round(4), "tipoff", np.percentile(r_[:, 3], 50).round(4), "len", np.percentile(r_[:, 4], 50).round(3))
    raise SystemExit
if os.environ.get("GUIDES_ONLY"):
    hg.make_curves("Guides", guides, r_root=2.5e-4, r_tip=1.5e-4, material=hg.hair_material(color=(0.05, 0.03, 0.02)))
    hg.setup_render(samples=16, res=(600, 660))
    hg.render_views(out + "/pixieG", views=VIEWS)
    hg.save_grid([hg.load_px(f"{out}/pixieG-{v}.png") for v in VIEWS], f"{out}/pixieG-sheet.png", cols=4)
    raise SystemExit
roots, cn = head.sample(KIDS, hg.scalp_density, seed=23)
clab = [label(Vector(r)) for r in roots]
kids, near = hg.children(guides, groots, glab, roots, clab, k=3)
# pieces on the top and crown splay apart (tousled), pieces on the taper stay neat
ae = np.array([hg.azel(Vector(r)) for r in roots])
fan = 0.004 + 0.010 * np.clip((ae[:, 1] - 0.0) / 30.0, 0, 1)
fan = np.where(np.array(clab) == "L", np.maximum(fan, 0.008), fan)
fr = np.array([(r[1] - C.y) < -0.055 and r[0] < PART_X and r[2] > 1.62 for r in roots])
kids, keep = hg.leader_clump(kids, roots, clab, PIECES, rng, strength=(0.6, 0.92), power=np.where(fr, 0.9, 1.6), fan=fan, jag=(0.75, 1.0),
                             cw_scale=np.where(fr, 1.15, 1.0))
keep = np.where(fr, keep * rng.uniform(0.55, 1.0, len(roots)), keep)
kids = hg.trim(kids, keep)
amp = np.where(rng.random(len(roots)) < 0.006, 0.006, 0.00015)
kids = hg.frizz(kids, amp, rng, power=2.0, freq=(1, 3))
kids = hg.pushout(head, kids, frac=0.6)
kids = hg.relax(kids, iters=3, k=0.4)
print("kids", kids.shape, round(time.time() - t0, 1))

if os.environ.get("COLORTEST"):
    hg.make_curves("Hair_pixie", kids)
    ob = hg.bpy.data.objects["Hair_pixie"]
    hg.setup_render(samples=24, res=(450, 500))
    for i, c in enumerate(os.environ["COLORTEST"].split(";")):
        c = tuple(float(x) for x in c.split(","))
        m = hg.hair_material(name=f"T{i}", color=c, coat=0.0, rough=0.26, grad=(0.75, 1.1), rnd=0.12)
        ob.data.materials.clear()
        ob.data.materials.append(m)
        hg.render_views(f"{out}/ct{i}", views=["front"])
        a = hg.load_px(f"{out}/ct{i}-front.png", (190, 90, 310, 135))[..., :3].reshape(-1, 3)
        print("COL", c, a.mean(0).round(3), np.percentile(a, 20, 0).round(3), np.percentile(a, 80, 0).round(3))
    raise SystemExit
mat = hg.hair_material(color=COLOR, coat=0.0, rough=float(os.environ.get("ROUGH", 0.32)), radial=0.35, grad=(0.6, 1.08), rnd=0.22)
hg.make_curves("Hair_pixie", kids, r_root=6e-5, r_tip=2e-5, material=mat)
head.scalp_cap(hg.scalp_density)
hg.setup_render(samples=int(os.environ.get("SPP", 96)))
hg.render_views(out + "/pixie", views=VIEWS)
hg.sheet("pixie", out)
if SAVE:
    import bpy
    bpy.ops.wm.save_as_mainfile(filepath=SAVE, compress=True)
print("done", round(time.time() - t0, 1))
