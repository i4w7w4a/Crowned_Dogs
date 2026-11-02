"""Copy the approved plates of the four closing maps from Art's folder into the game.

Run: python -P tools/kaskad-stage/import_closed.py C:/Users/qtas5/Documents/CrownedDogs/art/14-closed-maps/times
Then pack_times.py encodes them. Each map's picked frame (`*-pick.png`) is its hall plate at
the time it was drawn in; the other halls and every clean plate are relit edits of it.
All plates must be pixel registered to the pick: a plate that the model returned at another
size is fitted back with SIFT matches and an affine; where the fit leaves no source, the pick
fills in, colour-matched to the relit band beside the gap.
"""
import sys
from pathlib import Path

import cv2
import numpy as np

ROOT = Path(__file__).resolve().parents[2]
SRC = Path(sys.argv[1])
W, H = 941, 1672
# Folder name in the game, the time the pick was drawn in.
MAPS = {'intake': 'day', 'backwater': 'day', 'railyard': 'night', 'sludgeponds': 'evening'}
TIMES = ('morning', 'day', 'evening', 'night')
sift = cv2.SIFT_create(6000)


def load(path):
    image = cv2.imread(str(path), cv2.IMREAD_COLOR)
    if image is None:
        raise FileNotFoundError(path)
    return image


def gray(image):
    return cv2.createCLAHE(3.0, (8, 8)).apply(cv2.cvtColor(image, cv2.COLOR_BGR2GRAY))


def register(image, ref):
    """Affine fit of `image` (already W x H) onto `ref`; returns the image and the median error."""
    kr, dr = sift.detectAndCompute(gray(ref), None)
    k, d = sift.detectAndCompute(gray(image), None)
    good = [a for a, b in cv2.BFMatcher().knnMatch(d, dr, k=2) if a.distance < 0.75 * b.distance]
    p = np.float32([k[g.queryIdx].pt for g in good])
    q = np.float32([kr[g.trainIdx].pt for g in good])
    fit, inliers = cv2.estimateAffine2D(p, q, ransacReprojThreshold=3.0, maxIters=5000)
    inliers = inliers.ravel().astype(bool)
    error = float(np.median(np.linalg.norm(p[inliers] - q[inliers], axis=1)))
    if error <= 1.5:
        return image, error
    out = cv2.warpAffine(image, fit, (W, H), flags=cv2.INTER_LANCZOS4, borderMode=cv2.BORDER_CONSTANT)
    cover = cv2.warpAffine(np.full((H, W), 255, np.uint8), fit, (W, H), flags=cv2.INTER_NEAREST, borderValue=0)
    gap = cv2.erode(cover, np.ones((5, 5), np.uint8)) == 0
    if gap.any():
        near = (cv2.dilate(gap.astype(np.uint8), np.ones((61, 61), np.uint8)) > 0) & ~gap
        lo = cv2.cvtColor(out, cv2.COLOR_BGR2LAB).astype(float)
        lr = cv2.cvtColor(ref, cv2.COLOR_BGR2LAB).astype(float)
        fill = lr.copy()
        for c in range(3):
            fill[..., c] = (lr[..., c] - lr[..., c][near].mean()) * (lo[..., c][near].std() / (lr[..., c][near].std() + 1e-6)) + lo[..., c][near].mean()
        fill = cv2.cvtColor(np.clip(fill, 0, 255).astype(np.uint8), cv2.COLOR_LAB2BGR)
        alpha = np.where(gap, 0, cv2.GaussianBlur((~gap).astype(np.float32), (0, 0), 8))[..., None]
        out = (out * alpha + fill * (1 - alpha)).astype(np.uint8)
    return out, error


for name, own in MAPS.items():
    pick = load(SRC / f'{name}-pick.png')
    if pick.shape[:2] != (H, W):
        raise ValueError(f'{name}-pick must be {W}x{H}')
    for time in TIMES:
        folder = ROOT / 'apps/web/public/img' / (f'{name}-day' if time == 'day' else f'{name}-times/{time}')
        folder.mkdir(parents=True, exist_ok=True)
        for kind in ('hall', 'clean'):
            source = SRC / (f'{name}-pick.png' if kind == 'hall' and time == own else f'{name}-{time}-{kind}.png')
            image = load(source)
            size = image.shape[1], image.shape[0]
            if size != (W, H):
                image = cv2.resize(image, (W, H), interpolation=cv2.INTER_LANCZOS4)
            image, error = register(image, pick)
            cv2.imwrite(str(folder / f'{kind}.png'), image)
            print(f'{name} {time}/{kind}: from {source.name} {size[0]}x{size[1]}, fit error {error:.1f}px')
print('Imported the closing maps; run pack_times.py next.')
