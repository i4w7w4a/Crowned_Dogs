"""Mouth and eye shapes for Sedoy's talking portrait: apps/web/public/img/base/instructor-face.webp.

GPT edited the briefing photograph four times (art/sedoy-talk in Art's folder, prompts in its
PROMPTS.md): mouth slightly open, open and rounded, and eyes shut. Each edit is aligned to the
original over the face (OpenCV ECC, affine), colour-matched to it around the cut, and cut out
with a soft oval. The four cells go into one 2 x 2 atlas with alpha; briefing_portrait.rs FACE
lists where they sit in the photograph and depth-backdrop.glsl lays them over it.

    python tools/base-depth/build_instructor_face.py <folder with talk-e/a/o.png and blink.png>

Needs numpy and opencv-python. The photograph itself is not edited.
"""
import sys
from pathlib import Path

import cv2
import numpy as np

ROOT = Path(__file__).resolve().parents[2]
PHOTO = ROOT / 'apps/web/public/img/base/instructor.webp'
OUTPUT = ROOT / 'apps/web/public/img/base/instructor-face.webp'
SIZE = 900
CELL = (256, 192)

# Must match briefing_portrait.rs FACE: u0, v0, u1, v1 of the photograph.
MOUTH = (.37, .34, .58, .49)
EYES = (.32, .22, .56, .32)
# The soft ovals cut from each edit: centre u, v and radii, in the photograph.
MOUTH_OVAL = (.462, .405, .072, .062)
EYE_OVALS = ((.38, .282, .055, .03), (.505, .259, .055, .03))
# Where the alignment looks: the head, without the part that changes.
HEAD = (.28, .08, .64, .52)


def oval(shape, centre, radii, feather=.35):
    h, w = shape
    y, x = np.mgrid[0:h, 0:w]
    d = np.sqrt(((x + .5) / w - centre[0]) ** 2 / radii[0] ** 2 + ((y + .5) / h - centre[1]) ** 2 / radii[1] ** 2)
    t = np.clip((1 + feather - d) / (2 * feather), 0, 1)
    return t * t * (3 - 2 * t)


def box(shape, rect):
    h, w = shape
    m = np.zeros(shape, np.float32)
    m[int(rect[1] * h):int(rect[3] * h), int(rect[0] * w):int(rect[2] * w)] = 1
    return m


def align(photo, edit, changed):
    """`edit` warped onto `photo`, matching the head outside the `changed` mask."""
    grey = lambda im: cv2.GaussianBlur(cv2.cvtColor(im, cv2.COLOR_BGR2GRAY).astype(np.float32) / 255, (0, 0), 1.2)
    mask = (box(photo.shape[:2], HEAD) * (1 - changed) > .5).astype(np.uint8)
    warp = np.eye(2, 3, dtype=np.float32)
    criteria = (cv2.TERM_CRITERIA_EPS | cv2.TERM_CRITERIA_COUNT, 300, 1e-7)
    score, warp = cv2.findTransformECC(grey(photo), grey(edit), warp, cv2.MOTION_AFFINE, criteria, mask, 5)
    aligned = cv2.warpAffine(edit, warp, (SIZE, SIZE), flags=cv2.INTER_CUBIC | cv2.WARP_INVERSE_MAP, borderMode=cv2.BORDER_REFLECT)
    return aligned, score, warp


def match(photo, aligned, ring):
    """Per-channel gain and offset so `aligned` has the photo's tone where `ring` is set."""
    out = aligned.astype(np.float32)
    w = ring / ring.sum()
    for c in range(3):
        a, b = out[..., c], photo[..., c].astype(np.float32)
        ma, mb = (a * w).sum(), (b * w).sum()
        sa = np.sqrt(((a - ma) ** 2 * w).sum()) + 1e-3
        sb = np.sqrt(((b - mb) ** 2 * w).sum())
        out[..., c] = (a - ma) * (sb / sa) + mb
    return np.clip(out, 0, 255)


def cell(image, alpha, rect):
    h, w = image.shape[:2]
    x0, y0, x1, y1 = int(rect[0] * w), int(rect[1] * h), int(rect[2] * w), int(rect[3] * h)
    rgba = np.dstack([image[y0:y1, x0:x1], alpha[y0:y1, x0:x1, None] * 255]).astype(np.float32)
    return cv2.resize(rgba, CELL, interpolation=cv2.INTER_AREA if CELL[0] < x1 - x0 else cv2.INTER_CUBIC)


def build(folder):
    photo = cv2.imread(str(PHOTO), cv2.IMREAD_COLOR)
    assert photo is not None and photo.shape[:2] == (SIZE, SIZE), PHOTO
    shape = photo.shape[:2]
    mouth = oval(shape, MOUTH_OVAL[:2], MOUTH_OVAL[2:])
    eyes = np.maximum(*(oval(shape, o[:2], o[2:]) for o in EYE_OVALS))
    atlas = np.zeros((CELL[1] * 2, CELL[0] * 2, 4), np.float32)
    checks = []
    for i, (name, alpha, rect) in enumerate([('talk-e', mouth, MOUTH), ('talk-a', mouth, MOUTH),
                                             ('talk-o', mouth, MOUTH), ('blink', eyes, EYES)]):
        edit = cv2.resize(cv2.imread(str(Path(folder) / f'{name}.png'), cv2.IMREAD_COLOR), (SIZE, SIZE), interpolation=cv2.INTER_AREA)
        changed = cv2.dilate((alpha > .01).astype(np.uint8), np.ones((25, 25), np.uint8)).astype(np.float32)
        aligned, score, warp = align(photo, edit, changed)
        ring = cv2.dilate((alpha > .01).astype(np.uint8), np.ones((31, 31), np.uint8)).astype(np.float32) * (alpha < .2)
        toned = match(photo, aligned, ring)
        x, y = i % 2, i // 2
        atlas[y * CELL[1]:(y + 1) * CELL[1], x * CELL[0]:(x + 1) * CELL[0]] = cell(toned, alpha, rect)
        mixed = photo * (1 - alpha[..., None]) + toned * alpha[..., None]
        checks.append((name, mixed.astype(np.uint8)))
        print(f'{name}: ECC {score:.4f}, scale {np.sqrt(abs(np.linalg.det(warp[:, :2]))):.4f}, shift {warp[0, 2]:+.1f} {warp[1, 2]:+.1f} px')
    ok = cv2.imwrite(str(OUTPUT), np.clip(atlas, 0, 255).astype(np.uint8), [cv2.IMWRITE_WEBP_QUALITY, 90])
    assert ok, OUTPUT
    print(f'{OUTPUT.name}: {OUTPUT.stat().st_size} bytes; original photograph unchanged')
    return photo, checks


if __name__ == '__main__':
    photo, checks = build(sys.argv[1])
    if len(sys.argv) > 2:
        # A strip of the face: the photo, then each shape laid over it as the game does.
        crop = lambda im: im[150:490, 230:590]
        cv2.imwrite(sys.argv[2], np.hstack([crop(photo)] + [crop(im) for _, im in checks]), [cv2.IMWRITE_JPEG_QUALITY, 88])
