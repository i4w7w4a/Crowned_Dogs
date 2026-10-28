"""Export the user's saved transforms and evaluated accessory surfaces without resaving the blend."""
import bpy, json, math, sys, os, hashlib
from mathutils import Matrix

output = os.path.abspath(sys.argv[sys.argv.index('--') + 1])
C = Matrix.Rotation(-math.pi / 2, 4, 'X')
equipment = Matrix.Translation((0, -.145, 0))
objects = {obj.get('crowned_dogs_role'): obj for obj in bpy.context.scene.objects if obj.get('crowned_dogs_role')}
gun_matrix = equipment @ C @ objects['weapon_mount'].matrix_world @ C.inverted()
data = {'sourceSha256': hashlib.sha256(open(bpy.data.filepath, 'rb').read()).hexdigest(), 'weaponMatrix': [gun_matrix[r][c] for c in range(4) for r in range(4)], 'accessories': []}
for role in ['sling', 'Sling_Clip_Stock', 'Sling_Clip_Foreend']:
    obj = objects[role].evaluated_get(bpy.context.evaluated_depsgraph_get())
    mesh = obj.to_mesh(); mesh.calc_loop_triangles()
    transform = equipment @ C @ obj.matrix_world
    normal_transform = transform.to_3x3().inverted().transposed()
    positions, normals, indices, unique = [], [], [], {}
    for triangle in mesh.loop_triangles:
        for loop_index in triangle.loops:
            point = transform @ mesh.vertices[mesh.loops[loop_index].vertex_index].co
            normal = (normal_transform @ mesh.corner_normals[loop_index].vector).normalized()
            key = tuple(round(value, 6) for value in (*point, *normal))
            if key not in unique:
                unique[key] = len(positions) // 3
                positions.extend(key[:3]); normals.extend(key[3:])
            indices.append(unique[key])
    material = mesh.materials[0]
    shader = material.node_tree.nodes.get('Principled BSDF')
    data['accessories'].append({'name': role, 'positions': positions, 'normals': normals, 'indices': indices, 'color': list(shader.inputs['Base Color'].default_value)[:3], 'roughness': shader.inputs['Roughness'].default_value, 'metalness': shader.inputs['Metallic'].default_value})
    obj.to_mesh_clear()
with open(output, 'w', encoding='utf-8') as target:
    json.dump(data, target, separators=(',', ':'))
print('USER_FIT_EXPORTED', json.dumps({'weaponMatrix': data['weaponMatrix'], 'accessories': [{'name': a['name'], 'triangles': len(a['indices']) // 3} for a in data['accessories']], 'sourceSha256': data['sourceSha256']}))
