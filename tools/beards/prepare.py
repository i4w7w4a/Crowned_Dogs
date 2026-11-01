"""Prepare actual committed game heads at rest for beard authoring. Does not alter source GLB."""
import bpy
from pathlib import Path
from mathutils import Vector
ROOT=Path(__file__).resolve().parents[2];OUT=ROOT/'output/beards';OUT.mkdir(parents=True,exist_ok=True)
bpy.ops.wm.read_factory_settings(use_empty=True);bpy.ops.import_scene.gltf(filepath=str(ROOT/'apps/web/models/persona-male.glb'))
sc=bpy.context.scene;arm=next(o for o in sc.objects if o.type=='ARMATURE');arm.animation_data_clear();arm.data.pose_position='REST';bpy.context.view_layer.update();dg=bpy.context.evaluated_depsgraph_get()
heads={o.name:bpy.data.meshes.new_from_object(o.evaluated_get(dg)) for o in sc.objects if o.type=='MESH' and o.name.startswith('Head_')}
for name,mesh in heads.items():
 o=bpy.data.objects[name];o.modifiers.clear();o.data=mesh;o.parent=None;o.matrix_world.identity()
for o in sc.objects:
 if o.type=='MESH':o.hide_render=o.name!='Head_m1'
sc.render.engine='BLENDER_EEVEE';sc.render.resolution_x=768;sc.render.resolution_y=900;sc.render.resolution_percentage=100;sc.render.image_settings.file_format='PNG';sc.view_settings.view_transform='AgX'
w=bpy.data.worlds.new('World');w.use_nodes=True;w.node_tree.nodes['Background'].inputs[0].default_value=(.35,.35,.35,1);w.node_tree.nodes['Background'].inputs[1].default_value=.5;sc.world=w
for pos,energy,size in [((-.4,-.6,2.1),30,.8),((.4,-.3,1.7),10,.5)]:
 l=bpy.data.lights.new('Softbox','AREA');l.energy=energy;l.shape='DISK';l.size=size;o=bpy.data.objects.new('Softbox',l);sc.collection.objects.link(o);o.location=pos;o.rotation_euler=(Vector((0,0,1.67))-o.location).to_track_quat('-Z','Y').to_euler()
c=bpy.data.cameras.new('Camera');c.type='ORTHO';c.ortho_scale=.34;o=bpy.data.objects.new('Camera',c);sc.collection.objects.link(o);o.location=(0,-.8,1.656);o.rotation_euler=(Vector((0,0,1.656))-o.location).to_track_quat('-Z','Y').to_euler();sc.camera=o
bpy.ops.file.pack_all();bpy.ops.wm.save_as_mainfile(filepath=str(OUT/'heads-rest.blend'))
