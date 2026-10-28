"""Make the fitting copy easy to edit: independent objects and a sparse Bezier sling."""
import bpy, json, math, os, sys
from mathutils import Vector


def simplify(points, tolerance):
    if len(points) < 3:
        return list(range(len(points)))
    result = {0, len(points) - 1}
    def visit(first, last):
        direction = points[last] - points[first]
        length = direction.length_squared
        best, distance = None, tolerance
        for index in range(first + 1, last):
            t = max(0, min(1, (points[index] - points[first]).dot(direction) / length)) if length else 0
            deviation = (points[index] - (points[first] + direction * t)).length
            if deviation > distance:
                best, distance = index, deviation
        if best is not None:
            result.add(best); visit(first, best); visit(best, last)
    visit(0, len(points) - 1)
    return sorted(result)


def make_independent():
    scene = bpy.context.scene
    roles = {obj.get('crowned_dogs_role'): obj for obj in scene.objects if obj.get('crowned_dogs_role')}
    gun = roles['weapon_mount']
    clips = [roles['Sling_Clip_Stock'], roles['Sling_Clip_Foreend']]
    old = roles['sling']
    bpy.context.view_layer.update()
    # Freeze any evaluated end positions before detaching. Preserve all saved user transforms.
    states = [(m, m.show_viewport) for m in old.modifiers if m.type == 'SOLIDIFY']
    for modifier, _ in states:
        modifier.show_viewport = False
    bpy.context.view_layer.update()
    evaluated = old.evaluated_get(bpy.context.evaluated_depsgraph_get())
    vertices = [evaluated.matrix_world @ v.co for v in evaluated.data.vertices]
    centres = [(vertices[i] + vertices[i + 1]) / 2 for i in range(0, len(vertices), 2)]
    for modifier, visible in states:
        modifier.show_viewport = visible
    materials = list(old.data.materials)
    target_collection = old.users_collection[0]
    for obj in [gun, *clips]:
        world = obj.matrix_world.copy()
        obj.parent = None
        obj.matrix_world = world
        obj['Назначение'] = 'Независимый объект. G — переместить, R — повернуть.'

    # Bevel a rectangular profile along a small set of handles, not 114 ribbon vertices.
    curve = bpy.data.curves.new('Ремень — редактируемая траектория', 'CURVE')
    curve.dimensions = '3D'; curve.resolution_u = 16; curve.twist_mode = 'MINIMUM'
    spline = curve.splines.new('BEZIER')
    selected = simplify(centres, .018)
    spline.bezier_points.add(len(selected) - 1)
    for point, index in zip(spline.bezier_points, selected):
        point.co = centres[index]
        point.handle_left_type = point.handle_right_type = 'AUTO'
    profile_curve = bpy.data.curves.new('Профиль стропы 31 × 1.8 мм', 'CURVE')
    profile_curve.dimensions = '2D'
    profile = profile_curve.splines.new('POLY'); profile.points.add(3)
    for point, coordinates in zip(profile.points, [(-.0155, -.0009, 0, 1), (.0155, -.0009, 0, 1), (.0155, .0009, 0, 1), (-.0155, .0009, 0, 1)]):
        point.co = coordinates
    profile.use_cyclic_u = True
    profile_obj = bpy.data.objects.new('Профиль ремня · ширина', profile_curve)
    helper_collection = bpy.data.collections.new('04 · Профиль ремня — служебный')
    scene.collection.children.link(helper_collection); helper_collection.objects.link(profile_obj)
    profile_obj.hide_render = True; profile_obj.hide_select = True; profile_obj.hide_set(True)
    curve.bevel_mode = 'OBJECT'; curve.bevel_object = profile_obj; curve.use_fill_caps = True
    for material in materials:
        curve.materials.append(material)
    bpy.data.objects.remove(old, do_unlink=True)
    sling = bpy.data.objects.new('Ремень', curve); target_collection.objects.link(sling)
    sling['crowned_dogs_role'] = 'sling'
    sling['Назначение'] = 'Tab — редактирование; выбрать точку; G — переместить. Ctrl+T — наклон стропы.'
    scene['crowned_dogs_layout'] = 'free edit: gun, clips and Bezier sling are independent'

    notes = bpy.data.texts.get('НАЧНИ ЗДЕСЬ') or bpy.data.texts.new('НАЧНИ ЗДЕСЬ')
    notes.clear()
    notes.write('''Автомат, два карабина и ремень независимы.
Нет родительских связей, крюков или ограничителей между ними.

Автомат: выбрать объект → G для перемещения, R для вращения.
Карабин — приклад / Карабин — цевьё: редактируются таким же образом.
Ремень: выбрать → Tab → выбрать точку → G. Ctrl+T меняет наклон ленты.
Чтобы снова двигать весь ремень, выйти из редактирования клавишей Tab.
Можно скрыть любой из объектов глазом в списке справа.

Ctrl+S сохраняет эту отдельную копию. Исходный файл не заменён.
Персона и подставка служат ориентирами; текстуры упакованы.
''')
    bpy.ops.object.select_all(action='DESELECT')
    sling.select_set(True); bpy.context.view_layer.objects.active = sling
    bpy.context.view_layer.update()

    # Check the actual dependency graph: moving the gun must not move or deform the others.
    def snapshot(obj):
        evaluated = obj.evaluated_get(bpy.context.evaluated_depsgraph_get())
        data = evaluated.to_mesh()
        coordinates = [evaluated.matrix_world @ v.co for v in data.vertices]
        evaluated.to_mesh_clear()
        return coordinates
    originals = {obj.name: snapshot(obj) for obj in [*clips, sling]}
    original_location = gun.location.copy()
    gun.location.x += .2; bpy.context.view_layer.update()
    drifts = {obj.name: max(((a - b).length for a, b in zip(originals[obj.name], snapshot(obj))), default=0) for obj in [*clips, sling]}
    gun.location = original_location; bpy.context.view_layer.update()
    assert all(value < 1e-6 for value in drifts.values()), drifts
    assert all(obj.parent is None and len(obj.constraints) == 0 for obj in [gun, *clips, sling])
    assert len(sling.modifiers) == 0
    return {'controlPoints': len(selected), 'parents': {obj.name: None for obj in [gun, *clips, sling]}, 'gunMoveDrift': drifts}


if __name__ == '__main__':
    target = os.path.abspath(sys.argv[sys.argv.index('--') + 1])
    if os.path.exists(target):
        raise RuntimeError('Refusing to overwrite an existing editable file: ' + target)
    report = make_independent()
    scene = bpy.context.scene
    scene.render.filepath = os.path.join(os.path.dirname(target), 'fitting-free-edit-preview.png')
    bpy.ops.wm.save_as_mainfile(filepath=target)
    with open(os.path.join(os.path.dirname(target), 'free-edit-check.json'), 'w', encoding='utf-8') as stream:
        json.dump(report, stream, ensure_ascii=False, indent=2)
    bpy.ops.render.render(write_still=True)
    print('FREE_EDIT_READY', json.dumps(report, ensure_ascii=False))
