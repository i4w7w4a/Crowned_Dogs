"""Male groom helpers: the reference sheet (research/hair-ref/hair-male-ref.webp, Art 2026-11-01) and a
reference | render sheet. Run the male styles with PERSONA=male on 3d/hair-hd/base_male.blend."""
import hairgen as hg

# a man's hairline sits lower on the forehead than hers; the scan heads are bald or buzzed
hg.HAIRLINE = [(0, 30), (20, 29), (35, 26), (50, 20), (62, 10), (70, 0), (75, -16), (84, -16), (87, 0), (100, -1),
               (112, -3), (118, -22), (135, -36), (150, -40), (180, -42)]

REF = r"C:\Users\qtas5\Documents\CrownedDogs\research\hair-ref\hair-male-ref.webp"
# rows m1..m4 and columns front / side / back of the reference (pixels, top-left origin)
ROWS = {"short": (48, 278), "slick": (282, 508), "medium": (512, 742), "dreads": (746, 1010)}
COLS = {"front": (190, 610), "left": (612, 1065), "back": (1067, 1515)}


def sheet(style, out, views=("front", "left", "back")):
    """Top row: the reference front | side | back; bottom row: the renders."""
    y0, y1 = ROWS[style]
    imgs = [hg.fit_h(hg.load_px(REF, (COLS[v][0], y0, COLS[v][1], y1)), 520) for v in views]
    imgs += [hg.fit_h(hg.load_px(f"{out}/{style}-{v}.png"), 520) for v in views]
    hg.save_grid(imgs, f"{out}/{style}-sheet.png", cols=len(views))
