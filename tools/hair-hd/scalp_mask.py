"""Where each male head's scan carries its baked buzz cut: a mask the hair packs bring to the head shader.

usage: blender -b --factory-startup --python scalp_mask.py -- PERSONA.glb OUT.png [STYLE=ROOTS.npz ...]
(writes OUT.png and OUT.json with each buzz's mean linear brightness and each head's own skin and buzz colour;
finish.py puts both into the male pack)
The Head_m1..m4 scans have a buzz cut painted into their colour maps (blond on m1, grey on m2 and m3,
faint on the bald m4). Under a pack hairstyle that buzz showed pale or grey at the hairline, at the temples
and between strips, a second hairline under the hair. look.rs paints it in the hair's colour instead,
keeping the scan's own stubble detail. One channel per face (R m1, G m2, B m3, A m4) in each head's UVs.
A texel counts as buzz by its colour, between the head's own forehead skin and crown buzz, and only on the
scalp: eyebrows, the face and the beard area are left out by position (rest space, metres, z up, face -y).

Each STYLE=ROOTS.npz (game_cards.py writes game_<style>_roots.npz for the slick and the ponytail) also gives
hairline-<style>.png beside OUT.png, at the full 1K, in the same channels: 0.5 plus half of where that
hairstyle's roots grow on each head, less half of the scan's buzz left bare on the forehead in front of them. The
head shader paints the first in the hair's colour, solid under the strips and thinning out past the last roots in
fine streaks along the comb, so the strips' ends no longer stand as steps on bare skin; and turns the second back
into the scan's own skin, so no patch of buzz shows ahead of the hairline.
"""
import bpy, json, sys
import numpy as np
from pathlib import Path

args = sys.argv[sys.argv.index('--') + 1:]
src, out_p = args[:2]
RES = 1024   # detection runs at the colour maps' 1K; the mask is saved at half that
OUT = 512

bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.ops.import_scene.gltf(filepath=src)
sc = bpy.context.scene
arm = next(o for o in sc.objects if o.type == 'ARMATURE')
arm.animation_data_clear(); arm.data.pose_position = 'REST'; bpy.context.view_layer.update()
dg = bpy.context.evaluated_depsgraph_get()


def smoothstep(a, b, x):
    t = np.clip((x - a) / (b - a), 0, 1)
    return t * t * (3 - 2 * t)


def base_pixels(mesh):
    """The colour map of the head's material, sRGB 0..1, rows from v=0 up, resampled to RES."""
    for n in mesh.materials[0].node_tree.nodes:
        if n.type == 'TEX_IMAGE' and n.image and any(l.to_socket.name == 'Base Color' for l in n.outputs[0].links):
            im = n.image
            if tuple(im.size) != (RES, RES):
                im.scale(RES, RES)
            im.colorspace_settings.name = 'Non-Color'   # the stored sRGB values, not linearised
            px = np.empty(RES * RES * 4, np.float32); im.pixels.foreach_get(px)
            return px.reshape(RES, RES, 4)[..., :3]
    raise RuntimeError('no colour map on ' + mesh.name)


def rasterize(mesh):
    """Rest position of every texel the head's UVs cover, NaN elsewhere."""
    mesh.calc_loop_triangles()
    V = np.array([v.co for v in mesh.vertices])
    uv = mesh.uv_layers.active.data
    P = np.full((RES, RES, 3), np.nan, np.float32)
    for tri in mesh.loop_triangles:
        t = np.array([uv[li].uv for li in tri.loops]) * RES - 0.5
        x0, y0 = np.floor(t.min(0)).astype(int); x1, y1 = np.ceil(t.max(0)).astype(int)
        x0, y0 = max(x0, 0), max(y0, 0); x1, y1 = min(x1, RES - 1), min(y1, RES - 1)
        if x1 < x0 or y1 < y0:
            continue
        gx, gy = np.meshgrid(np.arange(x0, x1 + 1), np.arange(y0, y1 + 1))
        a, b, c = t
        den = (b[1] - c[1]) * (a[0] - c[0]) + (c[0] - b[0]) * (a[1] - c[1])
        if abs(den) < 1e-12:
            continue
        l0 = ((b[1] - c[1]) * (gx - c[0]) + (c[0] - b[0]) * (gy - c[1])) / den
        l1 = ((c[1] - a[1]) * (gx - c[0]) + (a[0] - c[0]) * (gy - c[1])) / den
        l2 = 1 - l0 - l1
        inside = (l0 > -0.02) & (l1 > -0.02) & (l2 > -0.02)
        if not inside.any():
            continue
        w = np.stack([l0, l1, l2], -1)[inside]
        P[gy[inside], gx[inside]] = w @ V[list(tri.vertices)]
    return P


def blur(img, r):
    """Box blur, r texels each way, three passes (close to a gaussian)."""
    for _ in range(3):
        for ax in (0, 1):
            c = np.cumsum(np.pad(img, [(r + 1, r) if a == ax else (0, 0) for a in range(img.ndim)], mode='edge'), axis=ax)
            img = (np.take(c, range(2 * r + 1, c.shape[ax]), axis=ax) - np.take(c, range(0, c.shape[ax] - 2 * r - 1), axis=ax)) / (2 * r + 1)
    return img


def scalp_zone(P):
    """1 on the scalp, 0 on the face, the eyebrows and the beard area, soft in between."""
    x, y, z = np.abs(P[..., 0]), P[..., 1], P[..., 2]
    # sides and front come down to the top of the ear; the nape lower, behind the ears
    floor = 1.655 - 0.05 * smoothstep(0.0, 0.05, y)
    zone = smoothstep(floor - 0.01, floor + 0.008, z)
    # the forehead and the brows, in front of the temples
    face = (1 - smoothstep(-0.068, -0.056, y)) * (1 - smoothstep(1.728, 1.742, z)) * (1 - smoothstep(0.058, 0.07, x))
    return zone * (1 - face)


mask = np.zeros((RES, RES, 4), np.float32)
covs = np.zeros((RES, RES, 4), np.float32)
luma, tones = [], {'skin': [], 'buzz': []}
rest = []   # each face's texel positions, for the hairlines
for ch, face in enumerate(('Head_m1', 'Head_m2', 'Head_m3', 'Head_m4')):
    ob = bpy.data.objects[face]
    me = bpy.data.meshes.new_from_object(ob.evaluated_get(dg))
    me.transform(ob.matrix_world)
    C = base_pixels(ob.data)
    P = rasterize(me)
    cov = ~np.isnan(P[..., 0])
    Pz = np.where(cov[..., None], P, 0)
    x, y, z = np.abs(Pz[..., 0]), Pz[..., 1], Pz[..., 2]
    skin = np.median(C[cov & (x < 0.03) & (y < -0.07) & (z > 1.712) & (z < 1.728)], axis=0)
    buzz = np.median(C[cov & (z > 1.785)], axis=0)
    d = buzz - skin
    s = ((C - skin) @ d) / max(float(d @ d), 1e-6)
    # texel noise: the buzz is a speckle of dark hairs on skin, so it is judged over 3 mm or so
    s = blur(np.where(cov, s, 0), 2) / np.maximum(blur(cov.astype(np.float32), 2), 1e-3)
    m = smoothstep(0.12, 0.55, s) * scalp_zone(Pz) * cov
    m = blur(m, 1) / np.maximum(blur(cov.astype(np.float32), 1), 1e-3) * cov
    mask[..., ch] = m
    covs[..., ch] = cov
    rest.append(Pz)
    lum = (C ** 2.2) @ np.array([0.2126, 0.7152, 0.0722])
    luma.append(float(np.mean(lum[m > 0.6])) if (m > 0.6).any() else 0.1)
    tones['skin'].append([round(float(v), 4) for v in skin]); tones['buzz'].append([round(float(v), 4) for v in buzz])
    print('SCALP', face, 'skin', (skin * 255).round(), 'buzz', (buzz * 255).round(), 'texels', int((m > 0.5).sum()), 'luma', round(luma[-1], 4))


def grown(img, filled, steps=6):
    """Every island's edge grown outwards, so filtering at a UV seam never pulls in the empty gap (that drew
    pale lines along the seams)."""
    for _ in range(steps):
        acc = np.zeros_like(img); n = np.zeros_like(img)
        for sh in (-1, 1):
            for ax in (0, 1):
                acc += np.roll(img * filled, sh, ax); n += np.roll(filled, sh, ax)
        grow = ~filled & (n > 0)
        img = np.where(grow, acc / np.maximum(n, 1), img)
        filled = filled | grow
    return img


def save(img, path):
    im = bpy.data.images.new(Path(path).stem, img.shape[1], img.shape[0], alpha=True)
    im.colorspace_settings.name = 'Non-Color'
    im.alpha_mode = 'STRAIGHT'
    im.pixels.foreach_set(np.clip(img, 0, 1).astype(np.float32).ravel())
    im.filepath_raw = str(path); im.file_format = 'PNG'; im.save()
    print('SAVED', path)


# half size over covered texels only
num = (mask * covs).reshape(OUT, 2, OUT, 2, 4).sum((1, 3))
den = covs.reshape(OUT, 2, OUT, 2, 4).sum((1, 3))
save(grown(np.where(den > 0, num / np.maximum(den, 1e-6), 0), den > 0), out_p)
Path(out_p).with_suffix('.json').write_text(json.dumps({'luma': [round(v, 4) for v in luma], **tones}) + '\n')
print('LUMA', [round(v, 4) for v in luma])

# The hairlines are worked out on a grid of directions from the skull's centre (az round from the front, el up
# from level, GRID degrees a cell), so one set of roots serves every head: the per-face fits push the hair out
# from the skull, hardly along it.
GRID = 0.25
AZ = np.arange(-180, 180, GRID) + GRID / 2
EL = np.arange(-40, 90, GRID) + GRID / 2
R_SKULL = 0.095   # metres from the centre to the scalp, for millimetres in degrees


def angles(v):
    u = v / np.maximum(np.linalg.norm(v, axis=-1, keepdims=True), 1e-9)
    return np.degrees(np.arctan2(u[..., 0], -u[..., 1])), np.degrees(np.arcsin(np.clip(u[..., 2], -1, 1)))


def gauss(img, mm):
    """Gaussian blur of mm on the scalp: along el plainly, round az wider towards the crown (az wraps)."""
    s = np.degrees(mm / 1000 / R_SKULL) / GRID
    pad = int(4 * s) + 1
    n = img.shape[0] + 2 * pad
    f = np.fft.rfftfreq(n)
    out = np.fft.irfft(np.fft.rfft(np.pad(img, ((pad, pad), (0, 0))), axis=0) * np.exp(-2 * (np.pi * f * s) ** 2)[:, None], n, axis=0)[pad:-pad]
    sa = s / np.maximum(np.cos(np.radians(EL)), 0.15)
    f = np.fft.rfftfreq(img.shape[1])
    return np.fft.irfft(np.fft.rfft(out, axis=1) * np.exp(-2 * (np.pi * f[None] * sa[:, None]) ** 2), img.shape[1], axis=1)


def sample(img, az, el):
    """Bilinear, az wrapping."""
    y = np.clip((el - EL[0]) / GRID, 0, len(EL) - 1.001); x = (az - AZ[0]) / GRID
    y0 = np.floor(y).astype(int); x0 = np.floor(x).astype(int); fy = y - y0; fx = x - x0
    x0 %= len(AZ); x1 = (x0 + 1) % len(AZ); y1 = y0 + 1
    return img[y0, x0] * (1 - fx) * (1 - fy) + img[y0, x1] * fx * (1 - fy) + img[y1, x0] * (1 - fx) * fy + img[y1, x1] * fx * fy


def hairline(roots, steps, C):
    """How much of the head a hairstyle covers, per direction: 1 under its strips, thinning out in streaks
    along the comb as the roots thin out, a little past the last of them; and the roots' distance from the centre."""
    d = roots - C
    az, el = angles(d)
    cell = (np.clip(((el - EL[0]) / GRID).round().astype(int), 0, len(EL) - 1), ((az - AZ[0]) / GRID).round().astype(int) % len(AZ))
    count = np.zeros((len(EL), len(AZ))); np.add.at(count, cell, 1)
    dist = np.zeros_like(count); np.add.at(dist, cell, np.linalg.norm(d, axis=1))
    # the comb: each root's first step along the scalp, in el and az components
    u = d / np.linalg.norm(d, axis=1, keepdims=True)
    t = steps - u * np.einsum('nc,nc->n', steps, u)[:, None]
    t /= np.maximum(np.linalg.norm(t, axis=1, keepdims=True), 1e-9)
    a, e = np.radians(az), np.radians(el)
    e_az = np.stack([np.cos(a), np.sin(a), 0 * a], 1)
    e_el = np.stack([-np.sin(a) * np.sin(e), np.cos(a) * np.sin(e), np.cos(e)], 1)
    fe = np.zeros_like(count); np.add.at(fe, cell, np.einsum('nc,nc->n', t, e_el))
    fa = np.zeros_like(count); np.add.at(fa, cell, np.einsum('nc,nc->n', t, e_az))
    dens = gauss(count, 3.5)
    near = gauss(count, 4.0)
    radius = np.where(near > 1e-3, gauss(dist, 4.0) / np.maximum(near, 1e-3), 0)
    fe, fa = gauss(fe, 3.0), gauss(fa, 3.0)
    fl = np.maximum(np.hypot(fe, fa), 1e-9); fe, fa = fe / fl, fa / fl
    g = dens / np.median(sample(dens, az, el))   # 1: as dense as round the median root
    # fine streaks along the comb: noise smeared 2 mm each way along it (line integral convolution)
    noise = np.random.default_rng(11).random(count.shape)
    el_g, az_g = np.meshgrid(EL, AZ, indexing='ij')
    lic = np.zeros_like(noise); n = 9
    for k in range(-n, n + 1):
        s = np.degrees(k / n * 0.002 / R_SKULL)
        lic += sample(noise, az_g + s * fa / np.maximum(np.cos(np.radians(el_g)), 0.15), el_g + s * fe)
    lic = (lic - lic.mean()) / lic.std()
    cover = smoothstep(0, 1, (g - 0.04) / 0.45 + 0.35 * lic)
    print('HAIRLINE roots', len(roots), 'density at the roots 5/25/50 %', np.percentile(sample(g, az, el), [5, 25, 50]).round(2))
    return cover, radius


for arg in args[2:]:
    style, npz = arg.split('=', 1)
    with np.load(npz) as z:
        roots, steps, centre = z['roots'], z['dirs'], z['centre']
    cover, radius = hairline(roots, steps, centre)
    line = np.zeros((RES, RES, 4), np.float32)
    for ch, P in enumerate(rest):
        d = P - centre
        az, el = angles(d)
        # an ear stands out from the scalp: only texels near the roots' own distance from the centre take it
        off = np.linalg.norm(d, axis=-1) - sample(radius, az, el)
        hair = sample(cover, az, el) * (1 - smoothstep(0.007, 0.012, off))
        # the forehead in front of the hairline, between the temples: the buzz there goes back to skin
        front = (1 - smoothstep(0.035, 0.05, np.abs(d[..., 0]))) * (1 - smoothstep(-0.05, -0.03, d[..., 1]))
        bare = mask[..., ch] * (1 - hair) * front
        line[..., ch] = np.where(covs[..., ch] > 0, 0.5 + 0.5 * (hair - bare), 0.5)
        print('LINE', style, ch, 'hair texels', int((hair > 0.5).sum()), 'bare texels', int((bare > 0.5).sum()),
              'off under the hair, mm', np.percentile(off[hair > 0.5] * 1000, [5, 50, 95]).round(1))
    save(grown(line, covs > 0), Path(out_p).with_name(f'hairline-{style}.png'))
