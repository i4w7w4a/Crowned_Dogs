"""Puts armour batch 2 (fitted by run_batch2.sh) on a Persona's game animation file and exports the new pieces.

usage: blender -b -P dress_gear.py -- ANIM_ARMOR.blend FITDIR PREFIX CUT OUT.glb

ANIM_ARMOR.blend is the game animation file already wearing kit A (armor-game/male-armor-anim.blend or
fem-armor-anim.blend); FITDIR holds PREFIX-{ryvok,cap,sfera,plita4,vest,boots}.blend; the refitted Vest (Plita-2) replaces kit A's.
- The field suit is cut straight across at CUT metres (its boots go; boots are their own item now).
- Ryvok, Cap, Sfera, Plita4, Boots and the hair squeezed under each hat (FemHair*_Cap, _Sfera) move onto the
  animation armature; whatever rides the head bone follows it where that bone sits elsewhere (female: 2 cm).
- Each fitted body's hide_<NAME> group becomes the point attribute _<name> on the game body (nearest vertex).
  Suit and Ryvok stop hiding the body below CUT + 2 cm, so bare feet show when no boots are worn.
- Pieces are thinned to phone sizes and their materials named M_<NAME>.
- A hard vest (OVER) is fitted as modelled; the suit (and Ryvok) gets its point attribute `_plita4` = 1 where it pokes
  through the vest or sits in its shell, so the game hides those suit faces while the vest is worn.
OUT.glb gets the armature, the body (for its masks) and the cut suit plus the new pieces, without animations;
merge_gear.mjs then joins them into the game model.
"""
import bpy, bmesh, sys
from mathutils import Matrix, Vector
from mathutils.kdtree import KDTree

args = sys.argv[sys.argv.index('--') + 1:]
anim_p, fitdir, px, cut, out_p = args[:5]
CUT = float(cut)
ITEMS = [('ryvok', 'Ryvok'), ('cap', 'Cap'), ('sfera', 'Sfera'), ('plita4', 'Plita4'), ('vest', 'Vest'), ('boots', 'Boots')]
TRIS = {'Ryvok': 16000, 'Cap': 5000, 'Sfera': 9000, 'Plita4': 12000, 'Vest': 12000, 'Boots': 10000}
HATS = ('Cap', 'Sfera')
OVER = {'Plita4': 0.004, 'Vest': 0.004}  # hard vests: suit within this far below their surface counts as poking through

bpy.ops.wm.open_mainfile(filepath=anim_p)
arm = bpy.data.objects['Armature']
if arm.animation_data: arm.animation_data.action = None
for pb in arm.pose.bones: pb.matrix_basis.identity()
body = next(o for o in arm.children if o.type == 'MESH' and o.name in ('MaleBody', 'FemBody'))
coll = body.users_collection[0]
tris = lambda o: sum(len(p.vertices) - 2 for p in o.data.polygons)
# kit A's soft-fitted vest gives way to the rigid refit
for o in [o for o in arm.children if o.name.split('.')[0] == 'Vest']:
    d = o.data; mats = [m for m in d.materials if m]
    bpy.data.objects.remove(o, do_unlink=True); bpy.data.meshes.remove(d)
    for m in mats:
        if not m.users: m.name = 'old_' + m.name  # the refit's own material takes the name M_Vest

# ---- the field suit ends at the ankle
suit = bpy.data.objects['Suit']
W = suit.matrix_world
bm = bmesh.new(); bm.from_mesh(suit.data)
co = W.inverted() @ Vector((0, 0, CUT))
no = (W.to_3x3().transposed() @ Vector((0, 0, 1))).normalized()
bmesh.ops.bisect_plane(bm, geom=bm.verts[:] + bm.edges[:] + bm.faces[:], plane_co=co, plane_no=no, clear_inner=True)
bm.to_mesh(suit.data); bm.free()
print('SUIT cut at', CUT, 'tris', tris(suit))

def thin(o, r):
    if r >= 0.999: return
    before = tris(o)
    # Tripo meshes come split along UV seams: collapse without a weld tears them into flaps
    bm = bmesh.new(); bm.from_mesh(o.data)
    bmesh.ops.remove_doubles(bm, verts=bm.verts, dist=0.0003)
    bm.to_mesh(o.data); bm.free()
    m = o.modifiers.new('Decimate', 'DECIMATE'); m.ratio = r; m.use_collapse_triangulate = True
    bpy.context.view_layer.objects.active = o
    while o.modifiers[0].name != 'Decimate': bpy.ops.object.modifier_move_up(modifier='Decimate')
    bpy.ops.object.modifier_apply(modifier='Decimate')
    print('DECIMATED', o.name, before, '->', tris(o))

Wt = body.matrix_world
masks = {}
pieces = []
for tag, name in ITEMS:
    path = f'{fitdir}/{px}-{tag}.blend'
    before = set(bpy.data.objects)
    with bpy.data.libraries.load(path, link=False) as (src, dst):
        dst.objects = [n for n in src.objects if n == name or n.endswith('_' + name) or n == body.name]
    new = [o for o in bpy.data.objects if o not in before]
    src_arm = next(o for o in new if o.type == 'ARMATURE')
    src_body = next(o for o in new if o.type == 'MESH' and o.name.startswith(body.name))
    wear = [o for o in new if o.type == 'MESH' and o is not src_body]
    D = arm.matrix_world @ arm.data.bones['head'].matrix_local @ (src_arm.matrix_world @ src_arm.data.bones['head'].matrix_local).inverted()
    for o in wear:
        Wo = o.matrix_world.copy()
        o.parent = None
        o.data.transform(Wo)
        if name in HATS: o.data.transform(D)
        o.matrix_world = Matrix()
        o.parent = arm; o.matrix_parent_inverse = Matrix()
        for m in o.modifiers:
            if m.type == 'ARMATURE': m.object = arm
        for c in o.users_collection: c.objects.unlink(o)
        coll.objects.link(o)
        o.name = o.name.split('.')[0]
        if o.name == name:
            for i, m in enumerate(o.data.materials):
                if m: m.name = f'M_{name}' if i == 0 else f'M_{name}_{i}'
            thin(o, min(1.0, TRIS[name] / max(1, tris(o))))
        else:  # hat hair wears the hair's own materials
            for i, m in enumerate(o.data.materials):
                own = bpy.data.materials.get(m.name.split('.')[0])
                if own and own is not m: o.data.materials[i] = own
            if 'HairCap' in o.name: thin(o, 0.25)
        print('PIECE', o.name, 'verts', len(o.data.vertices), 'tris', tris(o), 'head move' if name in HATS else '',
              [round(x, 4) for x in D.translation] if name in HATS else '')
        pieces.append(o)
    # the covered body, from the fitted body's nearest vertex
    g = src_body.vertex_groups.get('hide_' + name)
    if g:
        Ws = src_body.matrix_world
        kd = KDTree(len(src_body.data.vertices))
        for v in src_body.data.vertices: kd.insert(Ws @ v.co, v.index)
        kd.balance()
        w = [0.0] * len(src_body.data.vertices)
        for v in src_body.data.vertices:
            for e in v.groups:
                if e.group == g.index: w[v.index] = e.weight
        far = 0.0
        vals = []
        for v in body.data.vertices:
            _, i, d = kd.find(Wt @ v.co); far = max(far, d); vals.append(w[i])
        masks['_' + name.lower()] = vals
        print('MASK', '_' + name.lower(), sum(1 for x in vals if x >= 0.5), 'of', len(vals), 'farthest match', round(far, 4))
    ad, bd = src_arm.data, src_body.data
    bpy.data.objects.remove(src_body, do_unlink=True); bpy.data.objects.remove(src_arm, do_unlink=True)
    bpy.data.meshes.remove(bd); bpy.data.armatures.remove(ad)

# a hard vest stays as modelled: suit faces that poke through it (or sit in its shell) hide while it is worn
from mathutils.bvhtree import BVHTree
LIMBS = ('upperarm', 'lowerarm', 'hand', 'thigh', 'calf', 'foot')
ARMS = ('upperarm', 'lowerarm', 'hand', 'thumb', 'index', 'middle', 'ring', 'pinky')
for vest in [o for o in pieces if o.name in OVER]:
    # it rides the torso only: side panels weighted to an arm flared out after the elbow
    limb = {g.index for g in vest.vertex_groups if any(k in g.name.lower() for k in LIMBS)}
    torso = vest.vertex_groups.get('spine_02') or vest.vertex_groups.new(name='spine_02')
    moved = 0
    for v in vest.data.vertices:
        drop = [e.group for e in v.groups if e.group in limb and e.weight > 0]
        if not drop: continue
        moved += 1
        for g in drop: vest.vertex_groups[g].remove([v.index])
        left = sum(e.weight for e in v.groups if e.group not in limb)
        if left < 1e-4: torso.add([v.index], 1.0, 'REPLACE')
        else:
            for e in v.groups: e.weight /= left
    print('TORSO ONLY', vest.name, moved, 'verts lost their limb weights')
    vb = bmesh.new(); vb.from_mesh(vest.data); vb.transform(vest.matrix_world)
    tree = BVHTree.FromBMesh(vb); vb.free()
    for s in [o for o in [suit] + pieces if o.name in ('Suit', 'Ryvok')]:
        Ws, N = s.matrix_world, s.matrix_world.to_3x3().inverted().transposed()
        # the sleeve leaves the vest as soon as the arm moves: hidden, it tears into a strip in a bent-arm pose
        arms = {g.index for g in s.vertex_groups if any(k in g.name.lower() for k in ARMS)}
        vals = []
        for v in s.data.vertices:
            if sum(e.weight for e in v.groups if e.group in arms) >= 0.3: vals.append(0.0); continue
            p = Ws @ v.co; n = (N @ v.normal).normalized()
            hit = tree.ray_cast(p + n * OVER[vest.name], -n, OVER[vest.name] + 0.06)[0]
            vals.append(1.0 if hit is not None else 0.0)
        key = '_' + vest.name.lower()
        a = s.data.attributes.get(key) or s.data.attributes.new(key, 'FLOAT', 'POINT')
        a.data.foreach_set('value', vals)
        print('UNDER', vest.name, s.name, int(sum(vals)), 'of', len(vals), 'hide')

low = [(Wt @ v.co).z < CUT + 0.02 for v in body.data.vertices]
suit_mask = body.data.attributes['_suit']
vals = [0.0] * len(body.data.vertices); suit_mask.data.foreach_get('value', vals)
masks['_suit'] = vals
for key in ('_suit', '_ryvok'):
    if key in masks: masks[key] = [0.0 if l else x for x, l in zip(masks[key], low)]
for key, vs in masks.items():
    a = body.data.attributes.get(key) or body.data.attributes.new(key, 'FLOAT', 'POINT')
    a.data.foreach_set('value', vs)
    print('BODY', key, sum(1 for x in vs if x >= 0.5), 'hidden of', len(vs))
for o in bpy.data.objects:
    if o.type == 'MESH' and o.parent == arm and o.name.startswith('FemHair'):
        print('HAIR', o.name, 'verts', len(o.data.vertices), 'tris', tris(o))

bpy.data.orphans_purge(do_recursive=True)
for o in bpy.context.view_layer.objects: o.select_set(False)
hair = [o for o in arm.children if o.name in ('FemHairCards', 'FemHairCap')]  # merge_gear's reference for the hair offset
for o in [arm, body, suit] + pieces + hair: o.select_set(True)
bpy.context.view_layer.objects.active = arm
bpy.ops.export_scene.gltf(filepath=out_p, export_format='GLB', use_selection=True, export_animations=False,
                          export_skins=True, export_attributes=True, export_yup=True, export_apply=False,
                          export_morph=False)
print('EXPORTED', out_p)
