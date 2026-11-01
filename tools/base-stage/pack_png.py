"""Recompress PNG scanlines losslessly; never change samples, filters or metadata."""
import struct
import zlib
from pathlib import Path


def pack_png(path):
    path = Path(path)
    original = path.read_bytes()
    assert original[:8] == b'\x89PNG\r\n\x1a\n'
    chunks = []
    offset = 8
    while offset < len(original):
        size = struct.unpack('>I', original[offset:offset + 4])[0]
        kind = original[offset + 4:offset + 8]
        data = original[offset + 8:offset + 8 + size]
        crc = struct.unpack('>I', original[offset + 8 + size:offset + 12 + size])[0]
        assert zlib.crc32(kind + data) & 0xffffffff == crc
        chunks.append((kind, data))
        offset += size + 12
    scanlines = zlib.decompress(b''.join(data for kind, data in chunks if kind == b'IDAT'))
    compressed = zlib.compress(scanlines, 9)
    assert zlib.decompress(compressed) == scanlines
    output = bytearray(original[:8])
    written = False
    for kind, data in chunks:
        if kind == b'IDAT':
            if written:
                continue
            data = compressed
            written = True
        output += struct.pack('>I', len(data)) + kind + data + struct.pack('>I', zlib.crc32(kind + data) & 0xffffffff)
    if len(output) < len(original):
        path.write_bytes(output)
    return len(original), min(len(output), len(original))


if __name__ == '__main__':
    import sys
    print(pack_png(sys.argv[1]))
