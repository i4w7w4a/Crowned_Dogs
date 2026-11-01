"""m4 Dreads: about 110 shoulder-length locs from square sections. Each loc is a centre path grown like a guide
(out from the crown, then down), and a bundle of twisted hairs packed round it: a root patch that gathers into
the loc within a centimetre, a lumpy body, a rounded end and a little fuzz. Run with PERSONA=male on
3d/hair-hd/base_male.blend."""
import sys, os, math, time
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import numpy as np
from mathutils import Vector, Matrix, noise
import hairgen as hg
import male
from hairgen import smooth, DOWN

C = hg.C
args = sys.argv[sys.argv.index("--") + 1:]
out = args[0]
LOCS = int(args[1]) if len(args) > 1 else 125
PER = int(args[2]) if len(args) > 2 else 320
VIEWS = args[3].split(",") if len(args) > 3 else ["front", "left", "back"]
SAVE = args[4] if len(args) > 4 else None
rng = np.random.default_rng(9)
COLOR = tuple(float(x) for x in os.environ.get("HAIRCOL", "0.06,0.036,0.021").split(","))
R_LOC = float(os.environ.get("RLOC", 0.0048))
TWIST = float(os.environ.get("TWIST", 38.0))   # turns per metre
N = 44
t0 = time.time()
head = hg.Head()
WHORL = Vector((C.x + 0.005, C.y + 0.06, C.z + 0.095))


def V(*a):
    return Vector(a)


def spaced_roots(count):
    """Roughly even loc sections: dart-throw on dense samples with a minimum spacing."""
    cand, cn = head.sample(count * 40, hg.scalp_density, seed=31)
    area = 0.0
    keep, kn = [], []
    d = 0.0175
    for tries in range(6):
        keep, kn = [], []
        for p, n in zip(cand, cn):
            if all(np.linalg.norm(p - q) > d for q in keep):
                keep.append(p)
                kn.append(n)
        if len(keep) >= count:
            break
        d *= 0.93
    return np.array(keep[:count]), np.array(kn[:count])


def guide_ctx(p, n):
    az, el = hg.azel(p)
    ly = p.y - C.y
    top = smooth(10, 45, el)
    front = smooth(0.0, -0.07, ly)
    sx = 1.0 if p.x > 0 else -1.0
    radial = p - WHORL
    radial.z = 0
    crown = radial.normalized() if radial.length > 1e-5 else V(0, -1, 0)
    # the front half parts from the middle and falls to either side, back past the temples, never over the face
    comb = crown.lerp(V(sx * 0.6, 0.95, -0.3).normalized(), smooth(0.045, -0.01, ly))
    # the sides fall back past the ear so the face stays open from the side
    side = smooth(25, 60, abs(az)) * smooth(150, 110, abs(az)) * (1 - 0.3 * top)
    comb = comb.lerp(V(sx * 0.2, 1.0, -0.6).normalized(), side)
    ctx = {"az": az, "el": el, "lab": "A", "zstop": None,
           "rot": 0.3 * noise.noise(p * 20.0) + rng.normal(0, 0.1), "comb": comb.normalized()}
    a = math.radians(20 + 30 * top * smooth(-0.01, 0.04, ly))
    ct = (comb - n * comb.dot(n)).normalized()
    ctx["d0"] = (ct * math.cos(a) + n * math.sin(a)).normalized()
    ctx["g0"], ctx["g1"] = 0.15, 0.85
    ctx["sh"] = R_LOC + 0.002 + 0.008 * top * (1 - 0.6 * front)
    L = (0.175 + 0.025 * smooth(90, 170, abs(az)) - 0.025 * front * top) * rng.uniform(0.88, 1.1)
    return ctx, L


def want(p, s, prev, n, ctx):
    if s == 0.0:
        return ctx["d0"]
    c = Matrix.Rotation(ctx["rot"] * (0.3 + s), 3, n) @ ctx["comb"]
    ct = c - n * c.dot(n)
    if ct.length < 1e-4:
        ct = DOWN - n * DOWN.dot(n)
    g = min(1.0, ctx["g0"] + ctx["g1"] * smooth(0.0, 0.35, s))
    return (ct.normalized() * (1 - g) + DOWN * g).normalized()


def shell(p, s, ctx):
    return 0.0015 + ctx["sh"] * smooth(0.0, 0.12, s)


roots, rn = spaced_roots(LOCS)
paths, lens = [], []
for r, n in zip(roots, rn):
    ctx, L = guide_ctx(Vector(r), Vector(n))
    pts = hg.grow(head, Vector(r), Vector(n), want, L, 0.004, shell, stiff=0.8, ctx=ctx)
    paths.append(hg.resample(pts, N))
    lens.append(L)
paths = np.array(paths)
# a slow sway along each loc
paths = hg.frizz(paths, np.full(len(paths), 0.010), rng, power=1.6, freq=(0.8, 1.6))
lens = np.array(lens)
print("locs", len(paths), round(time.time() - t0, 1))

# locs do not pass through each other: push apart points of different locs closer than two loc radii
for it in range(12):
    P = paths[:, 4:].reshape(-1, 3)
    owner = np.repeat(np.arange(len(paths)), N - 4)
    push = np.zeros_like(P)
    for i in range(0, len(P), 600):
        d = P[i:i + 600, None, :] - P[None, :, :]
        dist = np.linalg.norm(d, axis=2)
        mask = (dist < 2.1 * R_LOC) & (owner[i:i + 600, None] != owner[None, :]) & (dist > 1e-6)
        f = np.where(mask, (2.1 * R_LOC - dist) / np.maximum(dist, 1e-6), 0.0)
        push[i:i + 600] = (d * f[..., None]).sum(1) * 0.25
    paths[:, 4:] += push.reshape(len(paths), N - 4, 3)
    paths = hg.pushout(head, paths, frac=1.0, minoff=R_LOC + 0.0015, body=True)
    paths = hg.relax(paths, iters=2, k=0.4)
print("relaxed", round(time.time() - t0, 1))


def frames(path):
    """Parallel-transport frames along a path."""
    T = np.gradient(path, axis=0)
    T /= np.linalg.norm(T, axis=1, keepdims=True)
    u = np.cross(T[0], [0.0, 0.0, 1.0])
    if np.linalg.norm(u) < 1e-3:
        u = np.cross(T[0], [1.0, 0.0, 0.0])
    u /= np.linalg.norm(u)
    U = [u]
    for i in range(1, len(T)):
        u = U[-1] - T[i] * np.dot(U[-1], T[i])
        u /= np.linalg.norm(u)
        U.append(u)
    U = np.array(U)
    return U, np.cross(T, U)


s = np.linspace(0, 1, N)
strands = []
for k, (path, L) in enumerate(zip(paths, lens)):
    U, W = frames(path)
    l = s * L
    seed = rng.random() * 100
    lump = np.array([1 + 0.18 * noise.noise(Vector((li * 55.0, seed, 0.0))) for li in l])
    R = R_LOC * rng.uniform(0.8, 1.15) * lump
    R = R + 0.0045 * np.exp(-l / 0.006)                       # the section at the root gathers into the loc
    R = R * np.sqrt(np.clip((1.0 - s) / 0.04, 0.05, 1.0))    # rounded end
    tw = TWIST * rng.uniform(0.8, 1.2) * (1 if rng.random() < 0.5 else -1)
    a0 = rng.random(PER) * 2 * np.pi
    rho = np.sqrt(rng.random(PER)) * 0.95
    fuzz = rng.random(PER) < 0.06
    rho[fuzz] = rng.uniform(1.0, 1.5, fuzz.sum())
    ang = a0[:, None] + tw * 2 * np.pi * l[None, :]
    off = (rho[:, None, None] * R[None, :, None]) * (np.cos(ang)[..., None] * U[None] + np.sin(ang)[..., None] * W[None])
    hs = path[None] + off
    # fuzz hairs wander a little off the loc
    hs[fuzz] += rng.normal(0, 0.0012, (fuzz.sum(), N, 3)) * s[None, :, None] ** 0.5
    strands.append(hs)
strands = np.concatenate(strands)
strands = hg.pushout(head, strands, frac=0.25, minoff=0.0008)
print("hairs", strands.shape, round(time.time() - t0, 1))

mat = hg.hair_material(color=COLOR, coat=0.0, rough=float(os.environ.get("ROUGH", 0.42)), radial=0.4, grad=(0.6, 1.35), rnd=0.25)
hg.make_curves("Hair_dreads", strands, r_root=8e-5, r_tip=6e-5, material=mat)
head.scalp_cap(hg.scalp_density, color=(0.05, 0.035, 0.025))
hg.setup_render(samples=int(os.environ.get("SPP", 64)))
hg.render_views(out + "/dreads", views=VIEWS)
male.sheet("dreads", out, views=[v for v in VIEWS if v in male.COLS])
if SAVE:
    import bpy
    bpy.ops.wm.save_as_mainfile(filepath=SAVE, compress=True)
print("done", round(time.time() - t0, 1))
