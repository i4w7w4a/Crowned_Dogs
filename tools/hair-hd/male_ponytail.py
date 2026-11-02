"""m4 Low ponytail (Art's sketch, 2026-11-02): brown hair combed straight back from the forehead with a little
volume, a few loose strands at the temples, the sides back over the tops of the ears, gathered low at the nape
under the occiput by a black elastic of several turns; the tail hangs onto the upper back, about 15 cm below
the tie, and tapers to a point. Run with PERSONA=male on 3d/hair-hd/base_male.blend."""
import sys, os, math, time
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import numpy as np
from mathutils import Vector, Matrix, noise
import hairgen as hg
from hairgen import smooth, DOWN
import male  # noqa: F401  (the male hairline)

C = hg.C
args = sys.argv[sys.argv.index("--") + 1:]
out = args[0]
GUIDES = int(args[1]) if len(args) > 1 else 1500
KIDS = int(args[2]) if len(args) > 2 else 55000
VIEWS = args[3].split(",") if len(args) > 3 else ["front", "left", "back"]
SAVE = args[4] if len(args) > 4 else None
rng = np.random.default_rng(6)
COLOR = tuple(float(x) for x in os.environ.get("HAIRCOL", "0.05,0.03,0.017").split(","))
TAIL_N = int(os.environ.get("TAIL_N", 9000))
t0 = time.time()
head = hg.Head()


def V(*a):
    return Vector(a)


# the tie sits low on the nape, under the occiput; the tail leaves it down and a little back
G = head.on((0.003, 0.115, 1.622), 0.013)
T0 = V(0.0, 0.3, -1.0).normalized()
XA = V(1.0, 0.0, 0.0)
BA = T0.cross(XA).normalized()          # across the tail, pointing up and back
R_TIE = 0.0085
print("G", tuple(round(x, 4) for x in G))


def label(p):
    az, el = hg.azel(p)
    # a few loose strands at the temples and in front of the ears
    if 22 < abs(az) < 75 and 8 < el < 38 and noise.noise(p * 70.0) > 0.42:
        return "F"
    return "A"


def guide_ctx(p, n):
    az, el = hg.azel(p)
    ly = p.y - C.y
    sx = 1.0 if p.x > 0 else -1.0
    top = smooth(15, 45, el)
    front = smooth(0.0, -0.07, ly)
    lab = label(p)
    ctx = {"az": az, "el": el, "lab": lab, "rot": 0.12 * noise.noise(p * 30.0) + rng.normal(0, 0.03)}
    if lab == "F":
        # pulled back with the rest, then slipped out and falling down beside the face
        back = V(sx * 0.25, 0.8, -0.2).normalized()
        ctx["comb"] = back
        ct = (back - n * back.dot(n)).normalized()
        ctx["d0"] = (ct + n * 0.3).normalized()
        ctx["stick"] = 0.0
        ctx["zstop"] = 1.55
        return ctx, rng.uniform(0.07, 0.11)
    # every strand ends in the tie, keeping its place round it so the strands do not cross
    ox = 0.75 * R_TIE * max(-1.0, min(1.0, p.x / 0.075))
    oz = 0.6 * R_TIE * max(-1.0, min(1.0, (p.z - 1.66) / 0.07))
    j = V(*rng.normal(0, 0.0012, 3))
    aim = G + T0 * 0.006 + XA * ox + BA * oz + j
    aims = []
    if 35 < abs(az) < 105 and el < 40:
        # the sides go back over the tops of the ears, covering them a little
        aims.append(head.on((sx * 0.082, 0.02 + 0.3 * (ly + 0.03), 1.662 + 0.25 * (p.z - 1.68)), 0.009) + j * 2)
    aims.append(aim)
    ctx["aim"] = aims.pop(0)
    ctx["aims"] = aims
    ctx["extra"] = 0.010
    ctx["stick"] = 0.6
    # combed straight back from the forehead with a little lift at the front
    first = ctx["aim"] - p
    ct = (first - n * first.dot(n)).normalized()
    lift = math.radians(4 + 26 * front * top + 6 * top + rng.normal(0, 3))
    ctx["d0"] = (ct * math.cos(lift) + n * math.sin(lift)).normalized()
    ctx["vol"] = 0.004 + 0.009 * top + 0.008 * front * top
    return ctx, 0.40


def want(p, s, prev, n, ctx):
    if s == 0.0:
        return ctx["d0"]
    if ctx["lab"] == "F":
        g = 0.35 + 0.55 * s
        c = ctx["comb"] - n * ctx["comb"].dot(n)
        return (c.normalized() * (1 - g) + DOWN * g).normalized()
    if "left" in ctx:
        return (prev * 0.6 + T0 * 0.4).normalized()
    v = (ctx["aim"] - p).normalized()
    v = (Matrix.Rotation(ctx["rot"], 3, n) @ v).normalized()
    # lie on the head until the last few centimetres, then lift off into the tie
    ctx["stick"] = 0.6 * smooth(0.02, 0.05, (G - p).length)
    return v


def shell(p, s, ctx):
    if ctx["lab"] == "F":
        return 0.002
    return 0.0015 + ctx["vol"] * smooth(0.0, 0.05, s) * smooth(0.03, 0.08, (G - p).length)


groots, gn = head.sample(GUIDES, hg.scalp_density, seed=13)
N = 44
guides, glab = [], []
for r, n in zip(groots, gn):
    p = Vector(r)
    ctx, L = guide_ctx(p, Vector(n))
    pts = hg.grow(head, p, n, want, L, 0.004, shell, stiff=0.55, ctx=ctx)
    guides.append(hg.resample(pts, N))
    glab.append(ctx["lab"])
guides = hg.relax(np.array(guides), iters=5)
print("guides", len(guides), "loose", glab.count("F"), round(time.time() - t0, 1))

# ---- the tail: a rope hanging from the tie, settling on the neck and the upper back ----
M, LR = 48, 0.175
seg = LR / (M - 1)
X = np.array([G + T0 * seg * i for i in range(M)])
Xp = X.copy()
F = np.array([0.0, 0.0, -9.8]) * 1e-4
RAD = 0.011


def collide(q):
    for bvh in (head.body, head.bvh):
        loc, nn, _, _ = bvh.find_nearest(Vector(q), 0.1)
        if loc is not None:
            off = (Vector(q) - loc).dot(nn)
            if off < RAD:
                q = np.array(Vector(q) + nn * (RAD - off))
    return q


for it in range(400):
    v = (X - Xp) * 0.9
    Xp = X.copy()
    X = X + v + F
    for _ in range(20):
        X[0] = np.array(G)
        X[1] = np.array(G + T0 * seg)
        for i in range(M - 1):
            d = X[i + 1] - X[i]
            l = np.linalg.norm(d)
            corr = d * (1 - seg / l) * 0.5
            if i > 1:
                X[i] += corr
                X[i + 1] -= corr
            else:
                X[i + 1] -= corr * 2
        # a bundle of hair is stiff: it bends gently
        for i in range(M - 2):
            d = X[i + 2] - X[i]
            l = np.linalg.norm(d)
            rest = 2 * seg * 0.97
            if l < rest:
                corr = d * (1 - rest / l) * 0.25
                if i > 1:
                    X[i] += corr
                X[i + 2] -= corr
    for i in range(2, M):
        X[i] = collide(X[i])
X = hg.relax(X[None], iters=4, k=0.5)[0]
arc = np.concatenate([[0], np.cumsum(np.linalg.norm(np.diff(X, axis=0), axis=1))])
ss_ = np.linspace(0, min(LR, arc[-1]), M)
X = np.stack([np.interp(ss_, arc, X[:, k]) for k in range(3)], 1)
print("rope end", X[-1].round(3), round(time.time() - t0, 1))

K = 300
arc = np.concatenate([[0], np.cumsum(np.linalg.norm(np.diff(X, axis=0), axis=1))])
ss = np.linspace(0, arc[-1], K)
P = np.stack([np.interp(ss, arc, X[:, k]) for k in range(3)], 1)
T = np.gradient(P, axis=0)
T /= np.linalg.norm(T, axis=1, keepdims=True)
B = np.zeros_like(P)
for i in range(K):
    loc, nn, _, _ = head.body.find_nearest(Vector(P[i]), 0.2)
    B[i] = nn if loc is not None else (0, 1, 0)
for _ in range(30):
    B[1:-1] = (B[:-2] + B[2:] + B[1:-1]) / 3
B -= T * np.sum(B * T, 1, keepdims=True)
B /= np.linalg.norm(B, axis=1, keepdims=True)
Nw = np.cross(T, B)
LB = ss[-1]
TIE0, TIE1 = 0.002, 0.016
# squeezed in the tie, puffed just below it, then thinning to a point
flare = np.array([smooth(TIE1, TIE1 + 0.025, a) for a in ss])
thin = np.array([1 - smooth(TIE1 + 0.03, LB, a) for a in ss]) ** 0.6
rad = R_TIE * (1 + 0.9 * flare) * np.maximum(thin, 0.12)

th = rng.uniform(0, 2 * np.pi, TAIL_N)
rr = np.sqrt(rng.uniform(0, 1, TAIL_N))
tail = np.zeros((TAIL_N, 60, 3))
for h in range(TAIL_N):
    drift = 0.25 * np.sin(ss * rng.uniform(15, 45) + rng.uniform(0, 6.3))
    a = th[h] + drift
    q = P + (Nw * (rr[h] * np.cos(a))[:, None] * 1.1 + B * (rr[h] * np.sin(a))[:, None] * 0.85) * rad[:, None]
    tail[h] = hg.resample(q, 60)
# loose, slightly messy pieces; outer hairs end sooner, so the tail comes to a point
tail, keep = hg.leader_clump(tail, tail[:, 0], ["T"] * TAIL_N, 90, rng, strength=(0.45, 0.85), power=1.2, fan=0.009, jag=(0.75, 1.0))
keep = np.minimum(keep, 1 - 0.45 * rr ** 2 * rng.uniform(0.6, 1.0, TAIL_N))
tail = hg.trim(tail, keep)
tail = hg.frizz(tail, np.where(rng.random(TAIL_N) < 0.04, 0.008, 0.0015), rng, power=1.2, freq=(1, 4))
print("tail", tail.shape, round(time.time() - t0, 1))

# the elastic: three rubber turns round the tie
ring_v, ring_f = [], []
a0, a1 = np.searchsorted(ss, TIE0), np.searchsorted(ss, TIE1)
NA = 28
rows = list(range(a0, a1 + 1))
for j, i in enumerate(rows):
    f = (ss[i] - ss[a0]) / max(1e-6, ss[a1] - ss[a0])
    r0 = rad[i] * 1.18 + 0.0012 + 0.0008 * abs(math.sin(f * 3 * math.pi))
    for t in range(NA):
        a = 2 * math.pi * t / NA
        ring_v.append(tuple(P[i] + (Nw[i] * math.cos(a) * 1.1 + B[i] * math.sin(a) * 0.85) * r0))
for j in range(len(rows) - 1):
    for t in range(NA):
        a, b = j * NA + t, j * NA + (t + 1) % NA
        ring_f.append((a, b, b + NA, a + NA))
for j, cap in ((0, False), (len(rows) - 1, True)):
    ring_f.append(tuple(j * NA + t for t in (range(NA) if cap else range(NA - 1, -1, -1))))

TARGET, DIST = (0.003, 0.02, 1.60), 1.15
if os.environ.get("GUIDES_ONLY"):
    hg.make_curves("Guides", guides, r_root=2.5e-4, r_tip=1.5e-4, material=hg.hair_material(color=(0.05, 0.03, 0.02)))
    hg.make_curves("TailG", tail[::60], r_root=2.5e-4, r_tip=1.5e-4, material=hg.hair_material(name="tg", color=(0.08, 0.03, 0.02)))
    hg.setup_render(samples=16, res=(600, 660))
    hg.render_views(out + "/ponytailG", views=VIEWS, target=TARGET, dist=DIST)
    hg.save_grid([hg.load_px(f"{out}/ponytailG-{v}.png") for v in VIEWS], f"{out}/ponytailG-sheet.png", cols=len(VIEWS))
    raise SystemExit

roots, cn = head.sample(KIDS, hg.scalp_density, seed=29)
clab = [label(Vector(r)) for r in roots]
kids, near = hg.children(guides, groots, glab, roots, clab, k=3)
# combed-back pieces that stay together; the loose strands are their own thin pieces
kids, keep = hg.leader_clump(kids, roots, clab, 600, rng, strength=(0.35, 0.7), power=1.6, fan=0.003, jag=(0.97, 1.0))
fr = np.array([c == "F" for c in clab])
keep = np.where(fr, keep * rng.uniform(0.7, 1.0, len(roots)), keep)
kids = hg.trim(kids, keep)
amp = np.where(rng.random(len(roots)) < 0.05, 0.008, 0.0012)
kids = hg.frizz(kids, amp, rng, power=2.0, freq=(1, 3))
kids = hg.pushout(head, kids, frac=0.8)
kids = hg.relax(kids, iters=3, k=0.4)
print("kids", kids.shape, round(time.time() - t0, 1))

import bpy
mat = hg.hair_material(color=COLOR, coat=0.12, rough=float(os.environ.get("ROUGH", 0.28)), radial=0.35, grad=(0.6, 1.25), rnd=0.3)
hg.make_curves("Hair_ponytail", kids, r_root=6.5e-5, r_tip=2.5e-5, material=mat)
hg.make_curves("Hair_ponytail_tail", tail, r_root=6.5e-5, r_tip=2e-5, material=mat)
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
head.scalp_cap(hg.scalp_density, color=(0.045, 0.033, 0.026))
hg.setup_render(samples=int(os.environ.get("SPP", 64)))
hg.render_views(out + "/ponytail", views=VIEWS, target=TARGET, dist=DIST)
hg.save_grid([hg.load_px(f"{out}/ponytail-{v}.png") for v in VIEWS], f"{out}/ponytail-sheet.png", cols=len(VIEWS))
if SAVE:
    bpy.ops.wm.save_as_mainfile(filepath=SAVE, compress=True)
print("done", round(time.time() - t0, 1))
