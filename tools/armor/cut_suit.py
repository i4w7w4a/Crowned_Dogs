"""Kit A field suit without its boots, for the separate boots (slot `boots`) and as the base the batch 2 pieces fit over.

usage: blender -b -P cut_suit.py -- FITTED.blend OUT.blend CUT_Z

FITTED.blend is armor/fitted/<sex>-armor.blend. Keeps the Persona and the Suit only (Vest, Helmet and the hair made
for the helmet go), and deletes every Suit vertex below CUT_Z metres: the trousers then end a few cm inside the boots.
"""
import bpy, bmesh, sys

src, dst, cut = sys.argv[sys.argv.index('--') + 1:][:3]
cut = float(cut)
bpy.ops.wm.open_mainfile(filepath=src)
for o in list(bpy.data.objects):
    if o.name in ('Vest', 'Helmet') or o.name.endswith('_Helmet'):
        bpy.data.objects.remove(o, do_unlink=True)
suit = bpy.data.objects['Suit']
bm = bmesh.new(); bm.from_mesh(suit.data)
M = suit.matrix_world
gone = [v for v in bm.verts if (M @ v.co).z < cut]
bmesh.ops.delete(bm, geom=gone, context='VERTS')
bm.to_mesh(suit.data); bm.free(); suit.data.update()
print(f'CUT {len(gone)} suit verts below {cut} m, {len(suit.data.vertices)} left')
bpy.data.orphans_purge(do_recursive=True)
bpy.ops.wm.save_as_mainfile(filepath=dst, compress=True)
print('SAVED', dst)
