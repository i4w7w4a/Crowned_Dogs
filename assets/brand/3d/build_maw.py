"""Crowned Dogs maw: two crowns as jaws, built procedurally in Blender (bpy) after the one-colour logo.
usage: python build_maw.py out.glb [out.blend]   (needs Blender as a Python module: pip install bpy)

Layout (Blender, Z up, front = -Y; glTF export turns it into Y up, front = +Z):
  CrownedDogsMaw (root empty)
    JawUpper (empty on the back hinge) -> CrownUpper mesh (crown upside down, points = upper fangs)
    JawLower (empty on the same hinge) -> CrownLower mesh (crown upright)
Animation clips (NLA tracks, one glTF clip each): Bite (loading loop), Idle, Open.
"""
import bpy, bmesh, math, sys
from mathutils import Vector

argv = sys.argv[sys.argv.index('--') + 1:] if '--' in sys.argv else sys.argv[1:]
OUT = argv[0]
BLEND = argv[1] if len(argv) > 1 else None

bpy.ops.wm.read_factory_settings(use_empty=True)
scene = bpy.context.scene
FPS = 30
scene.render.fps = FPS

# ---------------------------------------------------------------- materials
def material(name, rgb, rough, metal=0.0, coat=0.0, coat_rough=0.2):
    m = bpy.data.materials.new(name)
    m.use_nodes = True
    p = m.node_tree.nodes['Principled BSDF']
    p.inputs['Base Color'].default_value = (*rgb, 1)
    p.inputs['Roughness'].default_value = rough
    p.inputs['Metallic'].default_value = metal
    if coat:
        p.inputs['Coat Weight'].default_value = coat
        p.inputs['Coat Roughness'].default_value = coat_rough
    return m


def srgb(h):
    v = [int(h[i:i + 2], 16) / 255 for i in (1, 3, 5)]
    return tuple(c / 12.92 if c <= 0.04045 else ((c + 0.055) / 1.055) ** 2.4 for c in v)


BONE = material('Bone', srgb('#D9D0BB'), 0.40, 0.0, coat=0.35, coat_rough=0.22)
INNER = material('BoneInner', srgb('#2B2723'), 0.75, 0.0)   # inside of the rings = the dark of the mouth
for m in (BONE, INNER):
    m.use_backface_culling = True

# ---------------------------------------------------------------- mesh helpers
def frame(theta):
    u = Vector((math.sin(theta), -math.cos(theta), 0))   # radial, theta=0 is the front (-Y)
    t = Vector((math.cos(theta), math.sin(theta), 0))    # tangent
    return u, t


def P(theta, r, tang, z):
    u, t = frame(theta)
    return u * r + t * tang + Vector((0, 0, z))


def lathe(bm, profile, segs):
    rings = []
    for i in range(segs):
        a = 2 * math.pi * i / segs
        rings.append([bm.verts.new(P(a, r, 0, z)) for r, z in profile])
    n = len(profile)
    for i in range(segs):
        A, B = rings[i], rings[(i + 1) % segs]
        for j in range(n):
            k = (j + 1) % n
            bm.faces.new((A[j], A[k], B[k], B[j]))


def spike(bm, theta, k, R, z0, h, w, eo, ei, lean, drop, S=6, belly=0.8):
    """Faceted crown point: a long diamond blade above the band plus a short diamond plate down on the band."""
    rings = []
    for i in range(S):
        s = i / S
        taper = (1 - s) ** belly
        rc = R - lean * s * s
        z = z0 + h * s
        rings.append([bm.verts.new(P(theta, k * rc, k * -w * taper, k * z)),
                      bm.verts.new(P(theta, k * (rc + eo * taper), 0, k * z)),
                      bm.verts.new(P(theta, k * rc, k * w * taper, k * z)),
                      bm.verts.new(P(theta, k * (rc - ei * taper), 0, k * z))])
    tip = bm.verts.new(P(theta, k * (R - lean), 0, k * (z0 + h)))
    for a, b in zip(rings, rings[1:]):
        for j in range(4):
            bm.faces.new((a[j], a[(j + 1) % 4], b[(j + 1) % 4], b[j]))
    last = rings[-1]
    for j in range(4):
        bm.faces.new((last[j], last[(j + 1) % 4], tip))
    low = bm.verts.new(P(theta, k * (R - 0.005), 0, k * (z0 - drop)))
    first = rings[0]
    for j in range(4):
        bm.faces.new((first[(j + 1) % 4], first[j], low))


def canine(bm, theta, k, R, z0, h, w, curl, fwd, side, S=14, n=8):
    """Curved sabre fang: a tapering lens section swept along a quadratic curve that bends into the mouth."""
    p0 = Vector((R + 0.01, 0, z0 - 0.06))
    p1 = Vector((R + 0.10, 0, z0 + h * 0.62))
    p2 = Vector((R - curl, fwd * side, z0 + h))

    def at(s):
        return (1 - s) ** 2 * p0 + 2 * (1 - s) * s * p1 + s * s * p2

    def d_at(s):
        return (2 * (1 - s) * (p1 - p0) + 2 * s * (p2 - p1)).normalized()

    def world(v):
        return P(theta, k * v.x, k * v.y, k * v.z)

    rings = []
    for i in range(S):
        s = i / S
        c, T = at(s), d_at(s)
        side_v = Vector((0, 1, 0))
        side_v = (side_v - T * side_v.dot(T)).normalized()
        nrm = T.cross(side_v).normalized()
        taper = (1 - s) ** 0.85
        ring = []
        for j in range(n):
            a = 2 * math.pi * j / n
            # lens: wide sideways, thinner front-back, sharp ridge at front and back
            sx = math.cos(a) * w * taper
            sy = math.sin(a) * w * 0.62 * taper * (1 - 0.35 * abs(math.cos(a)))
            ring.append(bm.verts.new(world(c + side_v * sx + nrm * sy)))
        rings.append(ring)
    tip = bm.verts.new(world(p2))
    for a, b in zip(rings, rings[1:]):
        for j in range(n):
            bm.faces.new((a[j], a[(j + 1) % n], b[(j + 1) % n], b[j]))
    for j in range(n):
        bm.faces.new((rings[-1][j], rings[-1][(j + 1) % n], tip))
    bm.faces.new(list(reversed(rings[0])))


def gem(bm, theta, k, R, zc, hh, hw, out):
    """Diamond stud on the band: a rhombus pyramid (four facets, like the light star in the logo's studs)."""
    def rh(rr, sc):
        return [bm.verts.new(P(theta, k * rr, 0, k * (zc + hh * sc))), bm.verts.new(P(theta, k * rr, k * hw * sc, k * zc)),
                bm.verts.new(P(theta, k * rr, 0, k * (zc - hh * sc))), bm.verts.new(P(theta, k * rr, -k * hw * sc, k * zc))]
    base, top = rh(R - 0.004, 1.32), rh(R + 0.012, 1.32)          # setting plate
    for j in range(4):
        bm.faces.new((base[j], base[(j + 1) % 4], top[(j + 1) % 4], top[j]))
    bm.faces.new(list(reversed(base)))
    inner = rh(R + 0.012, 1.0)
    for j in range(4):
        bm.faces.new((top[j], top[(j + 1) % 4], inner[(j + 1) % 4], inner[j]))
    a = bm.verts.new(P(theta, k * (R + 0.012 + out), 0, k * zc))
    for j in range(4):
        bm.faces.new((inner[j], inner[(j + 1) % 4], a))


def finish(name, bm, smooth, sharp_deg, inner=False):
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    if inner:
        for fc in bm.faces:
            c = fc.calc_center_median()
            radial = Vector((c.x, c.y, 0)).normalized()
            if fc.normal.dot(radial) < -0.5:
                fc.material_index = 1
    me = bpy.data.meshes.new(name)
    bm.to_mesh(me)
    bm.free()
    for p in me.polygons:
        p.use_smooth = smooth
    if smooth:
        me.set_sharp_from_angle(angle=math.radians(sharp_deg))
    me.materials.append(BONE)
    me.materials.append(INNER)
    return me


# ---------------------------------------------------------------- one crown
HB = 0.30            # band height
TOP = 0.34           # top of the upper rim: the points stand here
PROFILE = [          # (r, z) cross-section of band + rims, closed loop
    (0.94, -0.10), (1.055, -0.10), (1.08, -0.075), (1.08, 0.005), (1.06, 0.025), (1.0, 0.025),
    (0.972, 0.04), (0.972, 0.056), (1.0, 0.07),                    # engraved groove above the flange
    (1.0, 0.245), (0.972, 0.258), (0.972, 0.272), (1.0, 0.285),    # groove under the top rim
    (1.04, 0.285), (1.056, 0.30), (1.056, 0.325), (1.04, TOP), (0.94, TOP),
    (0.94, 0.26), (0.94, 0.17), (0.94, 0.08), (0.94, 0.0),
]

# point layout: 16 slots, front = 0; (height, half-width, kind)
SLOTS = {0: (0.66, 0.16, 'spike'), 1: (0.36, 0.11, 'spike'), 2: (0.52, 0.14, 'spike'), 3: (0.34, 0.11, 'spike'),
         4: (0.82, 0.15, 'canine'), 5: (0.32, 0.10, 'spike'), 6: (0.44, 0.12, 'spike'), 7: (0.30, 0.10, 'spike'),
         8: (0.40, 0.12, 'spike')}


def crown(name, k, height_scale, lean, curl):
    bm = bmesh.new()
    lathe(bm, [(k * r, k * z) for r, z in PROFILE], 160)
    body = finish(name + 'Band', bm, True, 28, inner=True)

    bm = bmesh.new()
    for i in range(16):
        slot = i if i <= 8 else 16 - i
        h, w, kind = SLOTS[slot]
        th = 2 * math.pi * i / 16
        if kind == 'canine':
            spike(bm, th, k, 1.0, TOP, 0.07, w * 1.1, 0.07, 0.03, 0.0, 0.2, S=2)   # diamond plate under the fang
        else:
            spike(bm, th, k, 1.0, TOP, h * height_scale, w, 0.11, 0.05, lean, 0.16 + 0.12 * h)
        # studs between the points
        gem(bm, th + math.pi / 16, k, 1.0, 0.15, 0.085, 0.045, 0.035)
    teeth_flat = finish(name + 'Points', bm, False, 0)

    bm = bmesh.new()
    for i in (4, 12):
        th = 2 * math.pi * i / 16
        canine(bm, th, k, 1.0, TOP, SLOTS[4][0] * height_scale, SLOTS[4][1], curl, 0.28, -1 if i == 4 else 1)
    fangs = finish(name + 'Fangs', bm, True, 50)
    return body, teeth_flat, fangs


def join_meshes(name, meshes, parent, rot=None, loc=(0, 0, 0)):
    objs = []
    for me in meshes:
        ob = bpy.data.objects.new(me.name, me)
        scene.collection.objects.link(ob)
        objs.append(ob)
    bpy.ops.object.select_all(action='DESELECT')
    for ob in objs:
        ob.select_set(True)
    bpy.context.view_layer.objects.active = objs[0]
    bpy.ops.object.join()
    ob = bpy.context.view_layer.objects.active
    ob.name = name
    ob.data.name = name
    if rot:
        ob.rotation_euler = rot
    ob.location = loc
    bpy.ops.object.transform_apply(location=True, rotation=True, scale=True)
    # re-centre on the parent's hinge
    ob.parent = parent
    ob.matrix_parent_inverse = parent.matrix_world.inverted()
    return ob


def empty(name, loc=(0, 0, 0), parent=None):
    e = bpy.data.objects.new(name, None)
    e.empty_display_type = 'PLAIN_AXES'
    e.empty_display_size = 0.2
    scene.collection.objects.link(e)
    e.location = loc
    if parent:
        e.parent = parent
    return e


def crown_parts(name, k, hs, lean, curl):
    return list(crown(name, k, hs, lean, curl))


K_LO, K_UP = 0.90, 1.0
GAP = 0.16                                  # closed: upper point base sits this far above the lower point base
Z_LO_TOP = K_LO * TOP
Z_UP_EDGE = Z_LO_TOP + GAP                  # world z of the upper crown's point base (closed)
HINGE = (0, K_LO * 0.99, (Z_LO_TOP + Z_UP_EDGE) / 2)

root = empty('CrownedDogsMaw')
jaw_up = empty('JawUpper', HINGE, root)
jaw_lo = empty('JawLower', HINGE, root)
bpy.context.view_layer.update()

lower = join_meshes('CrownLower', crown_parts('Lo', K_LO, 0.92, 0.06, 0.12), jaw_lo)
# upper crown: same crown flipped about Y (front stays -Y), then lifted so its point base is at Z_UP_EDGE
upper = join_meshes('CrownUpper', crown_parts('Up', K_UP, 1.0, 0.03, 0.10), jaw_up,
                    rot=(0, math.pi, 0), loc=(0, 0, Z_UP_EDGE + K_UP * TOP))

# ---------------------------------------------------------------- baked ambient occlusion -> vertex colours (each jaw on its own)
world = bpy.data.worlds.new('World')
scene.world = world
world.light_settings.distance = 0.32
scene.render.engine = 'CYCLES'
scene.cycles.device = 'CPU'
scene.cycles.samples = 128
scene.render.bake.target = 'VERTEX_COLORS'


def bake_ao(ob, other):
    other.hide_render = True
    me = ob.data
    ca = me.color_attributes.new('AO', 'BYTE_COLOR', 'CORNER')
    me.color_attributes.active_color = ca
    me.color_attributes.render_color_index = me.color_attributes.active_color_index
    bpy.ops.object.select_all(action='DESELECT')
    ob.select_set(True)
    bpy.context.view_layer.objects.active = ob
    bpy.ops.object.bake(type='AO')
    other.hide_render = False
    for d in ca.data:                       # keep it gentle: crevices darken, open surfaces stay as they are
        a = d.color[0]
        v = 0.34 + 0.66 * a
        d.color = (v, v, v, 1)


bake_ao(lower, upper)
bake_ao(upper, lower)

# ---------------------------------------------------------------- animation (degrees: upper opens negative, lower positive)
def keys(ob, frames):
    for f, deg in frames:
        ob.rotation_euler = (math.radians(deg), 0, 0)
        ob.keyframe_insert('rotation_euler', index=0, frame=f)
    return ob.animation_data.action


CLIPS = {
    # loading loop: wind up, snap shut, recoil, hold, reopen (2 s)
    'Bite': ([(0, -30), (10, -35), (16, -1.5), (19, -7), (22, -2.5), (28, -4), (42, -30), (60, -30)],
             [(0, 7), (10, 9), (16, 0.5), (19, 2.5), (22, 0.8), (28, 1), (42, 7), (60, 7)]),
    # breathing (4 s)
    'Idle': ([(0, -28), (60, -33), (120, -28)], [(0, 6), (60, 8), (120, 6)]),
    # intro: closed -> open (1 s)
    'Open': ([(0, -2), (10, -4), (30, -30)], [(0, 0.5), (10, 1), (30, 7)]),
}
for clip, (up_k, lo_k) in CLIPS.items():
    for ob, kk in ((jaw_up, up_k), (jaw_lo, lo_k)):
        if ob.animation_data:
            ob.animation_data.action = None
        act = keys(ob, kk)
        act.name = f'{clip}_{ob.name}'
        tr = ob.animation_data.nla_tracks.new()
        tr.name = clip
        st = tr.strips.new(clip, 0, act)
        st.action_frame_end = kk[-1][0]
        tr.mute = False
        ob.animation_data.action = None
# rest pose = logo pose (tracks muted so the static pose is what gets exported as rest)
for ob in (jaw_up, jaw_lo):
    for tr in ob.animation_data.nla_tracks:
        tr.mute = True
jaw_up.rotation_euler = (math.radians(-24), 0, 0)
jaw_lo.rotation_euler = (math.radians(6), 0, 0)
scene.frame_start, scene.frame_end = 0, 120

if BLEND:
    bpy.ops.wm.save_as_mainfile(filepath=BLEND)

bpy.ops.export_scene.gltf(filepath=OUT, export_format='GLB', export_apply=True, export_yup=True,
                          export_animations=True, export_animation_mode='NLA_TRACKS', export_force_sampling=True,
                          export_materials='EXPORT', export_vertex_color='ACTIVE')
tris = sum(len(p.vertices) - 2 for ob in (upper, lower) for p in ob.data.polygons)
print('OK', OUT, 'tris', tris)
