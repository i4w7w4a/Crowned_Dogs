"""Sharper textures for the high and ultra graphics settings.

The figure models carry 1K textures inside (the medium setting; low shrinks them in the
browser). High and ultra load 2K and 4K copies of the same maps from public/tex/, made
here from the full-size Tripo maps of each material. A manifest per model tells the game
which material takes which files.

    python -I scripts/texture-tiers.py persona-male \
        M_Male_Body=<dir with color.jpg normal.jpg rough.jpg metal.jpg> M_Male_Head=<dir> ...

Each <dir> holds the material's Tripo maps, as in the .fbm folder of its zip, renamed to
color / normal / rough / metal, or with rm.png in place of rough and metal (a Tripo PBR
map already laid out the glTF way). Rough and metal go into one picture the glTF way (green
roughness, blue metalness), the same as the 1K copy inside the model.

NAME=@tex/<other>/NAME points a material at copies made for another model (the armour
pieces both Personas wear). Materials already in the model's manifest are kept.
"""

import json
import sys
from pathlib import Path

from PIL import Image

SIZES = (2048, 4096)
# normals show every compression step as bumps, so they keep more
QUALITY = {"color": 88, "normal": 94, "orm": 86}

def save(img: Image.Image, out: Path, kind: str) -> None:
    for s in SIZES:
        path = out.with_name(f"{out.name}.{kind}.{s}.webp")
        img.resize((s, s), Image.LANCZOS).save(path, "WEBP", quality=QUALITY[kind], method=6)
        print(f"  {path.name}  {path.stat().st_size / 1e6:.2f} MB")

def main() -> None:
    model, pairs = sys.argv[1], sys.argv[2:]
    web = Path(__file__).resolve().parent.parent
    out_dir = web / "public" / "tex" / model
    out_dir.mkdir(parents=True, exist_ok=True)
    path = web / "public" / "tex" / f"{model}.json"
    manifest = json.loads(path.read_text()) if path.exists() else {"sizes": list(SIZES), "materials": {}}
    for pair in pairs:
        name, src = pair.split("=", 1)
        if src.startswith("@"):
            manifest["materials"][name] = src[1:]
            continue
        src = Path(src)
        print(name)
        base = out_dir / name
        save(Image.open(src / "color.jpg").convert("RGB"), base, "color")
        save(Image.open(src / "normal.jpg").convert("RGB"), base, "normal")
        if (src / "rm.png").exists():
            rm = Image.open(src / "rm.png").convert("RGB")
            rough, metal = rm.getchannel("G"), rm.getchannel("B")
        else:
            rough = Image.open(src / "rough.jpg").convert("L")
            metal = Image.open(src / "metal.jpg").convert("L").resize(rough.size, Image.LANCZOS)
        white = Image.new("L", rough.size, 255)
        save(Image.merge("RGB", (white, rough, metal)), base, "orm")
        manifest["materials"][name] = f"tex/{model}/{name}"
    path.write_text(json.dumps(manifest, indent=1) + "\n")

if __name__ == "__main__":
    main()
