"""Alpha modes on the exported hair pack, like the old female cards: strips cut at 0.15, the crop's stubble
at 0.5, all double-sided. The male pack also gets the head masks from scalp_mask.py on its hidden quads: the
buzz mask (scalp-male.png and .json) on Hair_scalp and each hairline (hairline-<style>.png) on Hair_line_<style>,
as PNGs with no colour chunks, so their four channels arrive as stored.

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


def attach(material, mask):
    """The mask PNG as the material's colour map, added to the binary chunk; False if the pack has no such material."""
    global tail
    at = next((i for i, m in enumerate(j['materials']) if m['name'] == material), None)
    if at is None:
        return False
    png = bare_png(mask.read_bytes())
    bin_len, bin_type = struct.unpack_from('<II', tail, 0)
    blob = tail[8:8 + bin_len]
    blob += b'\0' * ((-len(blob)) % 4)
    j['bufferViews'].append({'buffer': 0, 'byteOffset': len(blob), 'byteLength': len(png)})
    blob += png + b'\0' * ((-len(png)) % 4)
    j['buffers'][0]['byteLength'] = len(blob)
    j.setdefault('images', []).append({'name': mask.stem, 'mimeType': 'image/png', 'bufferView': len(j['bufferViews']) - 1})
    # linear, no mipmaps: the masks are smooth, and mip levels would pull the empty atlas gaps into the seams
    j.setdefault('samplers', []).append({'magFilter': 9729, 'minFilter': 9729, 'wrapS': 33071, 'wrapT': 33071})
    j.setdefault('textures', []).append({'sampler': len(j['samplers']) - 1, 'source': len(j['images']) - 1})
    j['materials'][at].setdefault('pbrMetallicRoughness', {})['baseColorTexture'] = {'index': len(j['textures']) - 1}
    tail = struct.pack('<II', len(blob), bin_type) + blob
    print('MASK', material, mask.name, len(png), 'bytes')
    return True


here = Path(__file__).parent
if attach('M_Hair_Scalp', here / 'scalp-male.png'):
    tones = json.loads((here / 'scalp-male.json').read_text())
    # each face's buzz brightness, and its forehead skin and buzz colours (sRGB) for the hairlines' bare forehead
    extras = {'cdScalpLuma': tones['luma'], 'cdScalpSkin': tones['skin'], 'cdScalpBuzz': tones['buzz']}
    next(n for n in j['nodes'] if n.get('name') == 'Hair_scalp')['extras'] = extras
    print('TONES', extras)
for style in ('slick', 'ponytail'):
    attach(f'M_Hair_Line_{style}', here / f'hairline-{style}.png')

data = json.dumps(j, separators=(',', ':')).encode(); data += b' ' * ((-len(data)) % 4)
p.write_bytes(struct.pack('<III', 0x46546c67, 2, 20 + len(data) + len(tail)) + struct.pack('<II', len(data), typ) + data + tail)
print('PATCHED', p, p.stat().st_size)
