"""Static preview model of armour batch 2 for the Снаряжение item viewer.

usage: blender -b -P preview_gear.py -- GEAR.glb RECOLOUR_DIR OUT.glb

GEAR.glb is dress_gear.py's export (textures already 1K). The pieces leave the skeleton in the rest pose,
the body and the hair go, and each RECOLOUR_DIR/M_<Name>.color.1024.webp adds a <Name> copy of the Suit.
Node names are what crates/game-client/src/workshop.rs models() points at (gear-<sex>#<Name>).
"""
import bpy, sys, os, glob

gear_p, recolours, out_p = sys.argv[sys.argv.index('--') + 1:][:3]
bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.ops.import_scene.gltf(filepath=gear_p)
keep = ('Suit', 'Ryvok', 'Boots', 'Cap', 'Sfera', 'Plita4')
for o in list(bpy.data.objects):
    if o.type != 'MESH' or o.name.split('.')[0] not in keep:
        continue
    W = o.matrix_world.copy()
    for m in list(o.modifiers): o.modifiers.remove(m)
    o.parent = None; o.matrix_world = W
    o.vertex_groups.clear()
for o in list(bpy.data.objects):
    if o.type != 'MESH' or o.name.split('.')[0] not in keep:
        bpy.data.objects.remove(o, do_unlink=True)
suit = bpy.data.objects['Suit']
for f in sorted(glob.glob(os.path.join(recolours, 'M_*.color.1024.webp'))):
    name = os.path.basename(f).split('.')[0][2:]
    o = suit.copy(); o.data = suit.data.copy(); o.name = name
    suit.users_collection[0].objects.link(o)
    mat = suit.data.materials[0].copy(); mat.name = f'M_{name}'
    img = bpy.data.images.load(f)
    for n in mat.node_tree.nodes:
        if n.type == 'TEX_IMAGE' and any(l.to_socket.name == 'Base Color' for l in n.outputs[0].links):
            n.image = img
    o.data.materials[0] = mat
    print('RECOLOUR', name)
for o in bpy.data.objects: print('NODE', o.name, len(o.data.polygons))
bpy.ops.export_scene.gltf(filepath=out_p, export_format='GLB', export_animations=False, export_skins=False,
                          export_image_format='WEBP', export_yup=True)
print('EXPORTED', out_p)
