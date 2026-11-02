"""Build the bounded 3D set of a «Каскад» map, shared by Blender and the Rust renderer.

Blender 5.2 --background --factory-startup --python tools/kaskad-stage/build_stage.py -- --map dam|drybed|intake|backwater|railyard|sludgeponds [--no-render]
Same method as tools/blocks-stage: the source camera is fixed, only VIEW moves,
traced objects get their own depth, no depth displacement or zoom is used.
"""
import argparse
import hashlib
import json
import math
import sys
from array import array
from pathlib import Path

import bpy
from mathutils import Vector

REPO = Path(__file__).resolve().parents[2]
parser = argparse.ArgumentParser()
parser.add_argument('--map', choices=['dam', 'drybed', 'intake', 'backwater', 'railyard', 'sludgeponds'], required=True)
parser.add_argument('--no-render', action='store_true')
args = parser.parse_args(sys.argv[sys.argv.index('--')+1:] if '--' in sys.argv else [])
MAP = args.map
WEB = REPO/f'apps/web/public/img/{MAP}-stage'
OUT = REPO/f'release-local/{MAP}-stage'
WEB.mkdir(parents=True, exist_ok=True)
OUT.mkdir(parents=True, exist_ok=True)
SOURCE = REPO/f'apps/web/public/img/{MAP}-day'
W, H, CAMERA_H, FOV = 941, 1672, 2.0, 55.0
ASPECT = W/H
FOCAL = .5/math.tan(math.radians(FOV*.5))
# Measured on the approved day plates: where the lane, parapets and hull bottoms converge.
HORIZON = {'dam': .462, 'drybed': .365, 'intake': .40, 'backwater': .395, 'railyard': .41, 'sludgeponds': .381}[MAP]
# Column where the walls of a corridor converge; the closed maps are drawn a little yawed.
VANISH_U = {'intake': .40, 'backwater': .49, 'railyard': .33}.get(MAP, .5)
TITLE = {'dam': '11 · ПЛОТИНА — гребень под сетками', 'drybed': '12 · СУХОЕ ДНО — между корпусами',
    'intake': '13 · ВОДОЗАБОР — галерея над решётками', 'backwater': '14 · ЗАТОН — сухой док',
    'railyard': '15 · СОРТИРОВКА — депо', 'sludgeponds': '16 · ШЛАМОВЫЕ ПРУДЫ — солончак'}[MAP]

bpy.ops.wm.read_factory_settings(use_empty=True)
scene = bpy.context.scene
scene.name = TITLE
scene.render.engine = 'BLENDER_EEVEE'
scene.render.resolution_x, scene.render.resolution_y = W, H
scene.render.resolution_percentage = 100
scene.render.image_settings.file_format = 'PNG'
scene.render.fps = 24
scene.frame_start, scene.frame_end = 1, 193
scene.view_settings.view_transform = 'Standard'
scene.view_settings.look = 'None'
scene.view_settings.exposure = 0
scene.view_settings.gamma = 1
scene.world = bpy.data.worlds.new('Каскад · день')
scene.world.use_nodes = True
scene.world.node_tree.nodes['Background'].inputs[0].default_value = (.28, .29, .30, 1)
scene.world.node_tree.nodes['Background'].inputs[1].default_value = .6

def collection(name):
    value = bpy.data.collections.new(name)
    scene.collection.children.link(value)
    return value

architecture = collection('01 · Своя архитектура карты')
props = collection('02 · Объёмные объекты карты')
camera_collection = collection('00 · Камеры')

def camera(name):
    data = bpy.data.cameras.new(name)
    data.sensor_fit, data.sensor_height = 'VERTICAL', 36
    data.lens = 36*FOCAL
    data.shift_y = HORIZON-.5
    data.clip_start, data.clip_end = .05, 600
    obj = bpy.data.objects.new(name, data)
    camera_collection.objects.link(obj)
    obj.location = (0, 0, CAMERA_H)
    obj.rotation_euler = (math.pi/2, 0, 0)
    return obj

projector = camera('PROJECTOR · неподвижный исходный кадр')
projector.hide_select = True
projector.lock_location = projector.lock_rotation = (True,)*3
view = camera('VIEW · продольное движение')
scene.camera = view
for frame, distance in [(1, 0), (49, 2), (97, 0), (145, -1.5), (193, 0)]:
    view.location.y = distance
    view.keyframe_insert(data_path='location', frame=frame)
for frame, label in [(1, 'ИСХОДНИК'), (49, 'ВПЕРЁД · 2 м'), (145, 'НАЗАД · 1.5 м')]:
    scene.timeline_markers.new(label, frame=frame)
scene.frame_set(1)
bpy.context.view_layer.update()

images = {}
for key in ['hall', 'clean']:
    value = bpy.data.images.load(str(SOURCE/(key+'.png')))
    if tuple(value.size) != (W, H):
        raise ValueError(f'{key}: expected {W}x{H}, got {tuple(value.size)}')
    value.colorspace_settings.name = 'sRGB'
    value.pack()
    images[key] = value

def emission(name, color, kind='side'):
    value = bpy.data.materials.new(name)
    value.use_nodes = True
    ns, links = value.node_tree.nodes, value.node_tree.links
    ns.clear()
    emit = ns.new('ShaderNodeEmission')
    emit.inputs[0].default_value = (*color, 1)
    output = ns.new('ShaderNodeOutputMaterial')
    links.new(emit.outputs[0], output.inputs[0])
    value.diffuse_color = (*color, 1)
    value['stageMaterial'] = kind
    return value

def projected(name, image, kind):
    value = emission(name, (.2, .2, .2), kind)
    ns, links = value.node_tree.nodes, value.node_tree.links
    coord = ns.new('ShaderNodeTexCoord')
    coord.name = 'ProjectorCoordinates'
    coord.object = projector
    xyz = ns.new('ShaderNodeSeparateXYZ')
    links.new(coord.outputs['Object'], xyz.inputs[0])
    def math_node(op, a, b):
        node = ns.new('ShaderNodeMath'); node.operation = op
        for i, entry in enumerate((a, b)):
            if isinstance(entry, (float, int)): node.inputs[i].default_value = entry
            else: links.new(entry, node.inputs[i])
        return node.outputs[0]
    distance = math_node('MULTIPLY', xyz.outputs['Z'], -1)
    u = math_node('MULTIPLY_ADD', math_node('DIVIDE', xyz.outputs['X'], distance), FOCAL/ASPECT)
    u.node.inputs[2].default_value = .5
    v = math_node('MULTIPLY_ADD', math_node('DIVIDE', xyz.outputs['Y'], distance), FOCAL)
    v.node.inputs[2].default_value = 1-HORIZON
    uv = ns.new('ShaderNodeCombineXYZ')
    uv.name = 'ProjectionUV'
    links.new(u, uv.inputs['X']); links.new(v, uv.inputs['Y'])
    texture = ns.new('ShaderNodeTexImage'); texture.image = image; texture.extension = 'EXTEND'
    links.new(uv.outputs[0], texture.inputs[0])
    links.new(texture.outputs['Color'], ns.get('Emission').inputs[0])
    return value

photo = projected('Фото · объекты карты', images['hall'], 'equipment')
clean = projected('Чистая архитектура', images['clean'], 'clean')
floor = projected('Земля · фотопроекция', images['clean'], 'floor')
wall = projected('Стены · фотопроекция', images['clean'], 'wall')
concrete = emission('Скрытый бетон', (.25, .24, .22))
steel = emission('Ржавая обратная сторона металла', (.16, .12, .085))
dark = emission('Тёмная внутренняя поверхность', (.07, .065, .055))

repair = []
objects = []
def ray(u, v, depth):
    return Vector(((u-.5)*ASPECT/FOCAL*depth, depth, CAMERA_H+(HORIZON-v)/FOCAL*depth))

def floor_depth(v):
    return CAMERA_H*FOCAL/max(v-HORIZON, .015)

def plane_x_depth(u, x):
    """Depth where the source ray through column u meets the vertical plane at lateral x."""
    return x/((u-.5)*ASPECT/FOCAL)

# Corridor frame: lateral L across it, s along it towards the vanishing column.
YAW = math.atan((VANISH_U-.5)*ASPECT/FOCAL)
COS, SIN = math.cos(YAW), math.sin(YAW)

def at(L, s, z=0):
    return Vector((L*COS+s*SIN, -L*SIN+s*COS, z))

def lateral_depth(u, L):
    """Depth where the source ray through column u meets the corridor wall at lateral L."""
    return L/((u-.5)*ASPECT/FOCAL*COS-SIN)

def mesh(name, vertices, faces, materials, face_materials=None, target=props, occludes=True):
    data = bpy.data.meshes.new(name)
    data.from_pydata([tuple(p) for p in vertices], [], faces)
    data.update()
    obj = bpy.data.objects.new(name, data)
    target.objects.link(obj)
    for material in materials: data.materials.append(material)
    if face_materials:
        for face, index in zip(data.polygons, face_materials): face.material_index = index
    obj['occludesFigures'] = occludes
    objects.append(obj)
    return obj

def box(name, x0, x1, y0, y1, z0, z1, material, target=architecture, occludes=True):
    """Closed axis-aligned volume; every face samples the same projected material."""
    v = [(x0,y0,z0),(x1,y0,z0),(x1,y1,z0),(x0,y1,z0),(x0,y0,z1),(x1,y0,z1),(x1,y1,z1),(x0,y1,z1)]
    faces = [(0,3,2,1),(4,5,6,7),(0,1,5,4),(1,2,6,5),(2,3,7,6),(3,0,4,7)]
    return mesh(name, v, faces, [material], target=target, occludes=occludes)

def cbox(name, L0, L1, s0, s1, z0, z1, material, target=architecture, occludes=True, z1_far=None):
    """Closed volume squared to the corridor; z1_far slopes its top towards s1."""
    top_far = z1 if z1_far is None else z1_far
    v = [at(L0,s0,z0),at(L1,s0,z0),at(L1,s1,z0),at(L0,s1,z0),at(L0,s0,z1),at(L1,s0,z1),at(L1,s1,top_far),at(L0,s1,top_far)]
    faces = [(0,3,2,1),(4,5,6,7),(0,1,5,4),(1,2,6,5),(2,3,7,6),(3,0,4,7)]
    return mesh(name, v, faces, [material], target=target, occludes=occludes)

def cquad(name, L0, L1, s0, s1, z, material, down=False, occludes=False):
    """Open floor (faces up) or ceiling (faces down) squared to the corridor."""
    face = (0,3,2,1) if down else (0,1,2,3)
    return mesh(name, [at(L0,s0,z),at(L1,s0,z),at(L1,s1,z),at(L0,s1,z)], [face], [material], target=architecture, occludes=occludes)

def standing(outline, foot=None):
    """One depth for a prop that stands on the floor: where its lowest edge meets the ground."""
    return floor_depth(foot if foot is not None else max(v for u, v in outline))

def slab(name, outline, depths, thickness, back_material=concrete, front_material=photo, direction=None, target=props, repair_plate=True):
    n = len(outline)
    depths = [depths]*n if isinstance(depths, (float, int)) else depths
    front = [ray(u, v, d) for (u, v), d in zip(outline, depths)]
    # By default the volume recedes along the source ray through its middle, so
    # its sides stay edge-on from the painted camera.
    centre = sum(front, Vector())/n
    shift = Vector(direction) if direction else (centre-Vector((0, 0, CAMERA_H))).normalized()*thickness
    back = [p+shift for p in front]
    faces = [tuple(reversed(range(n))), tuple(range(n, n*2))]
    faces += [(i, (i+1)%n, (i+1)%n+n, i+n) for i in range(n)]
    # A side or top that faces the projector covers pixels the front does not, so it
    # samples the photograph there too; only truly hidden faces get the plain colour.
    eye, points = Vector((0, 0, CAMERA_H)), front+back
    def facing(face):
        a, b, c = (points[i] for i in face[:3])
        return (b-a).cross(c-a).dot(eye-(a+b+c)/3) > 0
    indices = [0, 1]+[0 if facing(f) else 1 for f in faces[2:]]
    obj = mesh(name, points, faces, [front_material, back_material], indices, target)
    if repair_plate: repair.append([[max(0, min(1, u)), max(0, min(1, v))] for u, v in outline])
    return obj

def tube(name, a, b, radius, segments=8, material=photo, back_material=steel):
    a, b = Vector(a), Vector(b)
    axis = (b-a).normalized()
    ref = Vector((0, 0, 1)) if abs(axis.z) < .9 else Vector((1, 0, 0))
    right, up = axis.cross(ref).normalized(), axis.cross(axis.cross(ref)).normalized()
    rings = [[center+radius*(right*math.cos(i*2*math.pi/segments)+up*math.sin(i*2*math.pi/segments)) for i in range(segments)] for center in (a, b)]
    faces = [(i, (i+1)%segments, (i+1)%segments+segments, i+segments) for i in range(segments)]
    faces += [tuple(reversed(range(segments))), tuple(range(segments, segments*2))]
    vertices = rings[0]+rings[1]
    # Only camera-facing faces sample the photograph; hidden sides stay plain metal.
    mids = [(vertices[f[0]]+vertices[f[1]]+vertices[f[2]])/3 for f in faces]
    indices = []
    for face, mid in zip(faces, mids):
        normal = (vertices[face[1]]-vertices[face[0]]).cross(vertices[face[2]]-vertices[face[0]])
        indices.append(0 if normal.dot(Vector((0, 0, CAMERA_H))-mid) > 0 else 1)
    return mesh(name, vertices, faces, [material, back_material], indices)

def pole(name, u, v_top, x, base_z):
    """Upright pole standing at lateral x; the clean plate keeps it, so no repair."""
    d = plane_x_depth(u, x)
    top = ray(u, v_top, d)
    tube(name, (top.x, d, base_z), top, .09, 8)

# Everything far away (sky, far bank, distant dam) on one fixed surface. It uses
# the clean plate inside repair outlines, so a hull or a vehicle that shifts with
# parallax reveals what the clean plate has behind it.
mesh('Far backdrop · fixed distant surface', [(-260,420,-30),(260,420,-30),(260,420,200),(-260,420,200)], [(0,1,2,3)], [clean], target=architecture, occludes=False)

if MAP == 'dam':
    # Crest roadway between a gate-house wall on the left and a low parapet on the right.
    LEFT_X, RIGHT_X, PARAPET_Z = -8.0, 6.4, 1.64
    mesh('Crest roadway · y=0 in game', [(LEFT_X,-12,0),(RIGHT_X,-12,0),(RIGHT_X,330,0),(LEFT_X,330,0)], [(0,1,2,3)], [floor], target=architecture, occludes=False)
    box('Gate towers · facade line', LEFT_X-3, LEFT_X, 4, 260, 0, 34, wall)
    box('Downstream parapet', RIGHT_X, RIGHT_X+.5, 4, 330, 0, PARAPET_Z, wall)
    # Anti-drone nets: a canopy over the lane only, so the towers and the right
    # poles keep their own depth. Its photograph carries the nets and the drones.
    NET_Z = 7.5
    mesh('Anti-drone nets · canopy over the crest', [(-5,11.5,NET_Z),(3.5,11.5,NET_Z),(3.5,240,NET_Z),(-5,240,NET_Z)], [(0,3,2,1)], [clean], target=architecture, occludes=False)
    for i, (u, v_top) in enumerate([(.862, .15), (.733, .26), (.662, .315), (.632, .345)]):
        pole(f'Net pole {i+1} on the parapet', u, v_top, RIGHT_X-.2, PARAPET_Z)

    # Burnt APC against the right parapet: its far left end recedes, the right end is near.
    apc = [(.575,.495),(.58,.475),(.62,.452),(.70,.445),(.78,.447),(.84,.455),(.852,.49),(.842,.517),(.70,.522),(.60,.512)]
    apc_depth = lambda u: 38-(u-.58)/.27*9
    slab('APC · burnt closed hull', apc, [apc_depth(u) for u, v in apc], 2.4, direction=(.3,2.4,0), back_material=steel)
    # Destroyed AA gun on its gabion and sandbag platform beside the gate towers.
    gun = [(.08,.50),(.08,.47),(.10,.40),(.16,.38),(.20,.36),(.30,.335),(.365,.343),(.35,.36),(.30,.39),(.34,.42),(.38,.44),(.392,.47),(.36,.495),(.20,.505)]
    slab('AA gun emplacement · gun and gabions', gun, 32, 3.0, back_material=steel)
    # Near left: stacked concrete blocks with the ammo box, then a low heap with the cable coil.
    slab('Near blocks and ammo box', [(-.15,.455),(.07,.46),(.13,.47),(.135,.53),(.15,.54),(.17,.60),(.175,.705),(.10,.712),(-.15,.712)], 7.7, 1.2, direction=(-1.0,1.4,0))
    heap = [(-.15,.72),(.08,.72),(.16,.76),(.22,.82),(.28,.86),(.30,.93),(.29,1.10),(-.15,1.10)]
    slab('Near rubble heap and cable coil', heap, 4.4, .8, direction=(-.4,.9,0))
    # Rusty frames and barrels along the right parapet.
    slab('Parapet junk · frames and barrels', [(.80,.53),(.80,.50),(.86,.495),(.93,.49),(1.15,.49),(1.15,.62),(.95,.585),(.88,.56)], 22, 1.0, direction=(.9,.8,0), back_material=steel)
    TILE, GROUND_TINT = 'concrete', (.31, .30, .27)
elif MAP == 'drybed':
    mesh('Dry reservoir bed · y=0 in game', [(-60,-12,0),(60,-12,0),(60,420,0),(-60,420,0)], [(0,1,2,3)], [floor], target=architecture, occludes=False)
    # Long tanker barge on the left, receding from about 50 m to 75 m, with its collapsed deck crane.
    barge = [(-.15,.39),(-.15,.20),(.06,.21),(.10,.20),(.25,.18),(.34,.18),(.34,.21),(.28,.26),(.38,.30),(.45,.33),(.465,.39),(.25,.392)]
    barge_depth = lambda u: 50+max(0, min(1, u/.46))*25
    slab('Tanker barge · stranded hull and crane', barge, [barge_depth(u) for u, v in barge], 9, direction=(-9,4,0), back_material=steel)
    # Upright river tug on the right, bow towards the lane, propeller exposed.
    tug = [(.77,.38),(.78,.28),(.80,.22),(.85,.18),(.87,.14),(.92,.12),(.93,.06),(.96,.06),(.97,.09),(1.15,.09),(1.15,.45),(.95,.42),(.85,.41),(.80,.42)]
    slab('River tug · upright hull', tug, 36, 6, direction=(4,6,0), back_material=steel)
    # Kept in the clean plate too, so these far shapes need depth but no repair.
    slab('Old pier on stilts', [(.665,.335),(.78,.335),(.78,.40),(.665,.40)], 55, 3, front_material=clean, repair_plate=False)
    slab('Far wrecked boats', [(.46,.39),(.47,.36),(.52,.35),(.58,.355),(.62,.37),(.62,.392)], 64, 4, front_material=clean, repair_plate=False)
    # Near right concrete ruin with the trench sandbags.
    slab('Near concrete ruin', [(.82,.62),(.83,.53),(.87,.50),(.88,.475),(.90,.47),(.93,.50),(1.15,.50),(1.15,.68),(.95,.65),(.88,.63)], 7.6, 1.6, direction=(1.2,1.2,0))
    # Bottom left: camouflage net over junk, then bollard, beams and rope close to the camera.
    slab('Camouflage net over junk', [(-.15,.45),(.04,.44),(.12,.46),(.18,.50),(.25,.55),(.30,.62),(.32,.66),(-.15,.68)], 6.6, 1.2, direction=(-1.0,1.0,0))
    junk = [(-.15,.68),(.12,.66),(.22,.70),(.30,.78),(.38,.85),(.35,.95),(.25,1.10),(-.15,1.10)]
    slab('Bollard, beams and rope', junk, 4.8, .7, direction=(-.4,.8,0), back_material=steel)
    TILE, GROUND_TINT = 'silt', (.36, .33, .29)
elif MAP == 'intake':
    # Gallery over the trash racks: rack wall on the left, low parapet and openings on the right.
    LEFT, RIGHT, CEILING, END = -2.0, 5.62, 6.0, 90
    cquad('Gallery floor · y=0 in game', LEFT, RIGHT, -12, END, 0, floor)
    cbox('Trash-rack wall and railings', LEFT-.6, LEFT, -12, END, -3, CEILING, wall)
    cbox('Reservoir parapet', RIGHT, RIGHT+.5, -12, END, 0, 1.5, wall)
    # The roof fell in between 18 and 27 m: through the gap the far bank stays far away.
    cbox('Reservoir wall · near bays', RIGHT, RIGHT+.5, -12, 18, 1.5, CEILING, wall)
    cbox('Reservoir wall · far bays and windows', RIGHT, RIGHT+.5, 27.4, END, 1.5, CEILING, wall)
    cquad('Gallery ceiling and beams', LEFT-.6, RIGHT+.5, -12, END, CEILING, wall, down=True)
    cbox('Far end wall', LEFT-.6, RIGHT+.5, END, END+1, 0, CEILING, wall)
    # Collapse at the broken opening: the heap and the snapped column above it.
    rubble = [(.64,.46),(.66,.445),(.70,.43),(.715,.36),(.72,.30),(.74,.27),(.77,.27),(.775,.33),(.77,.40),(.80,.42),(.84,.41),(.88,.42),(.93,.41),(.96,.42),(.96,.50),(.90,.50),(.85,.49),(.78,.485),(.72,.475),(.66,.47)]
    slab('Collapse rubble and broken column', rubble, 22, 6, direction=(.6,6,0))
    debris = [(-.15,.60),(.07,.60),(.13,.64),(.13,.70),(.05,.78),(.03,.82),(-.15,.83)]
    slab('Curb debris', debris, standing(debris), .9, direction=(-.4,.8,0))
    TILE, GROUND_TINT = 'concrete', (.27, .27, .26)
elif MAP == 'backwater':
    # Dry dock: stepped walls 14 m apart, the closed caisson gate 55 m ahead.
    LEFT, RIGHT, END = -6.6, 7.5, 55
    cquad('Dock floor · y=0 in game', LEFT, RIGHT, -12, END, 0, floor)
    cbox('Left dock wall', LEFT-1.5, LEFT, -12, END, 0, 14.7, wall, z1_far=8.9)
    cbox('Right dock wall', RIGHT, RIGHT+1.5, -12, END, 0, 13.5, wall, z1_far=8.9)
    cbox('Caisson gate', LEFT-1.5, RIGHT+1.5, END, END+2, 0, 8.6, wall)
    # The rusty ship stands on keel blocks along the right wall, stern towards the lane.
    hull = [(.585,-.05),(.585,.02),(.58,.12),(.575,.20),(.578,.26),(.595,.33),(.625,.39),(.64,.42),(.72,.42),(.80,.425),(.89,.43),(1.15,.43),(1.15,-.05)]
    hull_depth = lambda u: 20+max(0, .70-u)/.12*20
    slab('Ship hull · stern and propeller', hull, [hull_depth(u) for u, v in hull], 0, direction=(1,10,0), back_material=steel)
    blocks = [(.64,.425),(1.15,.425),(1.15,.515),(.98,.51),(.87,.505),(.80,.48),(.755,.47),(.70,.455),(.65,.44)]
    slab('Keel blocks', blocks, [17.5+max(0, .87-u)/.22*20 for u, v in blocks], 0, direction=(.5,8,0))
    TILE, GROUND_TINT = 'concrete', (.33, .32, .30)
elif MAP == 'railyard':
    # Engine shed: raised track bed with the locomotive on the left, inspection pit, brick wall on the right.
    LEFT, RIGHT, ROOF, END, PIT = -8.4, 8.6, 11.0, 64, -1.76
    cquad('Shed floor · y=0 in game', PIT, RIGHT, -12, END, 0, floor)
    cquad('Inspection pit bottom', -3.2, PIT, -12, END, -1.3, wall)
    cbox('Inspection pit near wall', PIT-.1, PIT, -12, END, -1.3, 0, wall, occludes=False)
    cbox('Track bed under the locomotive', LEFT, -3.2, -12, END, -1.3, 1.2, wall)
    cbox('Left shed wall', LEFT-.6, LEFT, -12, END, 1.2, ROOF, wall)
    cbox('Brick wall with arched windows', RIGHT, RIGHT+.6, -12, END, 0, ROOF, wall)
    cquad('Roof trusses and skylights', LEFT-.6, RIGHT+.6, -12, END, ROOF, wall, down=True)
    cbox('Far gates', LEFT-.6, RIGHT+.6, END, END+1, -1.3, ROOF, wall)
    loco = [(-.15,.12),(0,.13),(.06,.15),(.12,.19),(.15,.21),(.22,.22),(.25,.27),(.29,.31),(.29,.45),(.22,.47),(.15,.50),(.10,.53),(.05,.58),(0,.62),(-.15,.64)]
    slab('Diesel locomotive', loco, [min(60, lateral_depth(u, -2.0)) for u, v in loco], 0, direction=(-3*COS,3*SIN,0), back_material=steel)
    wheels = [(.48,.44),(.48,.35),(.54,.35),(.55,.42),(.58,.40),(.64,.39),(.72,.395),(.745,.41),(.75,.46),(.66,.465),(.57,.46)]
    slab('Wheelsets and far machines', wheels, standing(wheels), 2.5, direction=(1.2,2.2,0), back_material=steel)
    bench = [(.75,.51),(.75,.38),(.80,.36),(.86,.37),(.90,.35),(.95,.36),(1.15,.36),(1.15,.53),(.95,.52),(.85,.515)]
    slab('Workbench and shelves', bench, standing(bench, .515), 1.5, direction=(.6,1.2,0))
    TILE, GROUND_TINT = 'concrete', (.30, .30, .30)
else:
    # Salt flat to the horizon: only the ground, the near power pole and a far excavator.
    mesh('Salt crust · y=0 in game', [(-220,-12,0),(220,-12,0),(220,420,0),(-220,420,0)], [(0,1,2,3)], [floor], target=architecture, occludes=False)
    pole('Near power pole', .925, .315, 39.8, 0)
    slab('Far excavator', [(.085,.39),(.085,.375),(.10,.37),(.12,.366),(.135,.362),(.148,.372),(.148,.392)], 200, 0, direction=(-3,6,0), back_material=steel)
    TILE, GROUND_TINT = 'salt', (.66, .66, .64)

def noise(x,y):
    return math.sin(x*12.13+y*17.19)*math.sin(x*4.37-y*8.23)

def tile(name,base):
    size=256
    image=bpy.data.images.new(name,width=size,height=size,alpha=False)
    image.colorspace_settings.name='Non-Color'
    pixels=array('f')
    for y in range(size):
        for x in range(size):
            a,b=x/size*math.tau,y/size*math.tau
            grain=noise(math.sin(a)*5,math.sin(b)*5)*.045+noise(math.cos(a)*17,math.cos(b)*17)*.018
            chip=max(0,noise(math.sin(a)*31,math.cos(b)*31)-.65)*.22
            pixels.extend([max(.015,min(.8,c+grain+chip)) for c in base]+[1.])
    image.pixels.foreach_set(pixels)
    image.filepath_raw=str(WEB/(name+'.png'));image.file_format='PNG';image.save()

tile(TILE,GROUND_TINT)

# Orthorectified patch of the map's own ground for surfaces outside the photographed frustum.
patch_data=bpy.data.meshes.new('TEMP · ground material bake')
patch_data.from_pydata([(-1.3,5.5,0),(1.3,5.5,0),(1.3,8.1,0),(-1.3,8.1,0)],[],[(0,1,2,3)])
patch_data.update()
patch=bpy.data.objects.new('TEMP · ground material bake',patch_data)
scene.collection.objects.link(patch)
patch_data.materials.append(floor)
uv=patch_data.uv_layers.new(name='Ground material UV')
for loop,co in zip(uv.data,[(0,1),(1,1),(1,0),(0,0)]):loop.uv=co
ground_image=bpy.data.images.new('Ground · orthorectified material',width=512,height=512,alpha=False)
ground_image.colorspace_settings.name='sRGB'
bake_node=floor.node_tree.nodes.new('ShaderNodeTexImage');bake_node.image=ground_image
floor.node_tree.nodes.active=bake_node
bpy.ops.object.select_all(action='DESELECT');patch.select_set(True);bpy.context.view_layer.objects.active=patch
scene.render.engine='CYCLES';scene.cycles.samples=1
scene.render.bake.margin=2
bpy.ops.object.bake(type='EMIT')
ground_image.filepath_raw=str(WEB/'ground.png');ground_image.file_format='PNG';ground_image.save()
bpy.data.objects.remove(patch,do_unlink=True);floor.node_tree.nodes.remove(bake_node)
scene.render.engine='BLENDER_EEVEE'

def extend_native(material, tile_name, vertical=False):
    """Match Rust's world-anchored outside-frustum fallback in Blender previews."""
    ns,links=material.node_tree.nodes,material.node_tree.links
    xyz=ns.new('ShaderNodeSeparateXYZ')
    links.new(ns['ProjectorCoordinates'].outputs['Object'],xyz.inputs[0])
    def scalar(op,a,b):
        node=ns.new('ShaderNodeMath');node.operation=op
        for i,value in enumerate((a,b)):
            if isinstance(value,(int,float)):node.inputs[i].default_value=value
            else:links.new(value,node.inputs[i])
        return node.outputs[0]
    uv=ns.new('ShaderNodeCombineXYZ')
    links.new(scalar('MULTIPLY',xyz.outputs['Z' if vertical else 'X'],.25),uv.inputs[0])
    second=scalar('ADD',xyz.outputs['Y'],CAMERA_H) if vertical else xyz.outputs['Z']
    links.new(scalar('MULTIPLY',second,.25),uv.inputs[1])
    image=bpy.data.images.load(str(WEB/(tile_name+'.png')))
    image.colorspace_settings.name='sRGB';image.pack()
    tile_node=ns.new('ShaderNodeTexImage');tile_node.image=image;tile_node.extension='REPEAT'
    links.new(uv.outputs[0],tile_node.inputs[0])
    source_uv=ns['ProjectionUV'];valid=scalar('LESS_THAN',xyz.outputs['Z'],0)
    for value in [source_uv.inputs['X'].links[0].from_socket,source_uv.inputs['Y'].links[0].from_socket]:
        for edge in [value,scalar('SUBTRACT',1,value)]:
            smooth=ns.new('ShaderNodeMapRange');smooth.interpolation_type='SMOOTHSTEP';smooth.clamp=True
            links.new(edge,smooth.inputs['Value']);smooth.inputs['From Min'].default_value=0;smooth.inputs['From Max'].default_value=.08
            valid=scalar('MULTIPLY',valid,smooth.outputs[0])
    emit=ns['Emission'];source=emit.inputs[0].links[0].from_socket
    blend=ns.new('ShaderNodeMixRGB')
    links.new(valid,blend.inputs[0]);links.new(tile_node.outputs['Color'],blend.inputs[1]);links.new(source,blend.inputs[2])
    links.new(blend.outputs[0],emit.inputs[0])

extend_native(floor,'ground')
extend_native(wall,TILE,True)

# Export the exact same world geometry: Blender +Y forward, +Z up -> Three -Z, +Y.
payload={
 'version':1,
 'calibration':{'fov':FOV,'aspect':ASPECT,'camHeight':CAMERA_H,'horizon':HORIZON},
 'textures':{'clean':f'../{MAP}-day/clean.webp','equipment':f'../{MAP}-day/hall.webp','floor':'ground.png','wall':TILE+'.png'},
 'textures2x':{'clean':f'../{MAP}-day/clean.png','equipment':f'../{MAP}-day/hall.png'},
 'repairPolygons':repair,'meshes':[],
 'reconstruction':{'id':f'{MAP}-day','sourceSize':[W,H],'metricCameraHeight':CAMERA_H,'factoryMeshes':False,'boundedLongitudinalSet':True,'freeCamera':False,'noDepthDisplacement':True},
}
for obj in objects:
    data=obj.data;data.calc_loop_triangles()
    for material_index,material in enumerate(data.materials):
        triangles=[tri for tri in data.loop_triangles if tri.material_index==material_index]
        if not triangles: continue
        positions,indices,remap=[],[],{}
        for tri in triangles:
            for index in tri.vertices:
                if index not in remap:
                    remap[index]=len(positions)//3
                    p=obj.matrix_world@data.vertices[index].co
                    positions.extend([round(p.x,6),round(p.z,6),round(-p.y,6)])
                indices.append(remap[index])
        kind=material.get('stageMaterial','side')
        part={'name':obj.name+(' · hidden faces' if material_index else ''),'material':kind,'positions':positions,'indices':indices,'occludesFigures':bool(obj['occludesFigures'])}
        if kind=='side': part['color']=list(material.diffuse_color[:3])
        payload['meshes'].append(part)

manifest=WEB/'manifest.json'
manifest.write_text(json.dumps(payload,ensure_ascii=False,separators=(',',':')),encoding='utf-8')
overview=bpy.data.scenes.new('02 · ГЕОМЕТРИЯ — собственные объёмы карты')
overview.collection.children.link(architecture);overview.collection.children.link(props)
overview.render.engine='BLENDER_WORKBENCH'
overview.render.resolution_x,overview.render.resolution_y=1400,900
overview.render.resolution_percentage=100
overview.world=scene.world
overview.display.shading.color_type='RANDOM'
overview.display.shading.show_shadows=True;overview.display.shading.show_cavity=True
overview_cam=bpy.data.objects.new('Geometry overview',bpy.data.cameras.new('Geometry overview'))
overview.collection.objects.link(overview_cam)
overview_cam.location=(40,-25,45)
overview_cam.rotation_euler=(Vector((0,35,2))-overview_cam.location).to_track_quat('-Z','Y').to_euler()
overview_cam.data.type='ORTHO';overview_cam.data.ortho_scale=90;overview.camera=overview_cam

report={
 'map':MAP,'horizon':HORIZON,
 'sourceSize':[W,H],'sourceSha256':{key:hashlib.sha256((SOURCE/(key+'.png')).read_bytes()).hexdigest() for key in ['hall','clean']},
 'meshes':len(payload['meshes']),'objects':len(objects),'triangles':sum(len(p['indices'])//3 for p in payload['meshes']),
 'repairPolygons':len(repair),'projectionFixed':True,'forwardPreviewMetres':2,'backwardPreviewMetres':1.5,
 'manifestSha256':hashlib.sha256(manifest.read_bytes()).hexdigest(),
 'limits':['single-view approximate reconstruction','closed volumes are authored, not scanned','fixed photographic lighting','intended for forward and backward battle travel, not orbiting'],
}
(OUT/'stage-check.json').write_text(json.dumps(report,indent=2),encoding='utf-8')
note=f'{TITLE}\n\nPROJECTOR неподвижен. VIEW: кадр 1 исходник, 49 вперёд 2 м, 145 назад 1.5 м.\nОбъекты, убранные с чистого кадра, имеют свою глубину.\nЭто приближённая реконструкция одного ракурса для продольных перемещений, не свободная 360-градусная локация.\nВ игре используется та же геометрия manifest.json.\n'
bpy.data.texts.new('ПРОЧИТАЙ · карта').write(note)
scene.frame_set(1);bpy.context.window.scene=scene
bpy.ops.wm.save_as_mainfile(filepath=str(OUT/f'CrownedDogs-{MAP}-Stage.blend'),compress=True)
print('KASKAD_STAGE_READY',json.dumps(report),flush=True)
if not args.no_render:
    for frame,name in [(1,'origin'),(49,'forward'),(145,'backward')]:
        scene.frame_set(frame);scene.render.filepath=str(OUT/f'{MAP}-{name}.png')
        bpy.ops.render.render(write_still=True,scene=scene.name)
    overview.render.filepath=str(OUT/f'{MAP}-geometry.png')
    bpy.ops.render.render(write_still=True,scene=overview.name)
    print('KASKAD_STAGE_RENDERS_READY',flush=True)
