"""Stubble texture for the crop's faded sides: tools/hair-hd/hair-fade.png.

usage: python fade_texture.py tools/hair-hd/hair-fade.png
Short dark dashes, sparse at the bottom of the image (the ear and nape, where skin shows) and
packed at the top, where the fade meets the strips on top. The strip texture's mean colour, so the
same hair colour tint fits both. Tiles left to right; top to bottom it spans the whole fade.
"""
import sys
import numpy as np
from PIL import Image, ImageDraw

W, H = 512, 1024
rng = np.random.default_rng(5)
img = Image.new("RGBA", (W * 3, H), (0, 0, 0, 0))
d = ImageDraw.Draw(img)
base = np.array([104, 78, 58])   # ~ HAIR_BASE in look.rs: the strips' mean sRGB
for y in range(H):
    v = 1 - y / H                      # 0 at the bottom edge of the fade, 1 at the top
    if v > 0.75:                       # the top runs under the strips: a solid dark pile
        a = int(255 * min(1.0, (v - 0.75) / 0.12))
        d.line([(0, y), (W * 3, y)], fill=(*(base * 0.7).astype(int), a))
for y in range(-40, H):
    v = 1 - max(y, 0) / H
    cover = 0.06 + 0.94 * v ** 1.6
    # dashes starting on this row: enough that their area gives the cover
    n = rng.poisson(cover * W / 18.0 * 0.7)
    for _ in range(n):
        x = rng.uniform(W, 2 * W)
        ln = rng.uniform(14, 34) * (0.6 + 0.4 * v)
        ang = np.radians(rng.normal(0, 12))
        c = (base * rng.uniform(0.45, 1.1)).clip(0, 255).astype(int)
        x2, y2 = x + np.sin(ang) * ln, y + np.cos(ang) * ln
        wd = int(rng.integers(2, 4))
        for dx in (-W, 0, W):         # wrap left-right so the band tiles
            d.line([(x + dx, y), (x2 + dx, y2)], fill=(*c, 255), width=wd)
out = img.crop((W, 0, 2 * W, H))
out.save(sys.argv[1])
print("SAVED", sys.argv[1], out.size)
