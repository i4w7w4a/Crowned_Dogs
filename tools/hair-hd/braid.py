"""03 Braid: centre part, hair pulled back loosely over the ear tops, face-framing strands to the chin,
gathered behind/below her left ear into a three-strand plait that falls over the left shoulder,
black elastic near the end, short flared tail."""
import sys, os, math, time
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import numpy as np
from mathutils import Vector, Matrix, noise
import hairgen as hg
from hairgen import smooth, C, DOWN

args = sys.argv[sys.argv.index("--") + 1:]
out = args[0]
GUIDES = int(args[1]) if len(args) > 1 else 1500
KIDS = int(args[2]) if len(args) > 2 else 50000
VIEWS = args[3].split(",") if len(args) > 3 else ["front", "back", "left", "right"]
SAVE = args[4] if len(args) > 4 else None
rng = np.random.default_rng(3)
COLOR = tuple(float(x) for x in os.environ.get("HAIRCOL", "0.165,0.1,0.058").split(","))
PER_STRAND = int(os.environ.get("PER_STRAND", 3200))
t0 = time.time()
head = hg.Head()


def V(*a):
    return Vector(a)


G = head.on((0.070, 0.074, 1.522), 0.013)              # gather point behind/below the left ear
BEHIND_R = head.on((-0.050, 0.098, 1.530), 0.008)      # right-side hair passes low round the back to G
CROWN_LY = 0.045                                        # the centre part ends at the crown


def label(p):
    az, el = hg.azel(p)
    if 28 < abs(az) < 64 and 16 < el < 40 and noise.noise(p * 60.0) > -0.1:
        return "F"
    if p.y - C.y > CROWN_LY or abs(az) > 115:
        return "B"            # behind the crown: one fan converging on G, no part down the back
    return "L" if p.x > 0 else "R"


def guide_ctx(p, n):
    az, el = hg.azel(p)
    lab = label(p)
    sx = 1.0 if p.x > 0 else -1.0
    ctx = {"az": az, "el": el, "lab": lab, "rot": 0.18 * noise.noise(p * 30.0) + rng.normal(0, 0.04),
           "stick": 0.6, "vol": 0.006 + 0.007 * smooth(20, 60, el) + 0.007 * smooth(35, 85, abs(az)) * smooth(-15, 25, el)}
    j = V(*rng.normal(0, 0.003, 3))
    # the band where hair passes over the ear top: higher roots pass higher, front roots further forward
    ear = head.on((sx * 0.092, 0.022 + 0.25 * (p.y - C.y + 0.04), 1.566 + 0.30 * (p.z - 1.63)), 0.008) + j
    if lab == "F":
        ctx["aims"] = [head.on((sx * 0.080, -0.036, 1.500), 0.007) + j]
        ctx["aim"] = head.on((sx * 0.086, -0.044, 1.572), 0.007) + j
        ctx["extra"] = rng.uniform(0.0, 0.02)
        ctx["zstop"] = 1.468 + rng.normal(0, 0.008)
        ctx["stick"] = 0.0
        ctx["hang"] = True
        return ctx, rng.uniform(0.14, 0.2)
    aims = []
    if abs(az) < 110 and el > -5:
        aims.append(ear)
    if lab == "R":
        aims.append(BEHIND_R + j * 1.5 + V(0, 0, 0.3 * (p.z - 1.62)))
    aims.append(G + j * 1.5)
    ctx["aim"] = aims.pop(0)
    ctx["aims"] = aims
    ctx["extra"] = 0.006
    return ctx, 0.42


def want(p, s, prev, n, ctx):
    if "left" in ctx and ctx.get("hang"):
        return (prev * 0.4 + DOWN * 0.6).normalized()
    v = prev if "left" in ctx else (ctx["aim"] - p).normalized()
    v = (Matrix.Rotation(ctx["rot"], 3, n) @ v).normalized()
    g = 0.04 if not ctx.get("hang") else 0.15
    if not ctx.get("hang"):
        # leave the scalp and lift off into the gather only in the last few centimetres
        ctx["stick"] = 0.6 * smooth(0.02, 0.05, (G - p).length)
    return (v * (1 - g) + DOWN * g).normalized()


def shell(p, s, ctx):
    return 0.0015 + ctx["vol"] * smooth(0.0, 0.03, s * 0.42)


groots, gn = head.sample(GUIDES, hg.scalp_density, seed=17)
N = 48
guides, glab = [], []
for r, n in zip(groots, gn):
    p = Vector(r)
    ctx, L = guide_ctx(p, Vector(n))
    pts = hg.grow(head, p, n, want, L, 0.004, shell, stiff=0.5, ctx=ctx)
    guides.append(hg.resample(pts, N))
    glab.append(ctx["lab"])
guides = hg.relax(np.array(guides), iters=6)
print("guides", len(guides), round(time.time() - t0, 1))

# ---- braid path: a rope hanging from G, draped over the left shoulder in front ----
M, LR = 64, 0.32
seg = LR / (M - 1)
d0 = V(0.28, -0.55, -1.0).normalized()
X = np.array([G + d0 * seg * i for i in range(M)])
Xp = X.copy()
F = np.array([0.3, -1.6, -9.8]) * 1e-4
RAD = 0.012


def collide(q):
    for bvh in (head.body, head.bvh):
        loc, nn, _, _ = bvh.find_nearest(Vector(q), 0.1)
        if loc is not None:
            off = (Vector(q) - loc).dot(nn)
            if off < RAD:
                q = np.array(Vector(q) + nn * (RAD - off))
    return q


for it in range(500):
    v = (X - Xp) * 0.9
    Xp = X.copy()
    X = X + v + F
    for _ in range(24):
        X[0] = np.array(G)
        X[1] = np.array(G + d0 * seg)
        for i in range(M - 1):
            d = X[i + 1] - X[i]
            l = np.linalg.norm(d)
            corr = d * (1 - seg / l) * 0.5
            if i > 1:
                X[i] += corr
                X[i + 1] -= corr
            else:
                X[i + 1] -= corr * 2
        for i in range(M - 2):
            d = X[i + 2] - X[i]
            l = np.linalg.norm(d)
            rest = 2 * seg * 0.985
            if l < rest:
                corr = d * (1 - rest / l) * 0.25
                if i > 1:
                    X[i] += corr
                X[i + 2] -= corr
    for i in range(2, M):
        X[i] = collide(X[i])
X = hg.relax(X[None], iters=4, k=0.5)[0]
# the rope may still stretch a little under gravity: keep its shape, cut it back to its length
arc = np.concatenate([[0], np.cumsum(np.linalg.norm(np.diff(X, axis=0), axis=1))])
ss_ = np.linspace(0, min(LR, arc[-1]), M)
X = np.stack([np.interp(ss_, arc, X[:, k]) for k in range(3)], 1)
print("rope", X[-1].round(3), round(time.time() - t0, 1))

# dense arc-length samples with frames: B faces away from the body, Nw spans the braid's width
K = 400
arc = np.concatenate([[0], np.cumsum(np.linalg.norm(np.diff(X, axis=0), axis=1))])
ss = np.linspace(0, arc[-1], K)
P = np.stack([np.interp(ss, arc, X[:, k]) for k in range(3)], 1)
T = np.gradient(P, axis=0)
T /= np.linalg.norm(T, axis=1, keepdims=True)
B = np.zeros_like(P)
for i in range(K):
    loc, nn, _, _ = head.body.find_nearest(Vector(P[i]), 0.2)
    B[i] = nn if loc is not None else (0, -1, 0)
for _ in range(30):
    B[1:-1] = (B[:-2] + B[2:] + B[1:-1]) / 3
B -= T * np.sum(B * T, 1, keepdims=True)
B /= np.linalg.norm(B, axis=1, keepdims=True)
Nw = np.cross(T, B)
LB = ss[-1]
TIE0, TIE1 = 0.205, 0.218
u = ss / TIE0
period = 0.082 - 0.022 * np.clip(u, 0, 1)
phi = np.concatenate([[0], np.cumsum(2 * np.pi / period[1:] * np.diff(ss))])
conv = np.array([1 - smooth(TIE0 - 0.022, TIE0, a) for a in ss])          # strands merge into the tie
wid = (0.0160 - 0.0050 * np.clip(u, 0, 1)) * conv
dep = (0.0110 - 0.0030 * np.clip(u, 0, 1)) * conv
rs = 0.0076 - 0.0021 * np.clip(u, 0, 1)
start = np.array([smooth(0.0, 0.03, a) for a in ss])
rs = rs * (1.25 - 0.25 * start)
tail = np.array([smooth(TIE1, TIE1 + 0.02, a) for a in ss])
gather = np.array([smooth(TIE0 - 0.02, TIE0, a) * (1 - smooth(TIE1, TIE1 + 0.004, a)) for a in ss])
rad = rs * (1 - gather * 0.25) + tail * 0.0045                              # squeezed at the tie, flared below

bh = []
nb = PER_STRAND
for k in range(3):
    ph = phi + 2 * np.pi * k / 3
    cen = P + Nw * (wid * np.sin(ph))[:, None] + B * (dep * 0.5 * np.sin(2 * ph))[:, None]
    # each strand is a flat band crossing the braid diagonally: wide in the braid's plane across its own
    # direction, thin toward the body; below the tie the bands are one round bundle
    ts = np.gradient(cen, axis=0)
    ts /= np.linalg.norm(ts, axis=1, keepdims=True)
    wa = np.cross(B, ts)
    wa /= np.linalg.norm(wa, axis=1, keepdims=True)
    ta = np.cross(ts, wa)
    flat = conv[:, None]
    WA = wa * flat + Nw * (1 - flat)
    TA = ta * flat + B * (1 - flat)
    sw = 1.35 * conv + 1.0 * (1 - conv)
    st = 0.55 * conv + 1.0 * (1 - conv)
    th = rng.uniform(0, 2 * np.pi, nb)
    rr = np.sqrt(rng.uniform(0, 1, nb))
    for h in range(nb):
        drift = 0.18 * np.sin(ss * rng.uniform(20, 60) + rng.uniform(0, 6.3))
        a = th[h] + drift
        ou = rr[h] * np.cos(a) * sw
        ov = rr[h] * np.sin(a) * st
        q = cen + (WA * ou[:, None] + TA * ov[:, None]) * rad[:, None]
        end = TIE1 + rng.uniform(0.05, 0.088)
        n_ = np.searchsorted(ss, end)
        bh.append(hg.resample(q[:n_], 70))
bh = np.array(bh)
bh = hg.frizz(bh, np.where(rng.random(len(bh)) < 0.01, 0.003, 0.00012), rng, power=1.0, freq=(2, 6))
print("braid", bh.shape, round(time.time() - t0, 1))

# elastic: two rubber turns round the braid at the tie
ring_v, ring_f = [], []
a0, a1 = np.searchsorted(ss, TIE0), np.searchsorted(ss, TIE1)
NA = 28
rows = list(range(a0, a1 + 1))
for j, i in enumerate(rows):
    f = (ss[i] - ss[a0]) / max(1e-6, ss[a1] - ss[a0])
    bump = 0.0009 * abs(math.sin(f * 2 * math.pi))
    r0 = rad[i] * 1.32 + 0.0012 + bump
    for t in range(NA):
        a = 2 * math.pi * t / NA
        ring_v.append(tuple(P[i] + (Nw[i] * math.cos(a) * 1.15 + B[i] * math.sin(a) * 0.85) * r0))
for j in range(len(rows) - 1):
    for t in range(NA):
        a, b = j * NA + t, j * NA + (t + 1) % NA
        ring_f.append((a, b, b + NA, a + NA))
for j, cap in ((0, False), (len(rows) - 1, True)):
    ring_f.append(tuple(j * NA + t for t in (range(NA) if cap else range(NA - 1, -1, -1))))

if os.environ.get("GUIDES_ONLY"):
    hg.make_curves("Guides", guides, r_root=2.5e-4, r_tip=1.5e-4, material=hg.hair_material(color=(0.05, 0.03, 0.02)))
    hg.make_curves("BraidG", bh[::40], r_root=2.5e-4, r_tip=1.5e-4, material=hg.hair_material(name="bg", color=(0.08, 0.03, 0.02)))
    hg.setup_render(samples=16, res=(600, 660))
    hg.render_views(out + "/braidG", views=VIEWS, target=(0.0, 0.0, 1.49), dist=1.05)
    hg.save_grid([hg.load_px(f"{out}/braidG-{v}.png") for v in VIEWS], f"{out}/braidG-sheet.png", cols=4)
    raise SystemExit

roots, cn = head.sample(KIDS, hg.scalp_density, seed=31)
clab = [label(Vector(r)) for r in roots]
# the part only splits the hair in front of the crown: behind it every strand blends with its neighbours
# (all of them end in G), so there is no seam at the crown or behind the ears
OK = {"F": {"F"}, "L": {"L", "B"}, "R": {"R", "B"}, "B": {"L", "R", "B"}}
from mathutils.kdtree import KDTree
Gs = np.asarray(guides)
rel = Gs - Gs[:, :1]
trees = {}
for lab, allow in OK.items():
    idx = [i for i, l in enumerate(glab) if l in allow]
    t = KDTree(len(idx))
    for jj, i in enumerate(idx):
        t.insert(groots[i], jj)
    t.balance()
    trees[lab] = (t, idx)
kids = np.zeros((len(roots), Gs.shape[1], 3))
for c, r in enumerate(roots):
    t, idx = trees[clab[c]]
    found = t.find_n(r, 4)
    ids = [idx[f[1]] for f in found]
    w = 1.0 / (np.array([f[2] for f in found]) + 1e-4) ** 2
    kids[c] = r + np.tensordot(w / w.sum(), rel[ids], axes=1)
cl2 = [l if l in ("F", "R") else "H" for l in clab]     # clumps never bridge the part
kids, keep = hg.leader_clump(kids, roots, cl2, 1100, rng, strength=(0.45, 0.75), power=1.8, fan=0.0015, jag=(0.95, 1.0))
fr = np.array([c == "F" for c in clab])
keep = np.where(fr, keep * rng.uniform(0.75, 1.0, len(roots)), 1.0)
kids = hg.trim(kids, keep)
amp = np.where(rng.random(len(roots)) < 0.035, 0.011, 0.00012)
kids = hg.frizz(kids, amp, rng, power=2.0, freq=(1, 3))
kids = hg.pushout(head, kids, frac=0.85)
kids = hg.relax(kids, iters=3, k=0.4)
print("kids", kids.shape, round(time.time() - t0, 1))

import bpy
mat = hg.hair_material(color=COLOR, coat=0.0, rough=float(os.environ.get("ROUGH", 0.3)), radial=0.3, grad=(0.7, 1.1), rnd=0.16)
hg.make_curves("Hair_braid_scalp", kids, r_root=6e-5, r_tip=2.5e-5, material=mat)
hg.make_curves("Hair_braid_plait", bh, r_root=6e-5, r_tip=2.5e-5, material=mat)
me = bpy.data.meshes.new("Braid_tie")
me.from_pydata(ring_v, [], ring_f)
me.update()
for poly in me.polygons:
    poly.use_smooth = True
tm = bpy.data.materials.new("Braid_tie")
tm.use_nodes = True
tm.node_tree.nodes.clear()
bs = tm.node_tree.nodes.new("ShaderNodeBsdfPrincipled")
tm.node_tree.links.new(bs.outputs[0], tm.node_tree.nodes.new("ShaderNodeOutputMaterial").inputs[0])
bs.inputs["Base Color"].default_value = (0.012, 0.012, 0.013, 1)
bs.inputs["Roughness"].default_value = 0.42
me.materials.append(tm)
tie = bpy.data.objects.new("Braid_tie", me)
bpy.context.scene.collection.objects.link(tie)
head.scalp_cap(hg.scalp_density)
hg.setup_render(samples=int(os.environ.get("SPP", 96)))
hg.render_views(out + "/braid", views=[v for v in VIEWS if v != "back"], target=(0.0, 0.0, 1.49), dist=1.05)
if "back" in VIEWS:
    hg.render_views(out + "/braid", views=["back"], target=(0.0, 0.0, 1.53), dist=1.0)
hg.sheet("braid", out)
if SAVE:
    bpy.ops.wm.save_as_mainfile(filepath=SAVE, compress=True)
print("done", round(time.time() - t0, 1))
