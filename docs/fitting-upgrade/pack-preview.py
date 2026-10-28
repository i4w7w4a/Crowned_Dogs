"""Pack existing GLB geometry unchanged; only encode texture images for the web viewer."""
import argparse, base64, hashlib, io, json, struct
from pathlib import Path
from PIL import Image

p = argparse.ArgumentParser()
p.add_argument('source', type=Path)
p.add_argument('output', type=Path)
p.add_argument('--texture-size', type=int, default=2048)
a = p.parse_args()
raw = a.source.read_bytes()
length = struct.unpack_from('<I', raw, 12)[0]
doc = json.loads(raw[20:20 + length])
binary = raw[28 + length:]
images = {i['bufferView']: i for i in doc.get('images', [])}
blocks, offset, geometry = [], 0, hashlib.sha256()
for index, view in enumerate(doc['bufferViews']):
    start = view.get('byteOffset', 0)
    data = binary[start:start + view['byteLength']]
    if index in images:
        im = Image.open(io.BytesIO(data))
        im.thumbnail((a.texture_size, a.texture_size), Image.Resampling.LANCZOS)
        encoded = io.BytesIO()
        im.save(encoded, format='WEBP', quality=95, method=6)
        data = encoded.getvalue()
        images[index]['mimeType'] = 'image/webp'
    else:
        geometry.update(data)
    view['byteOffset'], view['byteLength'] = offset, len(data)
    block = data + b'\0' * (-len(data) % 4)
    blocks.append(block)
    offset += len(block)
for texture in doc.get('textures', []):
    source = texture.pop('source', None)
    if source is not None:
        texture.setdefault('extensions', {})['EXT_texture_webp'] = {'source': source}
for field in ['extensionsUsed', 'extensionsRequired']:
    if 'EXT_texture_webp' not in doc.setdefault(field, []): doc[field].append('EXT_texture_webp')
doc['buffers'] = [{'byteLength': offset}]
text = json.dumps(doc, separators=(',', ':')).encode()
text += b' ' * (-len(text) % 4)
binout = b''.join(blocks)
packed = struct.pack('<III', 0x46546c67, 2, 28 + len(text) + len(binout))
packed += struct.pack('<II', len(text), 0x4e4f534a) + text
packed += struct.pack('<II', len(binout), 0x004e4942) + binout
# Every non-image buffer view is copied byte-for-byte: no fitting, simplification or reposing.
check = hashlib.sha256()
for index, view in enumerate(doc['bufferViews']):
    if index not in images:
        check.update(binout[view['byteOffset']:view['byteOffset'] + view['byteLength']])
assert check.digest() == geometry.digest()
a.output.parent.mkdir(parents=True, exist_ok=True)
a.output.write_text(json.dumps({'data': base64.b64encode(packed).decode()}, separators=(',', ':')))
print(json.dumps({'source': a.source.name, 'sourceSha256': hashlib.sha256(raw).hexdigest(),
    'geometrySha256': geometry.hexdigest(), 'bytes': len(packed), 'output': a.output.name}))
