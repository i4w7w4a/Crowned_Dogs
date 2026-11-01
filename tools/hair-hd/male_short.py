"""m1 Short: textured crop. Spiky pieces on top, 4-5 cm, standing up and forward with a short
lifted fringe; skin fade on the sides and nape, from stubble at the ears to ~1.5 cm at the ridge.
Run with PERSONA=male on 3d/hair-hd/base_male.blend."""
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
GUIDES = int(args[1]) if len(args) > 1 else 1500
KIDS = int(args[2]) if len(args) > 2 else 50000
VIEWS = args[3].split(",") if len(args) > 3 else ["front", "left", "back"]
SAVE = args[4] if len(args) > 4 else None
rng = np.random.default_rng(7)
PIECES = int(os.environ.get("PIECES", 900))
COLOR = tuple(float(x) for x in os.environ.get("HAIRCOL", "0.032,0.023,0.017").split(","))
t0 = time.time()
head = hg.Head()
WHORL = Vector((C.x + 0.012, C.y + 0.07, C.z + 0.09))


def V(*a):
    return Vector(a)


def fade_len(el):
    """Side and nape length: stubble low, longer toward the parietal ridge."""
    return 0.0025 + 0.011 * smooth(-34, 34, el) ** 1.5


def stand(ctx, n, comb, lift_deg):
    ct = (comb - n * comb.dot(n)).normalized()
    a = math.radians(max(2.0, lift_deg))
    ctx["d0"] = (ct * math.cos(a) + n * math.sin(a)).normalized()
    ctx["comb"] = comb.normalized()


def guide_ctx(p, n):
    az, el = hg.azel(p)
    ly = p.y - C.y
    top = smooth(32, 52, el)
    front = smooth(0.0, -0.07, ly)
    back = smooth(80, 150, abs(az))
    ctx = {"az": az, "el": el, "top": top, "lab": "A",
           "rot": 0.6 * noise.noise(p * 40.0) + rng.normal(0, 0.15),
           "flick": Vector(noise.noise_vector(p * 50.0)).normalized(), "fl": rng.uniform(0.2, 0.7)}
    jit = 10.0 * noise.noise(p * 31.0 + V(5, 1, 2)) + rng.normal(0, 7.0)
    # top: up and forward, the front pieces lifting into a short fringe; crown: out from the whorl
    radial = p - WHORL
    radial.z = 0
    crown = (radial.normalized() * 0.6 + V(0, 0.4, -0.3)).normalized() if radial.length > 1e-5 else V(0, 1, 0)
    fwd = V(0.15 * noise.noise(p * 12.0), -1.0, -0.1).normalized()
    comb_top = crown.lerp(fwd, smooth(0.05, -0.01, ly))
    sx = 1.0 if p.x > 0 else -1.0
    comb_side = V(sx * 0.15, 0.35 + 0.4 * back, -1.0).normalized()
    comb = comb_side.lerp(comb_top, top)
    L_top = 0.044 + 0.006 * front - 0.008 * back
    L = fade_len(el) * (1 - top) + L_top * top
    lift = 6 + 4 * smooth(-10, 15, el) + (46 - 6 * front - 28 * back) * top
    stand(ctx, n, comb, lift + jit * (0.3 + 0.7 * top))
    ctx["stiff"] = 0.86 + 0.06 * top
    ctx["g0"], ctx["g1"] = 0.05, 0.15 + 0.15 * (1 - top)
    return ctx, L


def want(p, s, prev, n, ctx):
    if s == 0.0:
        return ctx["d0"]
    c = Matrix.Rotation(ctx["rot"] * (0.3 + s), 3, n) @ ctx["comb"]
    ct = c - n * c.dot(n)
    g = ctx["g0"] + ctx["g1"] * s
    w = ct.normalized() * (1 - g) + DOWN * g
    if s > 0.6:
        w += ctx["flick"] * ctx["fl"] * smooth(0.6, 1.0, s) * ctx["top"]
    return w.normalized()


def shell(p, s, ctx):
    return 0.0015


groots, gn = head.sample(GUIDES, hg.scalp_density, seed=11)
N = 18
guides, glab, gL = [], [], []
for r, n in zip(groots, gn):
    p = Vector(r)
    ctx, L = guide_ctx(p, Vector(n))
    L *= rng.uniform(0.85, 1.1)
    pts = hg.grow(head, p, n, want, L, min(0.004, L / 4), shell, stiff=ctx["stiff"], ctx=ctx)
    guides.append(hg.resample(pts, N))
    glab.append("A")
guides = hg.relax(np.array(guides), iters=4)
print("guides", len(guides), round(time.time() - t0, 1))
if os.environ.get("GUIDES_ONLY"):
    hg.make_curves("Guides", guides, r_root=2.5e-4, r_tip=1.5e-4, material=hg.hair_material(color=(0.05, 0.03, 0.02)))
    hg.setup_render(samples=16, res=(600, 660))
    hg.render_views(out + "/shortG", views=VIEWS)
    hg.save_grid([hg.load_px(f"{out}/shortG-{v}.png") for v in VIEWS], f"{out}/shortG-sheet.png", cols=len(VIEWS))
    raise SystemExit

def kid_density(az, el, p):
    # a soft hairline: the fade starts as sparse stubble, not an edge
    return hg.scalp_density(az, el, p, fade=14.0)


roots, cn = head.sample(KIDS, kid_density, seed=23)
clab = ["A"] * len(roots)
kids, near = hg.children(guides, groots, glab, roots, clab, k=3)
ae = np.array([hg.azel(Vector(r)) for r in roots])
top = np.clip((ae[:, 1] - 28.0) / 24.0, 0, 1)
# spiky pieces on top, the faded sides stay an even short pile
kids, keep = hg.leader_clump(kids, roots, clab, PIECES, rng, strength=(0.55, 0.9), power=1.3, fan=0.002 + 0.006 * top,
                             jag=(0.7, 1.0), cw_scale=0.25 + 0.75 * top)
kids = hg.trim(kids, keep)
# the fade thins out toward the ears and nape so skin shows through the stubble
dens = 0.12 + 0.88 * np.clip((ae[:, 1] + 30.0) / 70.0, 0, 1) ** 1.8
m = rng.random(len(roots)) < dens
kids, roots = kids[m], np.asarray(roots)[m]
amp = np.where(rng.random(len(roots)) < 0.006, 0.003, 0.0001)
kids = hg.frizz(kids, amp, rng, power=2.0, freq=(1, 3))
kids = hg.pushout(head, kids, frac=0.5, minoff=0.0010)
kids = hg.relax(kids, iters=2, k=0.4)
print("kids", kids.shape, round(time.time() - t0, 1))

mat = hg.hair_material(color=COLOR, coat=0.0, rough=float(os.environ.get("ROUGH", 0.34)), radial=0.35, grad=(0.55, 1.1), rnd=0.2)
hg.make_curves("Hair_short", kids, r_root=6.5e-5, r_tip=2.2e-5, material=mat)


def cap_density(az, el, p):
    # the fade shows skin through the stubble low on the sides and nape
    return hg.scalp_density(az, el, p) * (0.4 + 0.6 * smooth(-34, 20, el))


head.scalp_cap(cap_density, color=(0.045, 0.033, 0.026))
hg.setup_render(samples=int(os.environ.get("SPP", 64)))
hg.render_views(out + "/short", views=VIEWS)
male.sheet("short", out, views=[v for v in VIEWS if v in male.COLS])
if SAVE:
    import bpy
    bpy.ops.wm.save_as_mainfile(filepath=SAVE, compress=True)
print("done", round(time.time() - t0, 1))
