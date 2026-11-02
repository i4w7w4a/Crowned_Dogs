"""Encode the approved «Каскад» plates (day and three times per map), without resampling or grading.

Run: target/asset-env/Scripts/python.exe -I tools/kaskad-stage/pack_times.py [map ...]
With no names it re-encodes every map; name only the maps whose plates changed.
Generation prompts/provenance: docs/kaskad-art.md.
"""
import hashlib
import json
import sys
from pathlib import Path

from PIL import Image, ImageStat

ROOT = Path(__file__).resolve().parents[2]
IMG = ROOT / 'apps/web/public/img'
for name in sys.argv[1:] or ('dam', 'drybed', 'intake', 'backwater', 'railyard', 'sludgeponds'):
    for folder, times in ((IMG / f'{name}-day', ('',)), (IMG / f'{name}-times', ('morning', 'evening', 'night'))):
        records = []
        for time in times:
            for kind in ('hall', 'clean'):
                source = folder / time / f'{kind}.png'
                image = Image.open(source).convert('RGB')
                if image.size != (941, 1672):
                    raise ValueError(f'Incorrect calibrated size: {source}: {image.size}')
                encoded = source.with_suffix('.webp')
                image.save(encoded, 'WEBP', quality=94, method=6)
                for file in (source, encoded):
                    records.append({'path': file.relative_to(ROOT).as_posix(),
                        'width': image.width, 'height': image.height,
                        'bytes': file.stat().st_size,
                        'sha256': hashlib.sha256(file.read_bytes()).hexdigest()})
                print(f'{name} {time or "day"}/{kind}: mean L={ImageStat.Stat(image.convert("L")).mean[0]:.2f}')
        (folder / 'assets.json').write_text(json.dumps({'version': 1,
            'source': f'Codex image_gen: approved {name} plate, its clean edit and lighting-weather edits',
            'nativeSize': [941, 1672], 'files': records}, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
print('Encoded and hashed the Kaskad plates; no resizing or grading.')
