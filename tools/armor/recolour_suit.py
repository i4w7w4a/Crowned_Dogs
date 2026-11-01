"""Recolours of the field suit (Солончак, Тень) from its colour map.

usage: python -I recolour_suit.py SUIT_COLOR.webp OUT_PREFIX [OUT_PREFIX ...]

Writes OUT_PREFIX/M_<Name>.color.<size>.webp for the sizes of the input name (or 1024 from a GLB-sized map).
The camo pattern survives as light and dark; the hue is replaced by each palette's two ends.
"""
import sys
from pathlib import Path
from PIL import Image, ImageOps

PALETTES = {
    # salt flat: pale grey with a little sand in the darks
    'Solonchak': ((92, 88, 78), (222, 218, 208), 0.85, 0.12),
    # shadow: matte charcoal with a faint cool cast
    'Ten': ((14, 15, 18), (70, 74, 82), 1.10, 0.0),
}

def recolour(src: Image.Image, dark, light, gamma: float, lift: float) -> Image.Image:
    lum = ImageOps.autocontrast(src.convert('L'), cutoff=1)
    lum = lum.point(lambda v: int(255 * min(1.0, lift + (1 - lift) * (v / 255) ** gamma)))
    chans = [lum.point(lambda v, a=a, b=b: int(a + (b - a) * v / 255)) for a, b in zip(dark, light)]
    return Image.merge('RGB', chans)

def main() -> None:
    src_p, outs = Path(sys.argv[1]), sys.argv[2:]
    src = Image.open(src_p).convert('RGB')
    for name, (dark, light, gamma, lift) in PALETTES.items():
        img = recolour(src, dark, light, gamma, lift)
        for out in outs:
            out = Path(out)
            sizes = (2048, 4096) if 'tex' in out.parts else (1024,)
            for s in sizes:
                p = out / f'M_{name}.color.{s}.webp'
                img.resize((s, s), Image.LANCZOS).save(p, 'WEBP', quality=88, method=6)
                print(p, round(p.stat().st_size / 1e6, 2), 'MB')

if __name__ == '__main__':
    main()
