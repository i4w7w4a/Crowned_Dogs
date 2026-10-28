import bpy, json, sys, os, hashlib

folder = os.path.abspath(sys.argv[sys.argv.index('--') + 1])
records = {}
for filename in ['CrownedDogs-Fitting-Weapon.blend', 'CrownedDogs-Fitting-FreeEdit.blend']:
    bpy.ops.wm.open_mainfile(filepath=os.path.join(folder, filename))
    objects = []
    for obj in bpy.context.scene.objects:
        role = obj.get('crowned_dogs_role')
        if not role and obj.type == 'MESH' and any(c.name.startswith('01 · Персона') for c in obj.users_collection):
            role = 'persona:' + obj.name
        if not role: continue
        record = {'name': obj.name, 'role': role, 'type': obj.type, 'matrix': [list(row) for row in obj.matrix_world], 'parent': obj.parent.name if obj.parent else None, 'modifiers': [m.type for m in obj.modifiers]}
        if obj.type == 'MESH':
            vertices = [list(v.co) for v in obj.data.vertices]
            record.update(vertices=len(vertices), shapeHash=hashlib.sha256(json.dumps(vertices).encode()).hexdigest())
        if obj.type == 'CURVE':
            record['splines'] = [{'type': s.type, 'points': [{'co': list(p.co), 'left': list(p.handle_left), 'right': list(p.handle_right), 'tilt': p.tilt, 'radius': p.radius} for p in s.bezier_points]} for s in obj.data.splines]
        objects.append(record)
    records[filename] = objects
with open(os.path.join(folder, 'user-edit-inspection.json'), 'w', encoding='utf-8') as target:
    json.dump(records, target, ensure_ascii=False, indent=2)
print('EDIT_INSPECTION_READY')
