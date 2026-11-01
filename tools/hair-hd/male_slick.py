"""m2 Slicked back: 9-10 cm on top combed straight back from a lifted front, shorter sides combed back over
the ears, the back lying flat down to the nape. Run with PERSONA=male on 3d/hair-hd/base_male.blend."""
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
KIDS = int(args[2]) if len(args) > 2 else 60000
VIEWS = args[3].split(",") if len(args) > 3 else ["front", "left", "back"]
SAVE = args[4] if len(args) > 4 else None
rng = np.random.default_rng(3)
PIECES = int(os.environ.get("PIECES", 460))
COLOR = tuple(float(x) for x in os.environ.get("HAIRCOL", "0.055,0.042,0.033").split(","))
ZSTOP = float(os.environ.get("ZSTOP", 1.585))
t0 = time.time()
head = hg.Head()


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
    top = smooth(18, 42, el)
    front = smooth(0.0, -0.07, ly)
    back = smooth(100, 155, abs(az))
    sx = 1.0 if p.x > 0 else -1.0
    side = smooth(35, 75, abs(az)) * (1 - back)
    # the back ends in a soft V: lowest at the middle of the nape
    ctx = {"az": az, "el": el, "top": top, "lab": "A", "zstop": ZSTOP + 0.03 * smooth(0.0, 0.055, abs(p.x)),
           "rot": 0.25 * noise.noise(p * 30.0) + rng.normal(0, 0.06),
           "stick": 0.45, "free": 0.65}
    # straight back over the top, turning down over the occiput; sides back and a little down over the ears
    down = smooth(0.02, 0.09, ly)
    comb_top = V(0.06 * noise.noise(p * 9.0) + 0.08 * p.x / 0.08, 1.0, 0.0).lerp(DOWN, down)
    comb_side = V(sx * 0.05, 1.0, -0.45).lerp(DOWN, 0.7 * back)
    comb = comb_side.lerp(comb_top, top)
    # below the occiput the locks gather toward the middle of the nape
    comb = comb + V(-sx * 0.3 * smooth(5, -25, el) * back, 0, 0)
    L = 0.045 + 0.065 * top + 0.012 * front - 0.02 * back * (1 - top)
    lift = 6 + 48 * front * top + 8 * top + rng.normal(0, 4)
    stand(ctx, n, comb, lift)
    ctx["stiff"] = 0.82
    ctx["g0"], ctx["g1"] = 0.0, 0.12
    ctx["sh"] = (0.003 + 0.011 * top) * (1 - 0.55 * side) + 0.010 * front * top
    return ctx, L


def want(p, s, prev, n, ctx):
    if s == 0.0:
        return ctx["d0"]
    c = Matrix.Rotation(ctx["rot"] * (0.3 + s), 3, n) @ ctx["comb"]
    ct = c - n * c.dot(n)
    if ct.length < 1e-4:
        ct = DOWN - n * DOWN.dot(n)
    g = ctx["g0"] + ctx["g1"] * s
    return (ct.normalized() * (1 - g) + DOWN * g).normalized()


def shell(p, s, ctx):
    # the ends settle onto the layer below, so the top layer has no step where it ends
    return 0.0015 + ctx["sh"] * smooth(0.0, 0.35, s) * (1 - 0.65 * smooth(0.55, 1.0, s))


groots, gn = head.sample(GUIDES, hg.scalp_density, seed=11)
N = 24
guides, glab = [], []
for r, n in zip(groots, gn):
    p = Vector(r)
    ctx, L = guide_ctx(p, Vector(n))
    L *= rng.uniform(0.78, 1.12)
    pts = hg.grow(head, p, n, want, L, 0.004, shell, stiff=ctx["stiff"], ctx=ctx)
    guides.append(hg.resample(pts, N))
    glab.append("A")
guides = hg.relax(np.array(guides), iters=4)
print("guides", len(guides), round(time.time() - t0, 1))
if os.environ.get("GUIDES_ONLY"):
    hg.make_curves("Guides", guides, r_root=2.5e-4, r_tip=1.5e-4, material=hg.hair_material(color=(0.05, 0.03, 0.02)))
    hg.setup_render(samples=16, res=(600, 660))
    hg.render_views(out + "/slickG", views=VIEWS)
    hg.save_grid([hg.load_px(f"{out}/slickG-{v}.png") for v in VIEWS], f"{out}/slickG-sheet.png", cols=len(VIEWS))
    raise SystemExit

roots, cn = head.sample(KIDS, hg.scalp_density, seed=23)
clab = ["A"] * len(roots)
kids, near = hg.children(guides, groots, glab, roots, clab, k=3)
# combed, slightly wet pieces: long thin locks that stay together
kids, keep = hg.leader_clump(kids, roots, clab, PIECES, rng, strength=(0.5, 0.85), power=1.5, fan=0.005, jag=(0.72, 1.0))
kids = hg.trim(kids, keep)
amp = np.where(rng.random(len(roots)) < 0.02, 0.005, 0.0003)
kids = hg.frizz(kids, amp, rng, power=2.0, freq=(1, 3))
kids = hg.pushout(head, kids, frac=0.6, minoff=0.0012)
kids = hg.relax(kids, iters=2, k=0.4)
print("kids", kids.shape, round(time.time() - t0, 1))

mat = hg.hair_material(color=COLOR, coat=0.15, rough=float(os.environ.get("ROUGH", 0.28)), radial=0.35, grad=(0.55, 1.3), rnd=0.4)
hg.make_curves("Hair_slick", kids, r_root=6e-5, r_tip=2e-5, material=mat)
head.scalp_cap(hg.scalp_density, color=(0.045, 0.033, 0.026))
hg.setup_render(samples=int(os.environ.get("SPP", 64)))
hg.render_views(out + "/slick", views=VIEWS)
male.sheet("slick", out, views=[v for v in VIEWS if v in male.COLS])
if SAVE:
    import bpy
    bpy.ops.wm.save_as_mainfile(filepath=SAVE, compress=True)
print("done", round(time.time() - t0, 1))
