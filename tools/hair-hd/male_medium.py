"""m3 Medium: 10-15 cm, messy and a little wavy. A loose centre-left part, the fringe falling in pieces to
the brows, sides over the top of the ears, the back to the nape with the ends flicking out.
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
KIDS = int(args[2]) if len(args) > 2 else 65000
VIEWS = args[3].split(",") if len(args) > 3 else ["front", "left", "back"]
SAVE = args[4] if len(args) > 4 else None
rng = np.random.default_rng(4)
PIECES = int(os.environ.get("PIECES", 450))
COLOR = tuple(float(x) for x in os.environ.get("HAIRCOL", "0.03,0.019,0.012").split(","))
t0 = time.time()
head = hg.Head()
WHORL = Vector((C.x + 0.010, C.y + 0.075, C.z + 0.085))
PART_X = 0.012


def V(*a):
    return Vector(a)


def stand(ctx, n, comb, lift_deg):
    ct = (comb - n * comb.dot(n)).normalized()
    a = math.radians(max(2.0, lift_deg))
    ctx["d0"] = (ct * math.cos(a) + n * math.sin(a)).normalized()
    ctx["comb"] = comb.normalized()


def guide_ctx(p, n):
    az, el = hg.azel(p)
    ly = p.y - C.y
    top = smooth(15, 45, el)
    front = smooth(0.0, -0.07, ly)
    back = smooth(100, 155, abs(az))
    side = smooth(40, 80, abs(az)) * (1 - back)
    sx = 1.0 if p.x > PART_X else -1.0
    ctx = {"az": az, "el": el, "top": top, "lab": "A", "zstop": 1.585,
           "rot": 0.8 * noise.noise(p * 25.0) + rng.normal(0, 0.18),
           "flick": Vector(noise.noise_vector(p * 30.0)).normalized(), "fl": rng.uniform(0.2, 0.8),
           "out": rng.uniform(0.1, 1.0) * (0.4 * side + back) * (1 - 0.5 * top)}
    # out from the crown whorl; the front falls forward to the brows and opens from the part to either side
    radial = p - WHORL
    radial.z = 0
    crown = radial.normalized() if radial.length > 1e-5 else V(0, -1, 0)
    # curtains: swept out to the sides and back over the temples; a few loose pieces fall onto the forehead
    loose = rng.random() < 0.18
    fringe = V(sx * 0.5, -0.6, -0.9) if loose else V(sx * 0.7, 0.5 + 0.2 * noise.noise(p * 14.0), -0.5)
    comb = crown.lerp(fringe, 0.9 * front * top) + DOWN * (0.6 * (1 - top))
    # temples and sides go back and down over the top of the ear, never forward onto the cheek
    comb = comb.normalized().lerp(V(sx * 0.2, 0.75, -1.0).normalized(), smooth(30, 70, abs(az)) * (1 - back) * (1 - 0.5 * top))
    L = 0.105 + 0.03 * top + 0.004 * back - 0.012 * front - 0.02 * side * (1 - top)
    lift = 8 + 22 * top * (1 - (0.9 if loose else 0.2) * front) + rng.normal(0, 6)
    stand(ctx, n, comb, lift)
    ctx["stiff"] = 0.74
    ctx["g0"], ctx["g1"] = 0.08, 0.62
    ctx["sh"] = 0.005 + 0.016 * top * (1 - (0.9 if loose else 0.3) * front) + 0.008 * back
    return ctx, L


def want(p, s, prev, n, ctx):
    if s == 0.0:
        return ctx["d0"]
    c = Matrix.Rotation(ctx["rot"] * (0.3 + s), 3, n) @ ctx["comb"]
    ct = c - n * c.dot(n)
    if ct.length < 1e-4:
        ct = DOWN - n * DOWN.dot(n)
    g = ctx["g0"] + ctx["g1"] * s
    w = ct.normalized() * (1 - g) + DOWN * g
    # ends turn out at the sides and nape, loose pieces flick off anywhere
    w += n * ctx["out"] * smooth(0.65, 1.0, s)
    w += ctx["flick"] * ctx["fl"] * smooth(0.7, 1.0, s) * 0.35
    return w.normalized()


def shell(p, s, ctx):
    return 0.0015 + ctx["sh"] * smooth(0.0, 0.3, s)


groots, gn = head.sample(GUIDES, hg.scalp_density, seed=11)
N = 28
guides, glab = [], []
for r, n in zip(groots, gn):
    p = Vector(r)
    ctx, L = guide_ctx(p, Vector(n))
    L *= rng.uniform(0.8, 1.12)
    pts = hg.grow(head, p, n, want, L, 0.004, shell, stiff=ctx["stiff"], ctx=ctx)
    guides.append(hg.resample(pts, N))
    glab.append("A")
guides = hg.relax(np.array(guides), iters=4)
# a loose wave through each piece: the guides wave, the children follow
guides = hg.frizz(guides, np.full(len(guides), 0.006), rng, power=1.5, freq=(1, 2))
print("guides", len(guides), round(time.time() - t0, 1))
if os.environ.get("GUIDES_ONLY"):
    hg.make_curves("Guides", guides, r_root=2.5e-4, r_tip=1.5e-4, material=hg.hair_material(color=(0.05, 0.03, 0.02)))
    hg.setup_render(samples=16, res=(600, 660))
    hg.render_views(out + "/mediumG", views=VIEWS)
    hg.save_grid([hg.load_px(f"{out}/mediumG-{v}.png") for v in VIEWS], f"{out}/mediumG-sheet.png", cols=len(VIEWS))
    raise SystemExit

roots, cn = head.sample(KIDS, hg.scalp_density, seed=23)
clab = ["A"] * len(roots)
kids, near = hg.children(guides, groots, glab, roots, clab, k=3)
# messy layered pieces with pointed, uneven ends
kids, keep = hg.leader_clump(kids, roots, clab, PIECES, rng, strength=(0.7, 0.97), power=0.9, fan=0.010, jag=(0.65, 1.0))
kids = hg.trim(kids, keep)
# a loose wave through every piece, a few stray hairs
amp = np.where(rng.random(len(roots)) < 0.02, 0.009, 0.006)
kids = hg.frizz(kids, amp, rng, power=1.5, freq=(1, 3))
kids = hg.pushout(head, kids, frac=0.6, minoff=0.0012, body=True)
kids = hg.relax(kids, iters=2, k=0.4)
print("kids", kids.shape, round(time.time() - t0, 1))

mat = hg.hair_material(color=COLOR, coat=0.2, rough=float(os.environ.get("ROUGH", 0.24)), radial=0.35, grad=(0.5, 1.35), rnd=0.4)
hg.make_curves("Hair_medium", kids, r_root=7e-5, r_tip=2e-5, material=mat)
head.scalp_cap(hg.scalp_density, color=(0.045, 0.033, 0.026))
hg.setup_render(samples=int(os.environ.get("SPP", 64)))
hg.render_views(out + "/medium", views=VIEWS)
male.sheet("medium", out, views=[v for v in VIEWS if v in male.COLS])
if SAVE:
    import bpy
    bpy.ops.wm.save_as_mainfile(filepath=SAVE, compress=True)
print("done", round(time.time() - t0, 1))
