"""Check the exported Blender scene's sampled coverage and visible floor contacts.

Blender --background --python-exit-code 1 --python tools/base-stage/check_scene.py -- [--blend FILE]
This is a geometric check, not proof of texture quality or global watertightness.
"""
import argparse
import json
import math
import sys
from pathlib import Path

import bpy
from mathutils import Vector


REPO = Path(__file__).resolve().parents[2]
OUTPUT = REPO / 'release-local/base-stage'
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--blend', type=Path, default=OUTPUT / 'CrownedDogs-Base-Projection.blend')
parser.add_argument('--report', type=Path, default=OUTPUT / 'geometry-check.json')
parser.add_argument('--steps', type=int, default=11)
parser.add_argument('--columns', type=int, default=25)
parser.add_argument('--rows', type=int, default=45)
options = parser.parse_args(sys.argv[sys.argv.index('--') + 1:] if '--' in sys.argv else [])
if min(options.steps, options.columns, options.rows) < 2:
    parser.error('steps, columns and rows must each be at least two')

bpy.ops.wm.open_mainfile(filepath=str(options.blend.resolve()))
scene = bpy.context.scene
source_aspect = scene.render.resolution_x / scene.render.resolution_y
viewports = [('source', scene.render.resolution_x, scene.render.resolution_y),
             ('phone', 390, 844), ('desktop-panel', 480, 688)]
frames = sorted(set(round(scene.frame_start + i * (scene.frame_end - scene.frame_start)
                          / (options.steps - 1)) for i in range(options.steps)))


def viewport_rays(camera, width, height):
    """Use the native camera frame, with the game's centred object-fit-cover crop."""
    corners = camera.data.view_frame(scene=scene)
    left, right = min(p.x for p in corners), max(p.x for p in corners)
    bottom, top = min(p.y for p in corners), max(p.y for p in corners)
    full_width, full_height = max(height * source_aspect, width), max(width / source_aspect, height)
    margin_x, margin_y = (full_width - width) / (2 * full_width), (full_height - height) / (2 * full_height)
    for row in range(options.rows):
        v = .001 + .998 * row / (options.rows - 1)
        source_v = margin_y + v * (1 - 2 * margin_y)
        for column in range(options.columns):
            u = .001 + .998 * column / (options.columns - 1)
            source_u = margin_x + u * (1 - 2 * margin_x)
            local = Vector((left + (right - left) * source_u,
                            top - (top - bottom) * source_v, corners[0].z))
            yield u, v, (camera.matrix_world.to_3x3() @ local).normalized()


results, failures = [], []
minimum_distance = math.inf
for route in ('BATTLE', 'FITTING'):
    camera = bpy.data.objects.get('VIEW · ' + route)
    if camera is None or camera.type != 'CAMERA':
        failures.append('Missing authored camera: ' + route)
        continue
    scene.camera = camera
    for frame in frames:
        scene.frame_set(frame)
        graph = bpy.context.evaluated_depsgraph_get()
        for viewport, width, height in viewports:
            holes, near_hits, samples = [], [], 0
            closest = math.inf
            for u, v, direction in viewport_rays(camera, width, height):
                samples += 1
                hit, point, _, _, obj, _ = scene.ray_cast(graph, camera.matrix_world.translation,
                                                        direction, distance=camera.data.clip_end)
                if not hit:
                    holes.append([round(u, 4), round(v, 4)])
                    continue
                distance = (point - camera.matrix_world.translation).length
                closest = min(closest, distance)
                # Near clipping uses forward camera depth, not oblique ray length.
                forward_depth = -(camera.matrix_world.inverted() @ point).z
                if forward_depth < .05 - 1e-6:
                    near_hits.append({'uv': [round(u, 4), round(v, 4)], 'object': obj.name,
                                      'forwardDepth': round(forward_depth, 6)})
            minimum_distance = min(minimum_distance, closest)
            results.append({'route': route.lower(), 'frame': frame, 'viewport': viewport,
                            'samples': samples, 'holes': len(holes), 'nearClipHits': len(near_hits),
                            'minimumRayDistance': round(closest, 6) if math.isfinite(closest) else None,
                            **({'holeExamples': holes[:8]} if holes else {}),
                            **({'nearClipExamples': near_hits[:8]} if near_hits else {})})
            if holes or near_hits:
                failures.append(f'{route} frame {frame} {viewport}: {len(holes)} holes, '
                                f'{len(near_hits)} near-clip hits')

scene.frame_set(scene.frame_start)
required_contacts = [
    'Capsule · grounded side chassis · calibrated front',
    'Capsule · grounded front enclosure · calibrated front',
    *[name + ' Persona and docking cradle · platform and floor contact'
      for name in ('Near', 'Middle', 'Far')],
]
contact_results = []
for name in required_contacts:
    obj = bpy.data.objects.get(name)
    if obj is None or obj.type != 'MESH':
        failures.append('Missing ground-contact object: ' + name)
        contact_results.append({'object': name, 'present': False})
        continue
    heights = [(obj.matrix_world @ vertex.co).z for vertex in obj.data.vertices]
    minimum, maximum = min(heights), max(heights)
    grounded = abs(minimum) <= .02 and any(abs(height) <= .002 for height in heights)
    contact_results.append({'object': name, 'present': True, 'minWorldHeight': round(minimum, 7),
                            'maxWorldHeight': round(maximum, 7),
                            'floorVerticesWithin2mm': sum(abs(height) <= .002 for height in heights),
                            'grounded': grounded})
    if not grounded:
        failures.append(f'{name}: bottom {minimum:.6f} m is not in contact with the floor')

meshes = [obj for obj in scene.objects if obj.type == 'MESH' and not obj.hide_render]
for obj in meshes:
    obj.data.calc_loop_triangles()
report = {
    'scene': str(options.blend.resolve()),
    'passed': not failures,
    'scope': 'Sampled viewport coverage, forward near clipping and named visible floor contacts. '
             'Separate shells are not asserted to form a single watertight mesh; textures require visual review.',
    'meshes': len(meshes),
    'triangles': sum(len(obj.data.loop_triangles) for obj in meshes),
    'framesPerRoute': frames,
    'grid': [options.columns, options.rows],
    'totalRays': sum(result['samples'] for result in results),
    'totalHoles': sum(result['holes'] for result in results),
    'totalNearClipHits': sum(result['nearClipHits'] for result in results),
    'minimumRayDistance': round(minimum_distance, 6) if math.isfinite(minimum_distance) else None,
    'groundContacts': contact_results,
    'coverage': results,
    'failures': failures,
}
options.report.parent.mkdir(parents=True, exist_ok=True)
options.report.write_text(json.dumps(report, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
print('BASE_GEOMETRY_CHECK', json.dumps({key: report[key] for key in
      ('passed', 'meshes', 'triangles', 'totalRays', 'totalHoles', 'totalNearClipHits', 'minimumRayDistance')},
      ensure_ascii=False), str(options.report), flush=True)
if failures:
    raise RuntimeError('; '.join(failures[:10]))
