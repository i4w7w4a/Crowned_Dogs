"""Render finished beards on the companion-cleaned heads; never rewrite the master/GLBs.

Blender --background --python-exit-code 1 --python tools/beards/preview_clean.py
Optional script arguments: -- --faces m1 m4, or -- --skip-render.
"""
import argparse
import hashlib
import json
import sys
from pathlib import Path

import bpy
from mathutils import Vector
from mathutils.kdtree import KDTree

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'output/beards'
MASTER = OUT / 'CrownedDogs-Beards-Master.blend'
SKIN = ROOT / 'apps/web/models/beards-skin.glb'
PREVIEW = OUT / 'CrownedDogs-Beards-Preview.blend'
RENDERS = OUT / 'final-renders'
parser = argparse.ArgumentParser()
parser.add_argument('--faces', nargs='+', choices=['m1', 'm2', 'm3', 'm4'], default=['m1', 'm2', 'm3', 'm4'])
parser.add_argument('--skip-render', action='store_true')
args = parser.parse_args(sys.argv[sys.argv.index('--') + 1:] if '--' in sys.argv else [])

def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

protected = [MASTER, ROOT / 'apps/web/packs/beards-male.glb', ROOT / 'apps/web/packs/beards-authored.glb', ROOT / 'apps/web/models/persona-male.glb']
before = {str(p.relative_to(ROOT)): digest(p) for p in protected}
report = {'sourceSha256': {**before, str(SKIN.relative_to(ROOT)): digest(SKIN)}, 'heads': {}, 'renders': []}
bpy.ops.wm.open_mainfile(filepath=str(MASTER))
original_objects = set(bpy.data.objects)
bpy.ops.import_scene.gltf(filepath=str(SKIN))
imported = set(bpy.data.objects) - original_objects

def key(p):
    return tuple(round(float(x), 6) for x in p)

def triangle_key(mesh, poly):
    return tuple(sorted(key(mesh.vertices[i].co) for i in poly.vertices))

for face in ('m2', 'm4'):
    head = bpy.data.objects['Head_' + face]
    carrier = bpy.data.objects['CleanNormals_' + face]
    material_carrier = bpy.data.objects['CleanHead_' + face]
    source = carrier.data
    patch = source.attributes.get('_CLEAN_POSITION')
    if patch is None:
        raise RuntimeError(f'{face}: missing _CLEAN_POSITION; found {[a.name for a in source.attributes]}')
    tree = KDTree(len(source.vertices))
    for vertex in source.vertices:
        tree.insert(vertex.co, vertex.index)
    tree.balance()
    positions = []
    max_match = 0.0
    max_delta = 0.0
    for vertex in head.data.vertices:
        _, index, distance = tree.find(vertex.co)
        if distance > 0.000002:
            raise RuntimeError(f'{face}: head/companion position mismatch {distance:.9f} m')
        # Standard POSITION/NORMAL are converted to Z-up by the importer.
        # Custom vector attributes intentionally retain explicit glTF Y-up values.
        p = patch.data[index].vector
        target = Vector((p.x, -p.z, p.y))
        max_match = max(max_match, distance)
        max_delta = max(max_delta, (target - vertex.co).length)
        positions.append(target)
    if max_delta > .002001:
        raise RuntimeError(f'{face}: companion exceeds 2 mm displacement: {max_delta}')

    # Match complete triangles, preserving split corner normals at UV seams.
    # Nearest-vertex-only normal transfer would conflate different patch corners.
    triangles = {}
    centre_tree = KDTree(len(source.polygons))
    for poly in source.polygons:
        triangles.setdefault(triangle_key(source, poly), []).append(poly)
        centre = sum((source.vertices[i].co for i in poly.vertices), Vector()) / len(poly.vertices)
        centre_tree.insert(centre, poly.index)
    centre_tree.balance()
    normals = [None] * len(head.data.loops)
    max_corner_match = 0.0
    for poly in head.data.polygons:
        candidates = triangles.get(triangle_key(head.data, poly))
        if not candidates:
            centre = sum((head.data.vertices[i].co for i in poly.vertices), Vector()) / len(poly.vertices)
            candidates = [source.polygons[index] for _, index, distance in centre_tree.find_n(centre, 8) if distance < .000002]
        if not candidates:
            raise RuntimeError(f'{face}: missing original triangle {poly.index}; companion must match source geometry')
        def corner_error(candidate):
            return max(min((head.data.vertices[i].co - source.vertices[j].co).length_squared for j in candidate.vertices) for i in poly.vertices)
        match = min(candidates, key=corner_error)
        for loop_index in poly.loop_indices:
            vertex = head.data.vertices[head.data.loops[loop_index].vertex_index]
            source_loop = min(match.loop_indices, key=lambda i: (source.vertices[source.loops[i].vertex_index].co - vertex.co).length_squared)
            distance = (source.vertices[source.loops[source_loop].vertex_index].co - vertex.co).length
            max_corner_match = max(max_corner_match, distance)
            normals[loop_index] = tuple(source.corner_normals[source_loop].vector)
    if max_corner_match > .000002 or any(n is None for n in normals):
        raise RuntimeError(f'{face}: incomplete corner-normal correspondence')
    for vertex, target in zip(head.data.vertices, positions):
        vertex.co = target
    head.data.update()
    head.data.normals_split_custom_set(normals)
    head.data.materials.clear()
    for material in material_carrier.data.materials:
        head.data.materials.append(material)
    report['heads'][face] = {'vertices': len(positions), 'triangles': len(head.data.polygons), 'maxSourceMatchMeters': max_match, 'maxCornerMatchMeters': max_corner_match, 'maxPositionDeltaMeters': max_delta, 'materials': [m.name for m in head.data.materials]}

for obj in imported:
    bpy.data.objects.remove(obj, do_unlink=True)
scene = bpy.context.scene
scene.render.resolution_x = 640
scene.render.resolution_y = 760
scene.render.resolution_percentage = 100
for obj in scene.objects:
    if obj.type == 'MESH':
        visible = obj.name == 'Head_m1' or obj.name.startswith('Beard_m1_boxed_')
        obj.hide_render = not visible
        obj.hide_set(not visible)
bpy.ops.file.pack_all()
bpy.ops.wm.save_as_mainfile(filepath=str(PREVIEW))
print('CLEAN_PREVIEW_READY', json.dumps(report['heads']), flush=True)
RENDERS.mkdir(parents=True, exist_ok=True)
if not args.skip_render:
    for face in args.faces:
        for style in ('boxed', 'goatee', 'full', 'tapered'):
            for obj in scene.objects:
                if obj.type == 'MESH':
                    obj.hide_render = not (obj.name == 'Head_' + face or obj.name.startswith(f'Beard_{face}_{style}_'))
            for view, location in [('front', (0, -.8, 1.655)), ('profile', (.8, 0, 1.655))]:
                scene.camera.location = location
                scene.camera.rotation_euler = (Vector((0, 0, 1.655)) - scene.camera.location).to_track_quat('-Z', 'Y').to_euler()
                scene.camera.data.ortho_scale = .34
                path = RENDERS / f'{face}-{style}-{view}.png'
                scene.render.filepath = str(path)
                bpy.ops.render.render(write_still=True)
                report['renders'].append(str(path.relative_to(ROOT)))
for path in protected:
    if digest(path) != before[str(path.relative_to(ROOT))]:
        raise RuntimeError(f'Protected asset changed unexpectedly: {path}')
report['protectedAssetsUnchanged'] = True
(OUT / 'final-preview-report.json').write_text(json.dumps(report, indent=2), encoding='utf-8')
print('FINAL_BEARD_PREVIEWS_READY', len(report['renders']), str(PREVIEW), flush=True)
