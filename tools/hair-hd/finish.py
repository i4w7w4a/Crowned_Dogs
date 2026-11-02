"""Alpha modes on the exported hair pack, like the old female cards: strips cut at 0.15, the crop's stubble
at 0.5, all double-sided. The male pack also gets the scalp mask (scalp_mask.py: scalp-male.png and .json)
on its hidden Hair_scalp quad, as a PNG with no colour chunks, so its four channels arrive as stored.

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
# the per-face fits (game_cards.py) start off: look.rs turns on the chosen face's
for mesh in j['meshes']:
    if 'weights' in mesh:
        mesh['weights'] = [0] * len(mesh['weights'])


def bare_png(raw):
    """The PNG with only its pixel chunks: no gamma or colour profile for the browser to apply."""
    out, at = [raw[:8]], 8
    while at < len(raw):
        n, kind = struct.unpack_from('>I4s', raw, at)
        if kind in (b'IHDR', b'IDAT', b'IEND'):
            out.append(raw[at:at + 12 + n])
        at += 12 + n
    return b''.join(out)


scalp = next((i for i, m in enumerate(j['materials']) if m['name'] == 'M_Hair_Scalp'), None)
if scalp is not None:
    mask = Path(__file__).with_name('scalp-male.png')
    png = bare_png(mask.read_bytes())
    luma = json.loads(mask.with_suffix('.json').read_text())['luma']
    bin_len, bin_type = struct.unpack_from('<II', tail, 0)
    blob = tail[8:8 + bin_len]
    blob += b'\0' * ((-len(blob)) % 4)
    j['bufferViews'].append({'buffer': 0, 'byteOffset': len(blob), 'byteLength': len(png)})
    blob += png + b'\0' * ((-len(png)) % 4)
    j['buffers'][0]['byteLength'] = len(blob)
    j.setdefault('images', []).append({'name': 'scalp', 'mimeType': 'image/png', 'bufferView': len(j['bufferViews']) - 1})
    # linear, no mipmaps: the mask is smooth, and mip levels would pull the empty atlas gaps into the seams
    j.setdefault('samplers', []).append({'magFilter': 9729, 'minFilter': 9729, 'wrapS': 33071, 'wrapT': 33071})
    j.setdefault('textures', []).append({'sampler': len(j['samplers']) - 1, 'source': len(j['images']) - 1})
    j['materials'][scalp].setdefault('pbrMetallicRoughness', {})['baseColorTexture'] = {'index': len(j['textures']) - 1}
    node = next(n for n in j['nodes'] if n.get('name') == 'Hair_scalp')
    node['extras'] = {'cdScalpLuma': luma}
    tail = struct.pack('<II', len(blob), bin_type) + blob
    print('SCALP', mask, len(png), 'bytes, luma', luma)

data = json.dumps(j, separators=(',', ':')).encode(); data += b' ' * ((-len(data)) % 4)
p.write_bytes(struct.pack('<III', 0x46546c67, 2, 20 + len(data) + len(tail)) + struct.pack('<II', len(data), typ) + data + tail)
print('PATCHED', p, p.stat().st_size)
