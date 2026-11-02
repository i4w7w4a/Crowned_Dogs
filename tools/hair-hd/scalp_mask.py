"""Where each male head's scan carries its baked buzz cut: a mask the hair packs bring to the head shader.

usage: blender -b --factory-startup --python scalp_mask.py -- PERSONA.glb OUT.png
(writes OUT.png and OUT.json with each buzz's mean linear brightness; finish.py puts both into the male pack)
The Head_m1..m4 scans have a buzz cut painted into their colour maps (blond on m1, grey on m2 and m3,
faint on the bald m4). Under a pack hairstyle that buzz showed pale or grey at the hairline, at the temples
and between strips, a second hairline under the hair. look.rs paints it in the hair's colour instead,
keeping the scan's own stubble detail. One channel per face (R m1, G m2, B m3, A m4) in each head's UVs.
A texel counts as buzz by its colour, between the head's own forehead skin and crown buzz, and only on the
scalp: eyebrows, the face and the beard area are left out by position (rest space, metres, z up, face -y).
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
luma = []
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
    lum = (C ** 2.2) @ np.array([0.2126, 0.7152, 0.0722])
    luma.append(float(np.mean(lum[m > 0.6])) if (m > 0.6).any() else 0.1)
    print('SCALP', face, 'skin', (skin * 255).round(), 'buzz', (buzz * 255).round(), 'texels', int((m > 0.5).sum()), 'luma', round(luma[-1], 4))

# half size over covered texels only, then grow every island's edge outwards so filtering at a UV seam
# never pulls in the empty gap (that drew pale lines along the seams)
num = (mask * covs).reshape(OUT, 2, OUT, 2, 4).sum((1, 3))
den = covs.reshape(OUT, 2, OUT, 2, 4).sum((1, 3))
filled = den > 0
small = np.where(filled, num / np.maximum(den, 1e-6), 0)
for _ in range(6):
    acc = np.zeros_like(small); n = np.zeros_like(small)
    for sh in (-1, 1):
        for ax in (0, 1):
            acc += np.roll(small * filled, sh, ax); n += np.roll(filled, sh, ax)
    grow = ~filled & (n > 0)
    small = np.where(grow, acc / np.maximum(n, 1), small)
    filled = filled | grow
im = bpy.data.images.new('scalp', OUT, OUT, alpha=True)
im.colorspace_settings.name = 'Non-Color'
im.alpha_mode = 'STRAIGHT'
im.pixels.foreach_set(np.clip(small, 0, 1).astype(np.float32).ravel())
im.filepath_raw = out_p; im.file_format = 'PNG'; im.save()
Path(out_p).with_suffix('.json').write_text(json.dumps({'luma': [round(v, 4) for v in luma]}) + '\n')
print('LUMA', [round(v, 4) for v in luma])
print('SAVED', out_p)
