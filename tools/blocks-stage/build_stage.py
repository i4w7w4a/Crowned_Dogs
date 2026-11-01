"""Build the courtyard's own bounded 3D set, shared by Blender and the Rust renderer.

Blender 5.2 --background --factory-startup --python tools/blocks-stage/build_stage.py
The source camera is fixed. Only VIEW moves; no depth displacement or zoom is used.
"""
import argparse
import hashlib
import json
import math
import sys
from array import array
from pathlib import Path

import bpy
from bpy_extras.object_utils import world_to_camera_view
from mathutils import Vector

REPO = Path(__file__).resolve().parents[2]
parser = argparse.ArgumentParser()
parser.add_argument('--no-render', action='store_true')
parser.add_argument('--web-root', type=Path, default=REPO/'apps/web/public/img/blocks-stage')
parser.add_argument('--output-root', type=Path, default=REPO/'release-local/blocks-stage')
args = parser.parse_args(sys.argv[sys.argv.index('--')+1:] if '--' in sys.argv else [])
WEB, OUT = args.web_root, args.output_root
WEB.mkdir(parents=True, exist_ok=True)
OUT.mkdir(parents=True, exist_ok=True)
SOURCE = REPO/'apps/web/public/img/blocks-day'
W, H, CAMERA_H, FOV = 941, 1672, 2.0, 55.0
ASPECT = W/H
FOCAL = .5/math.tan(math.radians(FOV*.5))
HORIZON = .608 - CAMERA_H*.212/1.8

bpy.ops.wm.read_factory_settings(use_empty=True)
scene = bpy.context.scene
scene.name = '01 · ПЯТИЭТАЖКИ — боевой проезд'
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
scene.world = bpy.data.worlds.new('Сухой солнечный двор')
scene.world.use_nodes = True
scene.world.node_tree.nodes['Background'].inputs[0].default_value = (.28, .29, .30, 1)
scene.world.node_tree.nodes['Background'].inputs[1].default_value = .6

def collection(name):
    value = bpy.data.collections.new(name)
    scene.collection.children.link(value)
    return value

architecture = collection('01 · Своя архитектура двора')
props = collection('02 · Объёмные объекты двора')
camera_collection = collection('00 · Камеры')

def camera(name):
    data = bpy.data.cameras.new(name)
    data.sensor_fit, data.sensor_height = 'VERTICAL', 36
    data.lens = 36*FOCAL
    data.shift_y = HORIZON-.5
    data.clip_start, data.clip_end = .05, 180
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

photo = projected('Фото · автобус и площадка', images['hall'], 'equipment')
clean = projected('Чистая архитектура', images['clean'], 'clean')
floor = projected('Земля · фотопроекция', images['clean'], 'floor')
wall = projected('Фасады · фотопроекция', images['clean'], 'wall')
concrete = emission('Скрытый бетон', (.25, .235, .21))
steel = emission('Ржавая обратная сторона металла', (.16, .12, .085))
rubber = emission('Боковины шин', (.035, .032, .028))
dark = emission('Тёмная внутренняя поверхность', (.07, .065, .055))

repair = []
objects = []
def ray(u, v, depth):
    return Vector(((u-.5)*ASPECT/FOCAL*depth, depth, CAMERA_H+(HORIZON-v)/FOCAL*depth))

def floor_depth(v):
    return CAMERA_H*FOCAL/max(v-HORIZON, .015)

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

def slab(name, outline, depths, thickness, back_material=concrete, front_material=photo, direction=None, target=props, repair_plate=True):
    n = len(outline)
    depths = [depths]*n if isinstance(depths, (float, int)) else depths
    front = [ray(u, v, d) for (u, v), d in zip(outline, depths)]
    shift = Vector(direction) if direction else Vector((0, thickness, 0))
    back = [p+shift for p in front]
    faces = [tuple(reversed(range(n))), tuple(range(n, n*2))]
    faces += [(i, (i+1)%n, (i+1)%n+n, i+n) for i in range(n)]
    obj = mesh(name, front+back, faces, [front_material, back_material], [0]+[1]*(len(faces)-1), target)
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
    # Only genuinely camera-facing faces sample the source photo. Hidden sides
    # cannot accidentally acquire a strip of sky or another photographed prop.
    mids = [(vertices[f[0]]+vertices[f[1]]+vertices[f[2]])/3 for f in faces]
    indices = []
    for face, mid in zip(faces, mids):
        normal = (vertices[face[1]]-vertices[face[0]]).cross(vertices[face[2]]-vertices[face[0]])
        indices.append(0 if normal.dot(Vector((0, 0, CAMERA_H))-mid) > 0 else 1)
    return mesh(name, vertices, faces, [material, back_material], indices)

def photo_pipe(name, start, end, da, db, radius=.025):
    a, b = ray(*start, da), ray(*end, db)
    tube(name, a, b, radius)
    # Thin source-space repair follows the actual strut, preserving nearby trees.
    dx, dy = end[0]-start[0], end[1]-start[1]
    length = max(math.hypot(dx, dy), 1e-5)
    width = .0032
    ox, oy = -dy/length*width, dx/length*width
    repair.append([[max(0,min(1,u)),max(0,min(1,v))] for u,v in [(start[0]+ox,start[1]+oy),(end[0]+ox,end[1]+oy),(end[0]-ox,end[1]-oy),(start[0]-ox,start[1]-oy)]])

# Full ground and independent converging residential volumes. There is no roof
# spanning the open yard, and none of the factory reconstruction is imported.
mesh('Courtyard ground · y=0 in game', [(-35,-12,0),(35,-12,0),(35,70,0),(-35,70,0)], [(0,1,2,3)], [floor], target=architecture, occludes=False)
left_floor = [.548, .463]
left_depth = [floor_depth(v) for v in left_floor]
slab('Left apartment building · closed mass', [(-.18,-.095),(.462,.202),(.462,left_floor[1]),(-.18,left_floor[0])], [left_depth[0],left_depth[1],left_depth[1],left_depth[0]], 2.2, direction=(-2.2,0,0), front_material=wall, target=architecture, repair_plate=False)
right_floor = [.463, .535]
right_depth = [floor_depth(v) for v in right_floor]
slab('Right apartment building · closed mass', [(.462,.202),(1.18,-.005),(1.18,right_floor[1]),(.462,right_floor[0])], [right_depth[0],right_depth[1],right_depth[1],right_depth[0]], 2.2, direction=(2.2,0,0), front_material=wall, target=architecture, repair_plate=False)
mesh('Far sky · fixed distant surface', [(-130,110,-20),(130,110,-20),(130,110,150),(-130,110,150)], [(0,1,2,3)], [photo], target=architecture, occludes=False)

# The bus is a finite skewed hull, not a billboard. Its right-hand end is farther
# away than its cropped front. Wheels, roof and door steps have their own depth.
bus_outline = [(0,.334),(.035,.336),(.171,.359),(.305,.380),(.425,.397),(.448,.410),(.451,.491),(.406,.505),(.318,.509),(.211,.516),(.105,.510),(0,.487)]
bus_depth = lambda u: 12.8 + max(0,min(1,u/.45))*2.6
slab('Bus · closed damaged body', bus_outline, [bus_depth(u) for u,v in bus_outline], 2.2, direction=(-1.25,1.82,0), back_material=steel)
# Actual roof cross-sections, falling with the crushed end in the source picture.
roof_vertices, roof_faces = [], []
for u,v in [(0,.334),(.035,.336),(.171,.359),(.305,.380),(.425,.397),(.448,.410)]:
    edge = ray(u,v,bus_depth(u))
    for j in range(9):
        t = j/8
        roof_vertices.append(edge+Vector((-1.25*t,1.82*t,-.14+.14*math.sin(t*math.pi))))
for section in range(5):
    for j in range(8):
        a=section*9+j
        roof_faces.append((a,a+1,a+10,a+9))
mesh('Bus · curved roof with real width',roof_vertices,roof_faces,[photo])
for index,(u,v,r) in enumerate([(.243,.495,.32),(.374,.497,.28)]):
    center=ray(u,v,bus_depth(u));center.z=max(r,center.z)
    tube(f'Bus wheel {index+1} · finite cylinder',center+Vector((.025,.008,0)),center+Vector((-.20,-.07,0)),r,20,photo,rubber)
for index,(u,v) in enumerate([(.263,.504),(.277,.509),(.291,.514)]):
    p=ray(u,v,bus_depth(u))
    tube(f'Bus door step {index+1}',p+Vector((-.20,-.03,0)),p+Vector((.20,.03,0)),.035,6,photo,steel)

# Rocket: upright curved shell, conical nose, real legs and a finite slide tray.
# This object is in front of the right apartment facade. Its photograph alone
# cannot determine depth; use the nearer solution so it cannot sink into a wall.
rocket_d=12.6
rocket_origin=ray(.918,.505,rocket_d)
rocket_radius=.31
bottom=ray(.918,.447,rocket_d).z
top=ray(.918,.335,rocket_d).z
rocket_center=Vector((rocket_origin.x,rocket_origin.y,0))
tube('Playground rocket · cylindrical hull',rocket_center+Vector((0,0,bottom)),rocket_center+Vector((0,0,top)),rocket_radius,24,photo,steel)
tip=rocket_center+Vector((0,0,ray(.918,.293,rocket_d).z))
cone_ring=[rocket_center+Vector((rocket_radius*1.18*math.cos(i*math.tau/24),rocket_radius*1.18*math.sin(i*math.tau/24),top)) for i in range(24)]
mesh('Playground rocket · conical nose',cone_ring+[tip],[(i,(i+1)%24,24) for i in range(24)]+[tuple(reversed(range(24)))],[photo,steel],[0]*24+[1])
repair.append([[.877,.292],[.956,.292],[.983,.451],[.973,.505],[.874,.505]])
for i in range(4):
    angle=i*math.tau/4+.3
    a=rocket_center+Vector((math.cos(angle)*.25,math.sin(angle)*.25,bottom+.05))
    b=rocket_center+Vector((math.cos(angle)*.46,math.sin(angle)*.46,.03))
    tube(f'Rocket support {i+1}',a,b,.022,8)
slide_top=ray(.867,.412,rocket_d)
slide_end=ray(.765,.505,floor_depth(.505));slide_end.z=.08
across=Vector((.31,.10,0))
slide_points=[slide_top-across/2,slide_top+across/2,slide_end+across/2,slide_end-across/2]
mesh('Playground slide · real inclined tray',slide_points+ [p-Vector((0,0,.035)) for p in slide_points],[(0,1,2,3),(4,7,6,5),(0,4,5,1),(1,5,6,2),(2,6,7,3),(3,7,4,0)],[photo,steel],[0,0,1,1,1,1])
for side in [-1,1]:
    tube(f'Slide raised rail {side}',slide_top+across*side/2+Vector((0,0,.08)),slide_end+across*side/2+Vector((0,0,.08)),.022,8)
repair.append([[.858,.406],[.887,.423],[.795,.516],[.750,.514]])

# The original swing frame and ladder use individual tubes with actual gaps.
pipe_data=[
 ('Swing beam',(.742,.398),(.885,.386),16.2,16.8),
 ('Swing left leg',(.742,.398),(.723,.508),16.2,floor_depth(.508)),
 ('Swing middle leg',(.810,.393),(.798,.507),16.5,floor_depth(.507)),
 ('Swing rear diagonal',(.742,.398),(.790,.508),16.2,floor_depth(.508)),
 ('Swing hanging chain left',(.767,.397),(.767,.488),16.2,16.2),
 ('Swing hanging chain right',(.787,.395),(.787,.488),16.3,16.3),
 ('Swing seat',(.761,.489),(.797,.489),16.2,16.3),
 ('Back railing beam',(.613,.443),(.756,.438),18.2,17.0),
 ('Back railing left',(.613,.443),(.605,.490),18.2,floor_depth(.490)),
 ('Back railing right',(.756,.438),(.761,.496),17.0,floor_depth(.496)),
 ('Rocket ladder left',(.926,.443),(.928,.509),rocket_d,rocket_d),
 ('Rocket ladder right',(.945,.444),(.963,.512),rocket_d,rocket_d),
]
for name,a,b,da,db in pipe_data: photo_pipe(name,a,b,da,db,.018 if 'chain' in name else .025)
for i in range(1,6):
    t=i/6
    a=(.926+.002*t,.443+.066*t);b=(.945+.018*t,.444+.068*t)
    photo_pipe(f'Rocket ladder rung {i}',a,b,rocket_d,rocket_d,.018)

# Independent concrete entrance walls. Their hidden backs never sample the bus.
slab('Entrance left · finite concrete wall',[(0,.502),(.134,.514),(.177,.545),(.171,.635),(.114,.648),(0,.630)],[7.7,7.4,7.2,7.2,7.2,7.7],.65,direction=(-.55,.35,0))
slab('Entrance right · finite concrete wall',[(.859,.518),(1,.515),(1,.635),(.886,.630),(.843,.603)],[7.8,8.0,8.0,7.8,7.8],.65,direction=(.55,.35,0))

def noise(x,y):
    # Stable periodic mineral grain, created for this yard rather than copied
    # from the salt factory or from a repeated full-scene photograph.
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

tile('concrete',(.31,.30,.27))

# Bake an orthorectified local patch of actual courtyard ground. This is a
# material-space texture bake, not a repeated full-frame photograph or an
# enlarged bottom-row smear. The unseen return path uses the same rubble scale.
patch_data=bpy.data.meshes.new('TEMP · ground material bake')
patch_data.from_pydata([(-1.3,5.5,0),(1.3,5.5,0),(1.3,8.1,0),(-1.3,8.1,0)],[],[(0,1,2,3)])
patch_data.update()
patch=bpy.data.objects.new('TEMP · ground material bake',patch_data)
scene.collection.objects.link(patch)
patch_data.materials.append(floor)
uv=patch_data.uv_layers.new(name='Ground material UV')
for loop,co in zip(uv.data,[(0,1),(1,1),(1,0),(0,0)]):loop.uv=co
ground_image=bpy.data.images.new('Courtyard ground · orthorectified material',width=512,height=512,alpha=False)
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
extend_native(wall,'concrete',True)

# Export the exact same world geometry: Blender +Y forward, +Z up -> Three -Z, +Y.
payload={
 'version':1,
 'calibration':{'fov':FOV,'aspect':ASPECT,'camHeight':CAMERA_H,'horizon':HORIZON},
 'textures':{'clean':'../blocks-day/clean.webp','equipment':'../blocks-day/hall.webp','floor':'ground.png','wall':'concrete.png'},
 'textures2x':{'clean':'../blocks-day/clean.png','equipment':'../blocks-day/hall.png'},
 'repairPolygons':repair,'meshes':[],
 'reconstruction':{'id':'blocks-day','sourceSize':[W,H],'metricCameraHeight':CAMERA_H,'factoryMeshes':False,'closedBusHull':True,'rocketAndSwingTubes':True,'boundedLongitudinalSet':True,'freeCamera':False,'noDepthDisplacement':True},
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
overview=bpy.data.scenes.new('02 · ГЕОМЕТРИЯ — собственные объёмы двора')
overview.collection.children.link(architecture);overview.collection.children.link(props)
overview.render.engine='BLENDER_WORKBENCH'
overview.render.resolution_x,overview.render.resolution_y=1400,900
overview.render.resolution_percentage=100
overview.world=scene.world
overview.display.shading.color_type='RANDOM'
overview.display.shading.show_shadows=True;overview.display.shading.show_cavity=True
overview_cam=bpy.data.objects.new('Geometry overview',bpy.data.cameras.new('Geometry overview'))
overview.collection.objects.link(overview_cam)
overview_cam.location=(15,-11,17)
overview_cam.rotation_euler=(Vector((0,12,1.7))-overview_cam.location).to_track_quat('-Z','Y').to_euler()
overview_cam.data.type='ORTHO';overview_cam.data.ortho_scale=32;overview.camera=overview_cam

report={
 'sourceSize':[W,H],'sourceSha256':{key:hashlib.sha256((SOURCE/(key+'.png')).read_bytes()).hexdigest() for key in ['hall','clean']},
 'meshes':len(payload['meshes']),'objects':len(objects),'triangles':sum(len(p['indices'])//3 for p in payload['meshes']),
 'repairPolygons':len(repair),'projectionFixed':True,'forwardPreviewMetres':2,'backwardPreviewMetres':1.5,
 'manifestSha256':hashlib.sha256(manifest.read_bytes()).hexdigest(),
 'limits':['single-view approximate reconstruction','closed volumes are authored, not scanned','fixed photographic lighting','intended for forward and backward battle travel, not orbiting'],
}
(OUT/'stage-check.json').write_text(json.dumps(report,indent=2),encoding='utf-8')
note='ПЯТИЭТАЖКИ · собственная геометрия двора\n\nPROJECTOR неподвижен. VIEW: кадр 1 исходник, 49 вперёд 2 м, 145 назад 1.5 м.\nФабричных мешей нет. Автобус, ракета, площадка и входные стены имеют глубину.\nЭто приближённая реконструкция одного ракурса для продольных перемещений, не свободная 360-градусная локация.\nИсточник фактуры и света — утверждённая картинка; невидимые поверхности дописаны простыми материалами.\nВ игре используется та же геометрия manifest.json.\n'
bpy.data.texts.new('ПРОЧИТАЙ · двор').write(note)
scene.frame_set(1);bpy.context.window.scene=scene
bpy.ops.wm.save_as_mainfile(filepath=str(OUT/'CrownedDogs-Blocks-Stage.blend'),compress=True)
print('BLOCKS_STAGE_READY',json.dumps(report),flush=True)
if not args.no_render:
    for frame,name in [(1,'blocks-origin'),(49,'blocks-forward'),(145,'blocks-backward')]:
        scene.frame_set(frame);scene.render.filepath=str(OUT/(name+'.png'))
        bpy.ops.render.render(write_still=True,scene=scene.name)
    overview.render.filepath=str(OUT/'blocks-geometry.png')
    bpy.ops.render.render(write_still=True,scene=overview.name)
    print('BLOCKS_STAGE_RENDERS_READY',flush=True)
