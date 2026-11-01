"""Reference views of the in-game female hair (Art's strand groom in fem.blend). Read only: never saved."""
import bpy, sys, math
from mathutils import Vector
out = sys.argv[sys.argv.index('--') + 1]
arm = bpy.data.objects['Armature']
if arm.animation_data:
    arm.animation_data.action = None
for pb in arm.pose.bones:
    pb.matrix_basis.identity()
for o in bpy.data.objects:
    if o.name.startswith(("FemHairCards", "FemHairCap", "FemHairGuides", "FemHairMesh")):
        o.hide_render = True
    print("OBJ", o.name, o.type, o.hide_render)
sc = bpy.context.scene
sc.render.engine = 'CYCLES'
prefs = bpy.context.preferences.addons['cycles'].preferences
prefs.compute_device_type = 'OPTIX'
prefs.get_devices()
for d in prefs.devices:
    d.use = d.type == 'OPTIX'
sc.cycles.device = 'GPU'
sc.cycles.samples = 64
sc.cycles.use_denoising = True
sc.render.resolution_x, sc.render.resolution_y = 800, 880
w = bpy.data.worlds.new('w'); sc.world = w; w.use_nodes = True
nt = w.node_tree; nt.nodes.clear()
bg = nt.nodes.new('ShaderNodeBackground'); wo = nt.nodes.new('ShaderNodeOutputWorld'); nt.links.new(bg.outputs[0], wo.inputs[0])
bg.inputs[0].default_value = (0.55, 0.57, 0.6, 1); bg.inputs[1].default_value = 0.7
sun = bpy.data.objects.new('sun', bpy.data.lights.new('sun', 'SUN')); sc.collection.objects.link(sun)
sun.data.energy = 2.5; sun.rotation_euler = (math.radians(50), math.radians(10), math.radians(-30))
cam = bpy.data.objects.new('cam', bpy.data.cameras.new('cam')); sc.collection.objects.link(cam); sc.camera = cam
hb = arm.matrix_world @ arm.pose.bones['head'].head
top = max((o.matrix_world @ Vector(c)).z for o in bpy.data.objects if o.name.startswith("FemHairStrands") for c in o.bound_box) if any(o.name.startswith("FemHairStrands") for o in bpy.data.objects) else hb.z + 0.15
T = Vector((0, 0.0, (hb.z + top) / 2 - 0.02))
print("TARGET", T, "headbone", hb, "top", top)
for name, yaw, pitch in (("front", 14, 3), ("back", 194, 6), ("left", 90, 2), ("right", -90, 2)):
    cam.data.lens = 85
    d = Vector((math.sin(math.radians(yaw)) * math.cos(math.radians(pitch)), -math.cos(math.radians(yaw)) * math.cos(math.radians(pitch)), math.sin(math.radians(pitch))))
    cam.location = T + d * 0.95
    cam.rotation_euler = (-d).to_track_quat('-Z', 'Y').to_euler()
    sc.render.filepath = f'{out}/game-{name}.png'
    bpy.ops.render.render(write_still=True)
