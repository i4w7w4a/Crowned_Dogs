"""Pack the game hairstyles into apps/web/packs/hair-<sex>.glb (cut per style by apps/web/scripts/split-packs.ts), children of the head bone.

usage: blender -b --factory-startup --python pack.py -- GAME_DIR TEXTURE.png [female|male]
GAME_DIR holds game_<style>.blend from game_cards.py (rest-space meshes). Like the beards, every
mesh is placed in the head bone's frame taken from persona-<sex>.glb's own node graph, so the
runtime only has to add the pack under the `head` node. Then run finish.py and meshopt (README).
"""
import bpy, sys, json, struct
from pathlib import Path
from mathutils import Matrix, Vector, Quaternion

args = sys.argv[sys.argv.index('--') + 1:]
game_dir, tex_p = Path(args[0]), args[1]
ROOT = Path(__file__).resolve().parents[2]
SEX = args[2] if len(args) > 2 else 'female'
STYLES = {'female': ('pixie', 'bob', 'braid'), 'male': ('crop', 'slick', 'medium', 'ponytail')}[SEX]

raw = (ROOT / f'apps/web/models/persona-{SEX}.glb').read_bytes()
jn = struct.unpack_from('<I', raw, 12)[0]
source = json.loads(raw[20:20 + jn])
parents = {c: i for i, n in enumerate(source['nodes']) for c in n.get('children', [])}


def world_node(i):
    n = source['nodes'][i]
    if 'matrix' in n:
        m = Matrix([[n['matrix'][r + c * 4] for c in range(4)] for r in range(4)])
    else:
        q = n.get('rotation', [0, 0, 0, 1])
        m = Matrix.Translation(Vector(n.get('translation', [0, 0, 0]))) @ Quaternion((q[3], q[0], q[1], q[2])).to_matrix().to_4x4() @ Matrix.Diagonal((*n.get('scale', [1, 1, 1]), 1))
    return world_node(parents[i]) @ m if i in parents else m


head_index = next(i for i, n in enumerate(source['nodes']) if n.get('name') == 'head')
C = Matrix(((1, 0, 0, 0), (0, 0, -1, 0), (0, 1, 0, 0), (0, 0, 0, 1)))
HEAD = C @ world_node(head_index) @ C.inverted()
print('HEAD at', HEAD.translation)

bpy.ops.wm.read_factory_settings(use_empty=True)
sc = bpy.context.scene


def material(name, color, rough, tex=None):
    m = bpy.data.materials.new(name); m.use_nodes = True
    b = m.node_tree.nodes['Principled BSDF']
    b.inputs['Base Color'].default_value = (*color, 1)
    b.inputs['Roughness'].default_value = rough
    b.inputs['Specular IOR Level'].default_value = 0.25
    if tex:
        ti = m.node_tree.nodes.new('ShaderNodeTexImage'); ti.image = bpy.data.images.load(tex); ti.image.alpha_mode = 'STRAIGHT'
        m.node_tree.links.new(ti.outputs['Color'], b.inputs['Base Color'])
        m.node_tree.links.new(ti.outputs['Alpha'], b.inputs['Alpha'])
    m.use_backface_culling = False
    return m


# the old female hair's own colours: the strand texture and its dark cap
cards = material('M_Hair_Cards', (1, 1, 1), 0.72, tex_p)
cap = material('M_Hair_Cap', (0.0532, 0.0282, 0.0155), 0.8)
tie = material('M_Hair_Tie', (0.012, 0.012, 0.013), 0.6)
# the crop's faded sides: stubble from fade_texture.py
fade = material('M_Hair_Fade', (1, 1, 1), 0.72, str(Path(__file__).with_name('hair-fade.png')))

objs = []
for style in STYLES:
    with bpy.data.libraries.load(str(game_dir / f'game_{style}.blend')) as (src, dst):
        dst.objects = [n for n in src.objects if n.startswith(f'Hair_{style}_')]
    for o in dst.objects:
        sc.collection.objects.link(o)
        o.name = o.data.name = o.name.split('.')[0]  # the HD curves held the plain name in the source file
        o.data.materials.clear()
        o.data.materials.append(cap if o.name.endswith('_cap') else tie if o.name.endswith('_tie') else fade if o.name.endswith('_fade') else cards)
        objs.append(o)
        print('MESH', o.name, len(o.data.polygons), 'faces')


def mask_quad(name, mat):
    """A hidden 1 mm quad: finish.py hangs a head mask on its material, so the mask skips the WEBP export and
    travels with the pack."""
    me = bpy.data.meshes.new(name)
    me.from_pydata([(0, 0, 0), (.001, 0, 0), (.001, 0, .001), (0, 0, .001)], [], [(0, 1, 2, 3)])
    me.uv_layers.new().data.foreach_set('uv', (0, 0, 1, 0, 1, 1, 0, 1))
    me.materials.append(material(mat, (1, 1, 1), 1.0))
    o = bpy.data.objects.new(name, me); sc.collection.objects.link(o)
    me.transform(HEAD)  # at the bone once the loop below takes the head's frame out
    objs.append(o)


if SEX == 'male':
    mask_quad('Hair_scalp', 'M_Hair_Scalp')   # the scans' buzz cuts (scalp_mask.py)
    for style in ('slick', 'ponytail'):       # the painted hairlines under these two (scalp_mask.py)
        mask_quad(f'Hair_line_{style}', f'M_Hair_Line_{style}')

for o in objs:
    # baked into the vertices so the exported node sits at the bone with no offset of its own
    o.data.transform(HEAD.inverted(), shape_keys=True)  # and the per-face fits (game_cards.py)
    o.matrix_world = Matrix.Identity(4)
    o.select_set(True)
out = ROOT / f'output/hair-hd/raw/hair-{SEX}.glb'
out.parent.mkdir(parents=True, exist_ok=True)
# the slick's and the ponytail's hairline strips carry their roots' fade-in as vertex alpha (game_cards.py)
bpy.ops.export_scene.gltf(filepath=str(out), export_format='GLB', use_selection=True, export_yup=True,
    export_materials='EXPORT', export_image_format='WEBP', export_animations=False, export_cameras=False,
    export_lights=False, export_extras=False, export_vertex_color='NAME', export_vertex_color_name='Color')
print('EXPORTED', out)
