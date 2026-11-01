"""Check renders of a fitted .blend: python -I render_fit.py -- IN.blend OUT.jpg [hide=1]
Front, back, side in the rest pose, plus front and 3/4 views in a test pose (arms up and bent, one knee up)."""
import bpy, sys, math
import numpy as np
from mathutils import Vector, Euler
a = sys.argv[sys.argv.index('--') + 1:]
import os; a[1] = os.path.abspath(a[1])  # Blender reads a bare relative path from the drive root
opt = dict(x.split('=', 1) for x in a[2:])
bpy.ops.wm.open_mainfile(filepath=a[0])
sc = bpy.context.scene
arm = next(o for o in sc.objects if o.type == 'ARMATURE')
if arm.animation_data: arm.animation_data.action = None
body = max((o for o in sc.objects if o.type == 'MESH' and o.parent == arm and not o.get('garment')), key=lambda o: len(o.vertex_groups))
if opt.get('hide', '1') == '1':
    hide = [g for g in body.vertex_groups if g.name.startswith('hide_')]
    if hide:
        # one mask over all garments
        un = body.vertex_groups.get('hide_all') or body.vertex_groups.new(name='hide_all')
        for v in body.data.vertices:
            if any(g.group in [h.index for h in hide] for g in v.groups): un.add([v.index], 1.0, 'REPLACE')
        m = body.modifiers.new('mask', 'MASK'); m.vertex_group = 'hide_all'; m.invert_vertex_group = True
if opt.get('only') == 'garment':
    body.hide_render = True
    for o in sc.objects:
        if o.type == 'MESH' and o.parent == arm and not o.get('garment'): o.hide_render = True
variants = [o for o in sc.objects if o.get('hair_for')]
if variants:  # show the hair squeezed under the helmet instead of the free hair
    for o in sc.objects:
        if o.type in ('MESH', 'CURVES') and 'Hair' in o.name and not o.get('hair_for'): o.hide_render = True
    for o in variants:
        o.hide_render = 'Cap' in o.name and opt.get('cap', '0') != '1'
sc.render.engine = 'CYCLES'; sc.cycles.device = 'CPU'; sc.cycles.samples = int(opt.get('samples', 24)); sc.cycles.use_denoising = True
sc.view_settings.view_transform = 'AgX'
w = bpy.data.worlds.new('w'); sc.world = w; w.use_nodes = True
bg = w.node_tree.nodes['Background']; bg.inputs[0].default_value = (0.62, 0.63, 0.65, 1); bg.inputs[1].default_value = 0.9
for nm, en, rot in [('key', 3.2, (55, 0, -35)), ('fill', 0.9, (70, 0, 140))]:
    L = bpy.data.lights.new(nm, 'SUN'); L.energy = en
    o = bpy.data.objects.new(nm, L); sc.collection.objects.link(o); o.rotation_euler = [math.radians(r) for r in rot]
cam = bpy.data.objects.new('cam', bpy.data.cameras.new('cam')); sc.collection.objects.link(cam); sc.camera = cam
cam.data.type = 'ORTHO'
H = 1.85; cam.data.ortho_scale = H * 1.1
sc.render.resolution_x, sc.render.resolution_y = 400, 640
tiles = []
def shot(tag, yaw, pitch=0):
    d = Vector((math.sin(math.radians(yaw)) * math.cos(math.radians(pitch)), -math.cos(math.radians(yaw)) * math.cos(math.radians(pitch)), math.sin(math.radians(pitch))))
    cam.location = Vector((0, 0, H * 0.48)) + d * 6
    cam.rotation_euler = (-d).to_track_quat('-Z', 'Y').to_euler()
    p = f'{a[1]}.{tag}.png'; sc.render.filepath = p
    bpy.ops.render.render(write_still=True); tiles.append(p)
for pb in arm.pose.bones: pb.matrix_basis.identity()
shot('front', 0); shot('back', 180); shot('side', 90)
P = arm.pose.bones
def rot(b, x=0, y=0, z=0):
    P[b].rotation_mode = 'XYZ'; P[b].rotation_euler = Euler([math.radians(x), math.radians(y), math.radians(z)])
# a rough "aiming" pose in local bone axes (UE mannequin bones)
rot('upperarm_l', 0, 0, -45); rot('lowerarm_l', 0, 0, -70); rot('upperarm_r', 0, 0, 30); rot('lowerarm_r', 0, 0, -60)
rot('thigh_l', 0, 0, 35); rot('calf_l', 0, 0, -60); rot('spine_02', 0, 0, 10)
shot('pose_front', 0); shot('pose_34', 35, 8)
if opt.get('pose2') == '1':  # deep crouch with both arms raised: the hardest stretch for shoulders, crotch and knees
    for pb in arm.pose.bones: pb.matrix_basis.identity()
    rot('upperarm_l', 0, 0, -85); rot('lowerarm_l', 0, 0, -40); rot('upperarm_r', 0, 0, -85); rot('lowerarm_r', 0, 0, -40)
    rot('thigh_l', 0, 0, 75); rot('calf_l', 0, 0, -115); rot('thigh_r', 0, 0, 60); rot('calf_r', 0, 0, -100)
    rot('spine_01', 0, 0, 15); rot('neck_01', 0, 0, -15)
    shot('crouch_front', 0); shot('crouch_side', 90); shot('crouch_back', 200, 10)
from PIL import Image
ims = [Image.open(p).convert('RGB') for p in tiles]
sheet = Image.new('RGB', (400 * len(ims), 640))
for i, im in enumerate(ims): sheet.paste(im, (400 * i, 0))
sheet.save(a[1], quality=85)
import os
for p in tiles: os.remove(p)
