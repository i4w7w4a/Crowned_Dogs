"""Depth maps for the «Шлюз» base (region Каскад): the home photograph and the equipment room.

Same idea as build_depth.py: white = near, black = far, deliberately coarse so the 6% home
move and the equipment room's tilt stay stable. The photographs themselves are not edited.
Only the Python standard library is required.
"""
import json
import math
import struct
import zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUTPUT = ROOT / 'apps/web/public/img/base-kaskad'
ASPECT = 941 / 1672


def smooth(a, b, value):
    t = max(0., min(1., (value - a) / (b - a)))
    return t * t * (3 - 2 * t)


def ellipse(u, v, x, y, rx, ry):
    return math.exp(-2 * (((u - x) / rx) ** 2 + ((v - y) / ry) ** 2))


def polygon_weight(u, v, points, feather=.04):
    inside = False
    distance = 10.
    for (ax, ay), (bx, by) in zip(points, points[1:] + points[:1]):
        dx, dy = bx - ax, by - ay
        t = max(0., min(1., ((u - ax) * dx + (v - ay) * dy) / (dx * dx + dy * dy)))
        distance = min(distance, math.hypot(u - ax - dx * t, v - ay - dy * t))
        if (ay > v) != (by > v) and u < (bx - ax) * (v - ay) / (by - ay) + ax:
            inside = not inside
    return smooth(-feather, feather, distance if inside else -distance)


def blend(value, near, weight):
    return value * (1 - weight) + near * weight


# The comms table with the tablets and radios, lower left of the home photograph.
TABLE = [(0., .53), (.2, .53), (.42, .58), (.6, .66), (.63, .76), (.55, .83), (0., .84)]


def home(u, v):
    # The observation room: the slit and its view far away, the floor coming close.
    floor = .10 + .80 * smooth(.45, 1., v)
    ceiling = .10 + .40 * smooth(.24, -.05, v)
    left = .14 + .34 * smooth(.28, 0., u) * smooth(.62, .5, v)
    value = max(floor, ceiling, left)
    value *= 1 - .6 * ellipse(u, v, .54, .31, .2, .06)
    # The stairwell behind the open door goes back into the dark.
    value = blend(value, .12, ellipse(u, v, .13, .38, .05, .08))
    for x, y, rx, ry, near in [(.86, .38, .06, .17, .52), (.53, .44, .1, .09, .4),
                              (.37, .43, .07, .05, .3), (.94, .3, .07, .3, .5)]:
        value = blend(value, near, ellipse(u, v, x, y, rx, ry))
    value = blend(value, .78 + .1 * smooth(.55, .84, v), polygon_weight(u, v, TABLE))
    # The near cases at the bottom stay one soft volume.
    value = blend(value, .93, smooth(.8, .9, v) * smooth(.8, .7, u))
    return value


def fitting(u, v):
    # The gate hoist gallery as a box seen down its length: the farther from the vanishing
    # point, the nearer the floor, the ceiling or a wall. The hoist drums on the right stand
    # closer than the window wall on the left.
    dx = (u - .47) * ASPECT
    dy = v - .44
    # A soft maximum keeps the room's corners from showing as creases.
    planes = (dy / 1.6, -dy / 2.0, -dx / 2.2, dx / 1.8)
    near = sum(max(0., p) ** 5 for p in planes) ** .2 * (1.6 / .56)
    return .03 + .97 * min(1., near) ** .9


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
size = png(OUTPUT / 'home-depth.png', 512, round(512 / ASPECT), home)
print(f'home-depth.png: {size} bytes')
size = png(OUTPUT / 'fitting-depth.png', 470, 836, fitting)
print(f'fitting-depth.png: {size} bytes')
manifest = dict(version=1, kind='depth-photo', image='home.webp', depth='home-depth.png',
                aspect=ASPECT, zoom=1.06, parallax=.002,
                targets=dict(battle=[.53, .43], fitting=[.15, .37], persona=[.85, .36]))
(OUTPUT / 'home.json').write_text(json.dumps(manifest, indent=2) + '\n', encoding='utf-8')
