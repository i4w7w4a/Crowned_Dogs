"""Build a smooth inverse-depth data map; the colour photograph is never edited.

White = near, black = far. Deliberately coarse transitions keep a 6% photo move
stable. This is authored depth for a subtle effect, not a metric scene scan.
Only the Python standard library is required.
"""
import json
import math
import struct
import zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUTPUT = ROOT / 'apps/web/public/img/base-depth'
ASPECT = 1882 / 3344
WIDTH, HEIGHT = 512, round(512 / ASPECT)


def smooth(a, b, value):
    t = max(0., min(1., (value - a) / (b - a)))
    return t * t * (3 - 2 * t)


def ellipse(u, v, x, y, rx, ry):
    return math.exp(-2 * (((u - x) / rx) ** 2 + ((v - y) / ry) ** 2))


def polygon_weight(u, v, points, feather=.045):
    inside = False
    distance = 10.
    for (ax, ay), (bx, by) in zip(points, points[1:] + points[:1]):
        dx, dy = bx - ax, by - ay
        t = max(0., min(1., ((u - ax) * dx + (v - ay) * dy) / (dx * dx + dy * dy)))
        distance = min(distance, math.hypot(u - ax - dx * t, v - ay - dy * t))
        if (ay > v) != (by > v) and u < (bx - ax) * (v - ay) / (by - ay) + ax:
            inside = not inside
    return smooth(-feather, feather, distance if inside else -distance)


CHAIR = [(.065,.507),(.08,.438),(.111,.411),(.207,.415),(.267,.449),
         (.27,.508),(.371,.519),(.66,.627),(.699,.69),(.688,.742),
         (.464,.766),(.21,.661),(.153,.625),(.081,.55)]


def depth(u, v):
    # Broad architectural depth: distant tunnel, approaching floor and vault.
    floor = .10 + .78 * smooth(.37, 1., v)
    ceiling = .10 + .40 * smooth(.36, -.12, v)
    walls = .12 + .30 * smooth(.05, .72, abs(u - .28))
    value = max(floor, ceiling, walls)
    value *= 1 - .45 * ellipse(u, v, .275, .36, .13, .12)
    # Continuous cradles and shelving; no hard silhouettes or separate layers.
    for x, y, rx, ry, near in [(.76,.35,.13,.30,.50),(.60,.37,.09,.23,.38),
                              (.50,.38,.065,.18,.29),(.04,.38,.20,.28,.54)]:
        weight = ellipse(u, v, x, y, rx, ry)
        value = value * (1 - weight) + near * weight
    weight = polygon_weight(u, v, CHAIR)
    chair = .60 + .24 * smooth(.41, .77, v)
    value = value * (1 - weight) + chair * weight
    # The near console remains one soft volume, not an extracted cutout.
    weight = smooth(.75, .90, v)
    value = value * (1 - weight) + (.90 + .06 * smooth(.82, 1., v)) * weight
    return round(max(0., min(1., value)) * 255)


def chunk(kind, payload):
    return struct.pack('>I', len(payload)) + kind + payload + struct.pack('>I', zlib.crc32(kind + payload) & 0xffffffff)


OUTPUT.mkdir(parents=True, exist_ok=True)
rows = b''.join(b'\0' + bytes(depth((x + .5) / WIDTH, (y + .5) / HEIGHT)
                             for x in range(WIDTH)) for y in range(HEIGHT))
png = b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', WIDTH, HEIGHT, 8, 0, 0, 0, 0))
png += chunk(b'IDAT', zlib.compress(rows, 9)) + chunk(b'IEND', b'')
(OUTPUT / 'depth.png').write_bytes(png)
manifest = dict(version=1, kind='depth-photo', image='../base/salt-base.webp', depth='depth.png',
                aspect=ASPECT, zoom=1.06, parallax=.002,
                targets=dict(battle=[.235,.527], fitting=[.275,.36], persona=[.76,.33]))
(OUTPUT / 'manifest.json').write_text(json.dumps(manifest, indent=2) + '\n', encoding='utf-8')
print(f'Depth data: {WIDTH}x{HEIGHT}, {len(png)} bytes; original photograph unchanged')
