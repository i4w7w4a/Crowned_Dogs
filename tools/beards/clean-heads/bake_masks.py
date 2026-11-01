"""Bake authored facial-hair coverage as numerical UV data, never alter source colour textures."""
import bpy, numpy as np, json
from pathlib import Path

REPO=Path(__file__).parents[3]
OUT=REPO.parent/'output/beards-production/clean-heads'
OUT.mkdir(parents=True,exist_ok=True)
SOURCE=str(REPO/'apps/web/models/persona-male.glb')
SIZE=1024
PARAMS={'m2':dict(mouth=1.6148,upper=1.643,lower=1.542,side=1.692),
        'm4':dict(mouth=1.6078,upper=1.642,lower=1.535,side=1.690)}
def smooth(a,b,x):
 t=np.clip((x-a)/(b-a),0,1);return t*t*(3-2*t)
def mask_at(p,c):
 x=np.abs(p[...,0]);y=p[...,1];z=p[...,2]
 # Centre moustache, rising cheek/sideburn boundary, fading before ears/back neck.
 top=c['upper']+.006*smooth(.025,.055,x)+(c['side']-c['upper']-.006)*smooth(.055,.088,x)
 value=(1-smooth(top-.008,top+.008,z))*smooth(c['lower']-.004,c['lower']+.012,z)
 value*=1-smooth(.035,.055,y)
 value*=1-smooth(.094,.105,x)
 # Preserve ear skin behind the sideburn; lower side-jaw still needs full removal.
 ear=smooth(.071,.082,x)*smooth(1.637,1.649,z)*smooth(-.035,-.018,y)
 value*=1-ear
 # Preserve the actual lips including corners and the immediately adjacent vermilion edge.
 lip=np.sqrt((p[...,0]/.0295)**2+((z-c['mouth'])/.0060)**2)
 value*=smooth(.96,1.18,lip)
 # The nose is not facial hair. Keep nose tip and nostril region above the moustache.
 nose=(1-smooth(.014,.021,x))*smooth(c['mouth']+.027,c['mouth']+.034,z)
 value*=1-nose
 return np.clip(value,0,1)

bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.ops.import_scene.gltf(filepath=SOURCE)
for ob in bpy.context.scene.objects:
 ob.animation_data_clear()
 if ob.type=='ARMATURE':ob.data.pose_position='REST'
bpy.context.view_layer.update()
for key,c in PARAMS.items():
 ob=bpy.data.objects['Head_'+key];mesh=ob.data;mesh.calc_loop_triangles()
 mask=np.zeros((SIZE,SIZE),dtype=np.float32)
 for tri in mesh.loop_triangles:
  uv=np.array([mesh.uv_layers.active.data[i].uv[:] for i in tri.loops])*SIZE-.5
  xyz=np.array([mesh.vertices[i].co[:] for i in tri.vertices])
  lo=np.maximum(np.floor(uv.min(axis=0)).astype(int),0);hi=np.minimum(np.ceil(uv.max(axis=0)).astype(int),SIZE-1)
  if np.any(hi<lo):continue
  xx,yy=np.meshgrid(np.arange(lo[0],hi[0]+1),np.arange(lo[1],hi[1]+1))
  a,b,d=uv;den=(b[1]-d[1])*(a[0]-d[0])+(d[0]-b[0])*(a[1]-d[1])
  if abs(den)<1e-10:continue
  u=((b[1]-d[1])*(xx-d[0])+(d[0]-b[0])*(yy-d[1]))/den
  v=((d[1]-a[1])*(xx-d[0])+(a[0]-d[0])*(yy-d[1]))/den
  w=1-u-v;inside=(u>=-.00001)&(v>=-.00001)&(w>=-.00001)
  positions=u[...,None]*xyz[0]+v[...,None]*xyz[1]+w[...,None]*xyz[2]
  values=mask_at(positions,c)*inside
  block=mask[lo[1]:hi[1]+1,lo[0]:hi[0]+1]
  np.maximum(block,values,out=block)
 image=bpy.data.images.new('BeardCoverage_'+key,width=SIZE,height=SIZE,alpha=False,float_buffer=False)
 image.colorspace_settings.name='Non-Color'
 rgba=np.ones((SIZE,SIZE,4),dtype=np.float32);rgba[...,:3]=mask[...,None]
 image.pixels.foreach_set(rgba.ravel());image.filepath_raw=str(OUT/f'{key}-beard-coverage.png');image.file_format='PNG';image.save()
 print('MASK',key,float(mask.max()),int((mask>.5).sum()))
(OUT/'coverage-landmarks.json').write_text(json.dumps(PARAMS,indent=2))
