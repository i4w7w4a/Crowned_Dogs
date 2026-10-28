"""Blender 5.x: independent weapon and clips, plus an editable Bezier sling."""
import bpy, json, math, os, sys
from mathutils import Matrix, Vector, Quaternion

folder = os.path.abspath(sys.argv[sys.argv.index('--') + 1])
with open(os.path.join(folder, 'fitting-layout.json'), encoding='utf-8') as source:
    layout = json.load(source)
bpy.ops.wm.read_factory_settings(use_empty=True)
scene = bpy.context.scene
scene.unit_settings.system = 'METRIC'
scene.unit_settings.length_unit = 'METERS'
C = Matrix.Rotation(math.pi / 2, 4, 'X')

for source in layout['sources']:
    bpy.ops.import_scene.gltf(filepath=os.path.join(folder, source))
for obj in list(bpy.data.objects):
    bpy.data.objects.remove(obj, do_unlink=True)

def collection(name):
    result = bpy.data.collections.new(name)
    scene.collection.children.link(result)
    return result

body_collection = collection('01 · Персона — поза из игры')
equipment_collection = collection('02 · Автомат и крепления — редактировать')
stage_collection = collection('03 · Подставка и свет')

def material(name, color, roughness, metallic=0):
    result = bpy.data.materials.new(name)
    result.use_nodes = True
    node = result.node_tree.nodes.get('Principled BSDF')
    node.inputs['Base Color'].default_value = (*color, 1)
    node.inputs['Roughness'].default_value = roughness
    node.inputs['Metallic'].default_value = metallic
    result.diffuse_color = (*color, 1)
    return result

webbing = material('Стропа — ткань', (.026, .032, .023), .94)
clip_metal = material('Карабины — сталь', (.13, .145, .15), .48, .8)
steel = material('Постамент — ориентир', (.055, .06, .065), .82, .4)

def mesh(record, group, fallback=None):
    values = record['positions']
    vertices = [(values[i], -values[i + 2], values[i + 1]) for i in range(0, len(values), 3)]
    indices = record['indices']
    faces = [indices[i:i + 3] for i in range(0, len(indices), 3)]
    data = bpy.data.meshes.new(record['name'])
    data.from_pydata(vertices, [], faces); data.update()
    uv = record['uv']
    if uv:
        layer = data.uv_layers.new(name='UVMap')
        for loop in data.loops:
            i = loop.vertex_index * 2
            # glTF/three sample V from the image's top; Blender's imported material uses bottom-origin UVs.
            layer.data[loop.index].uv = (uv[i], 1 - uv[i + 1])
    for polygon in data.polygons:
        polygon.use_smooth = True
    obj = bpy.data.objects.new(record['name'], data); group.objects.link(obj)
    mat = bpy.data.materials.get(record['material']) if record['material'] else None
    if mat or fallback:
        data.materials.append(mat or fallback)
    return obj

body_names = {'Helmet': 'Каска К-6', 'MaleBody': 'Корпус Персоны', 'MaleHead': 'Голова Персоны', 'Suit': 'Костюм', 'Vest': 'Бронежилет'}
for record in layout['persona']:
    obj = mesh(record, body_collection)
    obj.name = body_names.get(record['name'], record['name'])
    obj.hide_select = True

def join(objects, name):
    bpy.ops.object.select_all(action='DESELECT')
    for obj in objects:
        obj.select_set(True)
    bpy.context.view_layer.objects.active = objects[0]
    if len(objects) > 1:
        bpy.ops.object.join()
    obj = bpy.context.object; obj.name = name
    return obj

def game_matrix(position, rotation):
    q = Quaternion((rotation[3], rotation[0], rotation[1], rotation[2]))
    return C @ (Matrix.Translation(Vector(position)) @ q.to_matrix().to_4x4()) @ C.inverted()

gun = join([mesh(record, equipment_collection) for record in layout['gun']], 'Автомат')
gun.matrix_world = game_matrix(layout['mount']['position'], layout['mount']['rotation'])
gun['crowned_dogs_role'] = 'weapon_mount'
gun['Назначение'] = 'Двигать и вращать целиком. G — переместить, R — повернуть.'
gun.rotation_mode = 'XYZ'

sling_record = next(r for r in layout['sling'] if r['name'] == 'Two_Point_Woven_Sling')
sling = mesh(sling_record, equipment_collection, webbing); sling.name = 'Ремень'
sling['crowned_dogs_role'] = 'sling'
sling['Назначение'] = 'Концы следуют за карабинами; середина остаётся на плече.'
solidify = sling.modifiers.new('Толщина стропы', 'SOLIDIFY'); solidify.thickness = .0018

clips = []
for info, name in zip(layout['clips'], ['Карабин — приклад', 'Карабин — цевьё']):
    obj = join([mesh(r, equipment_collection, clip_metal) for r in layout['sling'] if r['name'].startswith(info['name'])], name)
    transform = game_matrix(info['position'], info['rotation'])
    obj.data.transform(transform.inverted()); obj.matrix_world = transform
    obj['crowned_dogs_role'] = info['name']
    obj['Назначение'] = 'Независимое крепление; можно свободно перемещать и вращать.'
    obj.rotation_mode = 'XYZ'
    clips.append(obj)

def move_to(obj, group):
    for old in list(obj.users_collection): old.objects.unlink(obj)
    group.objects.link(obj)

bpy.ops.mesh.primitive_cylinder_add(vertices=96, radius=.79, depth=.145, location=(0, 0, .0725))
platform = bpy.context.object; platform.name = 'Постамент — ориентир'; move_to(platform, stage_collection)
platform.data.materials.append(steel); platform.hide_select = True
bevel = platform.modifiers.new('Кромка', 'BEVEL'); bevel.width = .012; bevel.segments = 3
bpy.ops.mesh.primitive_plane_add(size=200)
floor = bpy.context.object; floor.name = 'Пол'; move_to(floor, stage_collection)
floor.data.materials.append(material('Пол — нейтральный', (.06, .066, .073), .94)); floor.hide_select = True

def track(obj, target):
    obj.rotation_euler = (Vector(target) - obj.location).to_track_quat('-Z', 'Y').to_euler()

for name, location, power, size, color in [
    ('Основной свет', (-2.4, -2.7, 4.2), 650, 3.0, (.84, .9, 1)),
    ('Заполнение', (2.4, -2.1, 1.9), 190, 2.5, (.78, .84, .9)),
    ('Контровой', (1.4, 1.8, 2.7), 700, 2, (.88, .93, 1)),
]:
    light_data = bpy.data.lights.new(name, 'AREA'); light_data.energy = power; light_data.shape = 'DISK'; light_data.size = size; light_data.color = color
    light = bpy.data.objects.new(name, light_data); stage_collection.objects.link(light); light.location = location; track(light, (0, 0, 1)); light.hide_select = True
camera_data = bpy.data.cameras.new('Камера'); camera = bpy.data.objects.new('Камера', camera_data); stage_collection.objects.link(camera)
camera.location = (1.25, -4.2, 1.7); track(camera, (0, 0, 1.05)); camera_data.lens = 55; camera.hide_select = True; scene.camera = camera
world = bpy.data.worlds.new('Мастерская'); world.use_nodes = True; world.node_tree.nodes['Background'].inputs[0].default_value = (.12, .14, .17, 1); world.node_tree.nodes['Background'].inputs[1].default_value = .3; scene.world = world
scene.render.engine = 'CYCLES'; scene.cycles.samples = 24; scene.cycles.use_denoising = True
scene.render.resolution_x = 800; scene.render.resolution_y = 1000; scene.render.resolution_percentage = 100
scene.render.image_settings.file_format = 'PNG'; scene.render.filepath = os.path.join(folder, 'fitting-weapon-preview.png')
scene.view_settings.view_transform = 'AgX'

notes = bpy.data.texts.new('НАЧНИ ЗДЕСЬ')
notes.write('''Это копия текущей примерочной для подгонки подвеса.

Автомат — весь Варяг одним объектом. Он уже выделен.
Карабин — приклад / Карабин — цевьё — отдельные объекты справа в списке.
Ремень — середина на плече, концы следуют за карабинами.
Карабины следуют за автоматом, но их можно двигать отдельно.
Персона — точная текущая поза из игры, меши зафиксированы от случайного выбора.

G — переместить. R — повернуть. X / Y / Z после G или R — выбрать ось.
F3 → Frame Selected (Показать выделенное) — приблизить выбранный объект без numpad.
Ctrl+S — сохранить. После правок пришли этот .blend: я перенесу расположение в игру.

Оси Blender: X вправо, Y от камеры, Z вверх. Размеры в метрах.
Поза в этой рабочей копии запечена; исходный игровой скелет не менялся.
Подставка здесь служит ориентиром. Все текстуры упакованы в .blend.
''')
scene['crowned_dogs_layout'] = 'weapon fit from the live fitting scene; editable mount and sling clips'
bpy.ops.object.select_all(action='DESELECT'); gun.select_set(True); bpy.context.view_layer.objects.active = gun
for screen in bpy.data.screens:
    for area in screen.areas:
        if area.type == 'VIEW_3D':
            space = area.spaces.active
            space.shading.type = 'MATERIAL'
            space.overlay.show_floor = False; space.overlay.show_axis_x = False; space.overlay.show_axis_y = False
            space.region_3d.view_rotation = camera.rotation_euler.to_quaternion()
            space.region_3d.view_distance = 3.1
            space.region_3d.view_location = Vector((0, 0, 1.0))
            space.region_3d.view_perspective = 'PERSP'
            space.show_gizmo = True; space.show_gizmo_object_translate = True
for image in bpy.data.images:
    if image.source == 'FILE' and not image.packed_file:
        image.pack()
sys.path.insert(0, os.path.dirname(__file__))
from free_edit import make_independent
editing = make_independent()
sling = bpy.data.objects['Ремень']
target = os.path.join(folder, 'CrownedDogs-Fitting-FreeEdit.blend')
if os.path.exists(target):
    raise RuntimeError('Refusing to overwrite an existing editable file: ' + target)
bpy.ops.wm.save_as_mainfile(filepath=target)
report = {
    'blend': bpy.data.filepath, 'objects': len(scene.objects),
    'packedImages': sum(bool(i.packed_file) for i in bpy.data.images),
    'editable': [gun.name, *(o.name for o in clips), sling.name],
    'weaponLocation': list(gun.location), 'editing': editing,
}
with open(os.path.join(folder, 'scene-check.json'), 'w', encoding='utf-8') as target: json.dump(report, target, ensure_ascii=False, indent=2)
bpy.ops.render.render(write_still=True)
print('FITTING_BLEND_READY', json.dumps(report, ensure_ascii=False))
