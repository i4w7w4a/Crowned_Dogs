"""Depth map for Sedoy's video portrait (img/base/instructor.webp) in the briefing.

Same idea as build_depth.py: white = near, black = far, deliberately coarse. On a live quality
it tells the portrait where the man is, so only he breathes and the room behind him stays put.
The photograph itself is not edited. Only the Python standard library is required.
"""
import math
import struct
import zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUTPUT = ROOT / 'apps/web/public/img/base'

# Sedoy's outline: hair, the headset, the collar and both shoulders down to the frame's bottom.
MAN = [(.43,.035),(.55,.07),(.61,.17),(.65,.24),(.63,.33),(.67,.38),(.71,.45),(.79,.52),
       (.89,.57),(.97,.62),(1.02,.66),(1.02,1.02),(.07,1.02),(.09,.85),(.12,.75),(.17,.66),
       (.22,.58),(.27,.53),(.33,.47),(.32,.38),(.30,.30),(.295,.20),(.31,.10)]


def smooth(a, b, value):
    t = max(0., min(1., (value - a) / (b - a)))
    return t * t * (3 - 2 * t)


def ellipse(u, v, x, y, rx, ry):
    return math.exp(-2 * (((u - x) / rx) ** 2 + ((v - y) / ry) ** 2))


def polygon_weight(u, v, points, feather=.02):
    inside = False
    distance = 10.
    for (ax, ay), (bx, by) in zip(points, points[1:] + points[:1]):
        dx, dy = bx - ax, by - ay
        t = max(0., min(1., ((u - ax) * dx + (v - ay) * dy) / (dx * dx + dy * dy)))
        distance = min(distance, math.hypot(u - ax - dx * t, v - ay - dy * t))
        if (ay > v) != (by > v) and u < (bx - ax) * (v - ay) / (by - ay) + ax:
            inside = not inside
    return smooth(-feather, feather, distance if inside else -distance)


def portrait(u, v):
    # The control room recedes; the man stands in front, his face a little nearer than his coat.
    room = .12 + .18 * smooth(.6, 1., v)
    man = .78 + .1 * ellipse(u, v, .45, .3, .14, .2)
    weight = polygon_weight(u, v, MAN)
    return room * (1 - weight) + man * weight


def png(path, width, height, sample):
    rows = b''.join(b'\0' + bytes(round(max(0., min(1., sample((x + .5) / width, (y + .5) / height))) * 255)
                                 for x in range(width)) for y in range(height))

    def chunk(kind, payload):
        return struct.pack('>I', len(payload)) + kind + payload + struct.pack('>I', zlib.crc32(kind + payload) & 0xffffffff)

    data = b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', width, height, 8, 0, 0, 0, 0))
    data += chunk(b'IDAT', zlib.compress(rows, 9)) + chunk(b'IEND', b'')
    path.write_bytes(data)
    return len(data)


OUTPUT.mkdir(parents=True, exist_ok=True)
size = png(OUTPUT / 'instructor-depth.png', 256, 256, portrait)
print(f'instructor-depth.png: {size} bytes; original photograph unchanged')
