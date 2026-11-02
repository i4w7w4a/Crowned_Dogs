"""close-up renders of the neck: python -I neck_close.py -- IN.blend OUT.png [only=garment] [z=1.48]"""
import bpy, sys, math, numpy as np
from mathutils import Vector
a = sys.argv[sys.argv.index('--')+1:]; opt = dict(x.split('=',1) for x in a[2:])
bpy.ops.wm.open_mainfile(filepath=a[0]); sc = bpy.context.scene
arm = next(o for o in sc.objects if o.type == 'ARMATURE')
if arm.animation_data: arm.animation_data.action = None
for o in sc.objects:
    if o.type in ('MESH','CURVES') and o.parent == arm and not o.get('garment') and opt.get('only') == 'garment': o.hide_render = True
    if o.type == 'MESH' and 'Cap' in o.name: o.hide_render = True
body = max((o for o in sc.objects if o.type == 'MESH' and o.parent == arm and not o.get('garment')), key=lambda o: len(o.vertex_groups))
hide = [g.index for g in body.vertex_groups if g.name.startswith('hide_')]
if hide and opt.get('hide', '1') == '1':
    un = body.vertex_groups.new(name='hide_all')
    for v in body.data.vertices:
        if any(g.group in hide for g in v.groups): un.add([v.index], 1.0, 'REPLACE')
    m = body.modifiers.new('mask', 'MASK'); m.vertex_group = 'hide_all'; m.invert_vertex_group = True
variants = [o for o in sc.objects if o.get('hair_for')]
if variants:
    for o in sc.objects:
        if o.type in ('MESH', 'CURVES') and 'Hair' in o.name and not o.get('hair_for'): o.hide_render = True
    for o in variants: o.hide_render = 'Cap' in o.name
sc.render.engine = 'CYCLES'; sc.cycles.samples = 16; sc.cycles.use_denoising = True; sc.cycles.device='CPU'
w = bpy.data.worlds.new('w'); sc.world = w; w.use_nodes = True; w.node_tree.nodes['Background'].inputs[0].default_value = (0.6,0.6,0.62,1)
L = bpy.data.lights.new('k','SUN'); L.energy = 3; o = bpy.data.objects.new('k', L); sc.collection.objects.link(o); o.rotation_euler = (math.radians(40),0,math.radians(-30))
cam = bpy.data.objects.new('cam', bpy.data.cameras.new('cam')); sc.collection.objects.link(cam); sc.camera = cam
cam.data.type = 'ORTHO'; cam.data.ortho_scale = float(opt.get('scale', 0.45))
z = float(opt.get('z', 1.45))
sc.render.resolution_x = 500; sc.render.resolution_y = 500
views = [('front', (0,-3,z), (90,0,0)), ('side', (3,0,z), (90,0,90)), ('back', (0,3,z), (90,0,180)), ('top', (0,0,3), (0,0,0)), ('under45', (0,-2,z+2), (45,0,0))]
files = []
for nm, loc, rot in views:
    cam.location = loc; cam.rotation_euler = [math.radians(r) for r in rot]
    if nm == 'top': cam.data.clip_end = 3 - (z + 0.02); cam.data.clip_start = 0.1  # cut away above the collar? keep head out
    else: cam.data.clip_start = 0.1; cam.data.clip_end = 10
    f = f'/tmp/_neck_{nm}.png'; sc.render.filepath = f; bpy.ops.render.render(write_still=True); files.append(f)
import subprocess
from PIL import Image
ims = [Image.open(f) for f in files]; W = sum(i.width for i in ims); out = Image.new('RGB', (W, 500))
x = 0
for i in ims: out.paste(i.convert('RGB'), (x, 0)); x += i.width
out.save(a[1])
