"""Encode the approved time-of-day plates, without resampling or grading.

Run: target/asset-env/Scripts/python.exe -I tools/blocks-stage/pack_times.py
Generation prompts/provenance: docs/blocks-times-art.md.
"""
import hashlib
import json
from pathlib import Path

from PIL import Image, ImageStat

ROOT = Path(__file__).resolve().parents[2]
ASSETS = ROOT / 'apps/web/public/img/blocks-times'
records = []
for time in ('morning', 'evening', 'night'):
    for kind in ('hall', 'clean'):
        source = ASSETS / time / f'{kind}.png'
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
        print(f'{time}/{kind}: {image.width}x{image.height}, mean L={ImageStat.Stat(image.convert("L")).mean[0]:.2f}')
(ASSETS / 'assets.json').write_text(json.dumps({'version': 1,
    'source': 'Built-in image_gen: lighting-weather edits of approved blocks-day full/clean plates',
    'nativeSize': [941, 1672], 'files': records}, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
print(f'Encoded and hashed {len(records)} plates; no resizing or grading.')
