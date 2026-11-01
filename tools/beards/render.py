import bpy,sys,math
from pathlib import Path
from mathutils import Vector
ROOT=Path(__file__).resolve().parents[2];OUT=ROOT/'output/beards';(OUT/'renders').mkdir(exist_ok=True)
bpy.ops.wm.open_mainfile(filepath=str(OUT/'CrownedDogs-Beards-Master.blend'))
sc=bpy.context.scene;sc.render.resolution_x=640;sc.render.resolution_y=760;sc.render.resolution_percentage=100
for face in ['m1','m2','m3','m4']:
 for style in ['boxed','goatee','full','tapered']:
  for o in sc.objects:
   if o.type=='MESH':o.hide_render=not (o.name=='Head_'+face or o.name.startswith(f'Beard_{face}_{style}_'))
  for view,pos in [('front',(0,-.8,1.655)),('profile',(.8,0,1.655))]:
   sc.camera.location=pos;sc.camera.rotation_euler=(Vector((0,0,1.655))-sc.camera.location).to_track_quat('-Z','Y').to_euler();sc.camera.data.ortho_scale=.34
   sc.render.filepath=str(OUT/'renders'/f'{face}-{style}-{view}.png');bpy.ops.render.render(write_still=True)
