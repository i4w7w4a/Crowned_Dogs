"""Export only the user's saved Persona, in its exact world-space pose; never save the source blend."""
import bpy, json, os, sys, hashlib

output = os.path.abspath(sys.argv[sys.argv.index('--') + 1])
source = bpy.data.filepath
collection = next(c for c in bpy.data.collections if c.name.startswith('01 · Персона'))
parts = [o for o in collection.all_objects if o.type == 'MESH']
assert parts, 'No Persona meshes in the edited collection'
bpy.ops.object.select_all(action='DESELECT')
report = {'sourceSha256': hashlib.sha256(open(source, 'rb').read()).hexdigest(), 'parts': []}
for obj in parts:
    obj.hide_select = False
    obj.hide_set(False)
    obj.select_set(True)
    report['parts'].append({'name': obj.name, 'vertices': len(obj.data.vertices),
        'matrix': [list(row) for row in obj.matrix_world],
        'shapeHash': hashlib.sha256(json.dumps([list(v.co) for v in obj.data.vertices]).encode()).hexdigest()})
bpy.context.view_layer.objects.active = parts[0]
bpy.ops.export_scene.gltf(filepath=output, export_format='GLB', use_selection=True,
    export_yup=True, export_apply=True, export_animations=False, export_cameras=False,
    export_lights=False, export_image_format='AUTO', export_extras=True)
with open(output + '.report.json', 'w', encoding='utf-8') as target:
    json.dump(report, target, ensure_ascii=False, indent=2)
print('PERSONA_EXPORTED', json.dumps(report, ensure_ascii=False))
