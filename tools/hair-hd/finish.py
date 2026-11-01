"""Alpha modes on the exported hair pack, like the old female cards: strips cut at 0.15, the crop's stubble
at 0.5, all double-sided.

usage: python -I finish.py output/hair-hd/raw/hair-<sex>.glb
"""
import json, struct, sys
from pathlib import Path

p = Path(sys.argv[1])
b = p.read_bytes()
size, typ = struct.unpack_from('<II', b, 12)
j = json.loads(b[20:20 + size]); tail = b[20 + size:]
for m in j['materials']:
    m['doubleSided'] = True
    if m['name'] == 'M_Hair_Cards':
        m['alphaMode'] = 'MASK'; m['alphaCutoff'] = 0.15
    elif m['name'] == 'M_Hair_Fade':
        m['alphaMode'] = 'MASK'; m['alphaCutoff'] = 0.5
    else:
        m.pop('alphaMode', None); m.pop('alphaCutoff', None)
data = json.dumps(j, separators=(',', ':')).encode(); data += b' ' * ((-len(data)) % 4)
p.write_bytes(struct.pack('<III', 0x46546c67, 2, 20 + len(data) + len(tail)) + struct.pack('<II', len(data), typ) + data + tail)
print('PATCHED', p, p.stat().st_size)
