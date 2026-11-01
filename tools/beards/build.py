"""Reproducible beard cards fitted to the actual game male heads; no existing pixels are edited.
Run Blender -b --factory-startup -P build.py -- [--pilot]
"""
import bpy,math,random,json,sys,os,struct,hashlib
from pathlib import Path
from mathutils import Vector,Matrix
from mathutils.bvhtree import BVHTree
ROOT=Path(__file__).resolve().parents[2]; OUT=ROOT/'output/beards'; OUT.mkdir(parents=True,exist_ok=True)
PUBLIC=ROOT/'apps/web/models';PUBLIC.mkdir(parents=True,exist_ok=True)
PILOT='--pilot' in sys.argv
REPAIR_M4='--repair-m4' in sys.argv
if PILOT and REPAIR_M4:raise RuntimeError('Choose pilot or repair-m4, not both')
bpy.ops.wm.open_mainfile(filepath=str(OUT/('CrownedDogs-Beards-Master.blend' if REPAIR_M4 else 'heads-rest.blend')))
sc=bpy.context.scene
def mesh_signature(obj):
 mesh=obj.data
 return hashlib.sha256(json.dumps({'positions':[tuple(v.co) for v in mesh.vertices],'faces':[tuple(p.vertices) for p in mesh.polygons],'uv':[tuple(d.uv) for d in mesh.uv_layers.active.data] if mesh.uv_layers.active else [],'colors':[tuple(d.color) for d in mesh.color_attributes.active_color.data] if mesh.color_attributes.active_color else []},separators=(',',':')).encode()).hexdigest()
preserved={o.name:mesh_signature(o) for o in sc.objects if o.type=='MESH' and o.name.startswith('Beard_') and not o.name.startswith('Beard_m4_')} if REPAIR_M4 else {}
if REPAIR_M4:
 for o in list(sc.objects):
  if o.name.startswith(('Beard_m4_','Guides_m4_')):bpy.data.objects.remove(o,do_unlink=True)
 for datablocks in (bpy.data.meshes,bpy.data.curves):
  for data in list(datablocks):
   if data.users==0 and data.name.startswith(('Beard_m4_','Guides_m4_')):datablocks.remove(data)
arm=next(o for o in sc.objects if o.type=='ARMATURE')
# Blender reorients imported bones; use the original GLB frame, not edit-bone orientation.
raw=(ROOT/'apps/web/models/persona-male.glb').read_bytes();jn=struct.unpack_from('<I',raw,12)[0];source=json.loads(raw[20:20+jn]);parents={c:i for i,n in enumerate(source['nodes']) for c in n.get('children',[])}
from mathutils import Quaternion
cache={}
def world_node(i):
 if i in cache:return cache[i]
 n=source['nodes'][i]
 if 'matrix' in n:m=Matrix([[n['matrix'][r+c*4] for c in range(4)] for r in range(4)])
 else:
  q=n.get('rotation',[0,0,0,1]);m=Matrix.Translation(Vector(n.get('translation',[0,0,0])))@Quaternion((q[3],q[0],q[1],q[2])).to_matrix().to_4x4()@Matrix.Diagonal((*n.get('scale',[1,1,1]),1))
 if i in parents:m=world_node(parents[i])@m
 cache[i]=m;return m
head_index=next(i for i,n in enumerate(source['nodes']) if n.get('name')=='head')
C=Matrix(((1,0,0,0),(0,0,-1,0),(0,1,0,0),(0,0,0,1)))
HEAD=C@world_node(head_index)@C.inverted()

S=.9630493809698595
FIT=Matrix(((S,0,0,-.00429309),(0,S,0,-.00425317),(0,0,S,.17305149),(0,0,0,1)))
# FIT is authored transformation expressed in Blender axes: glTF(x,y,z)->Blender(x,-z,y).
CONFIG={'m1':(1.608,1.562),'m2':(1.6148,1.572),'m3':(1.614,1.565),'m4':(1.6078,1.559)}
STYLES=['boxed','goatee','full','tapered','stubble']
# Native, authored texture: narrow fibers with varied curvature/opacity, generated from fixed seeds.
W,H=512,1024
pix=[0.0]*(W*H*4);rnd=random.Random(311)
for tile in range(4):
 for strand in range(16):
  u=(strand+.5)/16;phase=rnd.random()*6.28;curve=rnd.uniform(-.08,.08);tip=rnd.uniform(.67,.99)
  shade=rnd.uniform(.68,1.3);width=rnd.uniform(.7,1.5)
  for y in range(int(H*tip)):
   t=y/H;x=tile*128+int(10+u*108+math.sin(t*4+phase)*2.3+curve*t*t*100)
   alpha=min(1,t*70+.32)*min(1,(tip-t)*25)*.94
   for dx in [-1,0,1]:
    xx=x+dx
    if tile*128+2<=xx<(tile+1)*128-2:
     a=alpha*max(0,1-abs(dx)/max(.5,width+1));i=(y*W+xx)*4
     if a>pix[i+3]:pix[i:i+4]=[min(1,104/255*shade),min(1,78/255*shade),min(1,59/255*shade),a]
# Fill transparent RGB too, preventing pale/black interpolation at strand borders.
for i in range(0,len(pix),4):
 if pix[i+3]==0:pix[i:i+3]=[104/255,78/255,59/255]
image=bpy.data.images.new('BeardFiberAtlas',width=W,height=H,alpha=True);image.colorspace_settings.name='Non-Color';image.pixels=pix
image.filepath_raw=str(OUT/'beard-fibers.png');image.file_format='PNG';image.save();image.colorspace_settings.name='sRGB';image.pack()
mat=bpy.data.materials.new('M_Beard_Cards');mat.use_nodes=True;mat.diffuse_color=(.14,.076,.043,1)
nt=mat.node_tree;bs=nt.nodes.get('Principled BSDF');bs.inputs['Roughness'].default_value=.79;bs.inputs['Metallic'].default_value=0;bs.inputs['Specular IOR Level'].default_value=.22
tex=nt.nodes.new('ShaderNodeTexImage');tex.image=image;tex.interpolation='Linear';tex.extension='EXTEND';nt.links.new(tex.outputs['Color'],bs.inputs['Base Color']);nt.links.new(tex.outputs['Alpha'],bs.inputs['Alpha'])
mat.surface_render_method='DITHERED';mat.use_backface_culling=False
# glTF exporter uses the stored legacy blend hint for MASK rather than expensive sorted transparency.
mat['gltf_alpha_mode']='MASK';mat['gltf_alpha_cutoff']=.2
rootmat=bpy.data.materials.new('M_Beard_Root');rootmat.use_nodes=True
rb=rootmat.node_tree.nodes.get('Principled BSDF');rb.inputs['Base Color'].default_value=(1,1,1,1);rb.inputs['Roughness'].default_value=.86;rb.inputs['Specular IOR Level'].default_value=.15
vc=rootmat.node_tree.nodes.new('ShaderNodeVertexColor');vc.layer_name='Color';rootmat.node_tree.links.new(vc.outputs['Color'],rb.inputs['Base Color']);rootmat.node_tree.links.new(vc.outputs['Alpha'],rb.inputs['Alpha']);rootmat.surface_render_method='DITHERED';rootmat.use_backface_culling=False
if REPAIR_M4:
 bpy.data.materials.remove(mat);bpy.data.materials.remove(rootmat);bpy.data.images.remove(image)
 mat=bpy.data.materials['M_Beard_Cards'];rootmat=bpy.data.materials['M_Beard_Root']
beards=[o for o in sc.objects if o.type=='MESH' and o.name.startswith('Beard_')] if REPAIR_M4 else []
guide_objects=[]
stats=[item for item in json.loads((OUT/'geometry-report.json').read_text())['styles'] if item['face']!='m4'] if REPAIR_M4 else []

def m4_rear(z):
 # The m4 scan has a separate low-cheek patch extending backwards below its
 # cheek plane. Front-camera rays must not turn that patch into a beard shelf.
 t=max(0,min(1,(z-1.598)/.027));t=t*t*(3-2*t)
 return -.046+.009*t
def region(p,n,face,style):
 mouth,chin=CONFIG[face];x=abs(p.x);z=p.z
 if n.y>.05 or p.y>-.018 or x>.082 or z<chin+.0005:return 0
 rear=-.018-.019*max(0,min(1,(x-.058)/.024))
 if face=='m4':rear=min(rear,m4_rear(z))
 if p.y>rear:return 0
 # Cheek-to-sideburn line rises outwards, with a soft physical root boundary.
 top=mouth+.005+max(0,x-.030)*.68
 if x<.044:top=max(top,mouth+.021-.16*x-max(0,x-.034)*.65)
 if z>top:return 0
 jaw=chin+.044*(x/.080)**1.8
 if z<jaw:return 0
 # Exact lip opening; moustache stays above, connected at the corners.
 if (x/.033)**2+((z-mouth)/.0085)**2<1:return 0
 if x<.018 and z>mouth+.018:return 0
 # Preserve the natural clean patches beside the soul patch.
 patch=math.exp(-((x-.018)/.010)**2-((z-(mouth-.017))/.008)**2)
 if style=='goatee':
  limit=.029+.012*math.exp(-((z-(mouth-.002))/.023)**2)
  if x>limit:return 0
  if z>mouth+.020:return 0
 lip=math.sqrt((x/.033)**2+((z-mouth)/.0085)**2)
 # Stubble fades out over a wider band, so its border never reads as a drawn line.
 soft=2.4 if style=='stubble' else 1
 edge=min((top-z)/(.006*soft),(z-jaw)/(.004*soft),(.082-x)/(.008*soft),(lip-1)/(.18 if style=='goatee' else .32),(rear-p.y)/(.010*soft))
 if style=='goatee':edge=min(edge,(limit-x)/.004)
 return max(0,min(1,edge))*(1-.85*patch)
def strand_length(p,face,style):
 mouth,chin=CONFIG[face];x=abs(p.x)
 # Light stubble, the default look: the full beard area, sparse fibre under 1.5 mm.
 if style=='stubble':return .0014
 if p.z>mouth+.007 and x<.034:return .0045
 jaw=chin+.044*(x/.080)**1.8
 low=max(0,min(1,(mouth+.012-p.z)/max(.010,mouth+.012-jaw)))
 if style=='boxed':return .0045+.003*low
 if style=='goatee':return .005+.006*low
 if style=='full':return .006+ .019*low
 return .005+.032*low*max(.15,1-(x/.075)**1.4)
for face in (['m1'] if PILOT else ['m4'] if REPAIR_M4 else CONFIG):
 head=bpy.data.objects['Head_'+face];mesh=head.data;mesh.calc_loop_triangles()
 bvh=BVHTree.FromPolygons([v.co for v in mesh.vertices],[list(p.vertices) for p in mesh.polygons])
 collar_guard=min(v.co.z for v in mesh.vertices)+.024
 def clear_collar(co):
  floor=collar_guard+(abs(co.x)*.30 if style=='tapered' else .012*(abs(co.x)/.06)**2)
  if co.z<floor+.004:
   previous=co.z;co.z=floor+.004*math.exp((co.z-floor-.004)/.004);co.y-=(co.z-previous)*.4
  return co

 for style in (['boxed'] if PILOT else STYLES):
  rng=random.Random(730+list(CONFIG).index(face)*32+STYLES.index(style))
  # Thin surface-fitted dense root layer; outer cards provide fiber detail and silhouette.
  rv=[];ra=[];rf=[];grid={};step=.00085
  mouth,chin=CONFIG[face]
  for ix in range(199):
   x=-.084+ix*step
   for iz in range(142):
    z=chin-.005+iz*step
    p,n,_,_=bvh.ray_cast(Vector((x,-.5,z)),Vector((0,1,0)))
    weight=region(p,n,face,style) if p is not None else 0
    if weight<=0:continue
    grid[(ix,iz)]=len(rv)
    jaw=chin+.044*(abs(x)/.080)**1.8
    bulk=strand_length(p,face,style)*max(0,min(1,(mouth+.012-z)/max(.01,mouth+.012-jaw)))*1.08
    if style=='boxed':bulk*=.2
    if style=='stubble':bulk=0
    rv.append(clear_collar(p+n*(.00055+bulk*.40)+Vector((0,0,-bulk*.60))));ra.append(weight*(.5 if style=='stubble' else .88))
  for (ix,iz),a in grid.items():
   if all(k in grid for k in [(ix+1,iz),(ix+1,iz+1),(ix,iz+1)]):
    quad=(a,grid[(ix+1,iz)],grid[(ix+1,iz+1)],grid[(ix,iz+1)])
    # Reject m4 ray discontinuities instead of spanning separate scan branches.
    if face=='m4' and all(abs(rv[i].x)>.045 and rv[i].y>-.065 for i in quad) and max((rv[quad[j]]-rv[quad[(j+1)%4]]).length for j in range(4))>.0045:continue
    rf.append(quad)
  rm=bpy.data.meshes.new(f'Beard_{face}_{style}_root');rm.from_pydata(rv,[],rf);rm.update();col=rm.color_attributes.new(name='Color',type='FLOAT_COLOR',domain='POINT')
  for i,a in enumerate(ra):col.data[i].color=(.07,.039,.022,a)
  for poly in rm.polygons:poly.use_smooth=True
  ro=bpy.data.objects.new(rm.name,rm);sc.collection.objects.link(ro);rm.materials.append(rootmat);beards.append(ro)
  outer=BVHTree.FromPolygons(rv,rf)

  roots=[]
  # Surface-area sampling keeps roots on each real head, including jaw curvature.
  for tri in mesh.loop_triangles:
   vs=[mesh.vertices[i] for i in tri.vertices];a,b,c=[v.co for v in vs];area=(b-a).cross(c-a).length*.5
   count=int(area*360000+rng.random())
   for _ in range(count):
    u,v=rng.random(),rng.random()
    if u+v>1:u,v=1-u,1-v
    p=a+(b-a)*u+(c-a)*v;n=(vs[0].normal*(1-u-v)+vs[1].normal*u+vs[2].normal*v).normalized()
    if rng.random()<region(p,n,face,style)*(.4 if style=='stubble' else 1):roots.append((p.copy(),n.copy()))
  verts=[];faces=[];uvs=[];guides=[]
  for p,n in roots:
   length=strand_length(p,face,style)*rng.uniform(.78,1.12)*(.45+.55*math.sqrt(region(p,n,face,style)))
   down=Vector((rng.uniform(-.27,.27),0,-1))
   if p.z>CONFIG[face][0]+.006 and abs(p.x)<.034:down=Vector((math.copysign(.65,p.x),0,-.35))
   tangent=(down-n*down.dot(n)).normalized()
   if tangent.length<.3:tangent=Vector((0,-1,-.3)).normalized()
   side=tangent.cross(n).normalized();width=rng.uniform(.0009,.0016) if style=='stubble' else rng.uniform(.0025,.0045)
   start=p+n*.00035
   path=[];curl=rng.uniform(-.0014,.0014)
   for k in range(4):
    t=k/3
    center=start+tangent*length*t+n*length*(.08*t+.45*t*t)+side*curl*math.sin(t*math.pi)
    nearest,nn,_,_=bvh.find_nearest(center)
    if nearest is not None and (center-nearest).dot(nn)<.0008:
     center=nearest+nn*.0008
    outer_p,outer_n,_,outer_d=outer.find_nearest(center)
    if outer_p is not None and outer_d<.045 and (center-outer_p).dot(outer_n)<.0008:
     center=outer_p+outer_n*.0008
    path.append(clear_collar(center))
   if face=='m4' and any(center.y>m4_rear(center.z)+.0015 for center in path):continue
   guides.append(path)
   tile=rng.randrange(4);base=len(verts)
   for k,center in enumerate(path):
    t=k/3;w=width*(1-.35*t)
    verts.extend([center-side*w*.5,center+side*w*.5]);uvs.extend([((tile+.025)/4,t),((tile+.975)/4,t)])
   for k in range(3):faces.append((base+k*2,base+k*2+1,base+k*2+3,base+k*2+2))
  m=bpy.data.meshes.new(f'Beard_{face}_{style}_cards');m.from_pydata(verts,[],faces);m.update();uv=m.uv_layers.new(name='UVMap')
  for poly in m.polygons:
   poly.use_smooth=True
   for li in poly.loop_indices:uv.data[li].uv=uvs[m.loops[li].vertex_index]
  o=bpy.data.objects.new(m.name,m);sc.collection.objects.link(o);m.materials.append(mat);o['beardFace']=face;o['beardStyle']=style
  beards.append(o);stats.append({'face':face,'style':style,'cards':len(guides),'triangles':len(faces)*2})
  # Editable native curves retain the exact four-point surface-fitted guides.
  cv=bpy.data.curves.new(f'Guides_{face}_{style}','CURVE');cv.dimensions='3D'
  for path in guides:
   sp=cv.splines.new('POLY');sp.points.add(3)
   for point,co in zip(sp.points,path):point.co=(*co,1)
  go=bpy.data.objects.new(cv.name,cv);sc.collection.objects.link(go);go.hide_render=True;go.hide_viewport=True;guide_objects.append(go)
  print('BUILT',stats[-1],flush=True)
# Preserve master in actual rest-world coordinates and with editable guide curves.
for o in sc.objects:
 if o.type=='MESH':o.hide_render=not(o.name=='Head_m1' or o.name.startswith('Beard_m1_boxed_'))
# Neutral lighting, adjusted for the 30 cm actual head rather than metre-size objects.
for o in sc.objects:
 if o.type=='LIGHT' and not REPAIR_M4:o.data.energy*=.17
sc.world.node_tree.nodes['Background'].inputs[1].default_value=.35
sc.render.resolution_x=768;sc.render.resolution_y=900;sc.render.resolution_percentage=100
sc.view_settings.view_transform='AgX';sc.render.image_settings.file_format='PNG'
sc.camera.data.ortho_scale=.34
if not REPAIR_M4:
 sc.render.filepath=str(OUT/'pilot-boxed-front.png');bpy.ops.render.render(write_still=True)
 sc.camera.location=(.52,-.62,1.656);sc.camera.rotation_euler=(Vector((0,0,1.656))-sc.camera.location).to_track_quat('-Z','Y').to_euler()
 sc.render.filepath=str(OUT/'pilot-boxed-threequarter.png');bpy.ops.render.render(write_still=True)
for mesh_object in sc.objects:
 if mesh_object.type=='MESH':mesh_object.hide_set(mesh_object.hide_render)
bpy.ops.file.pack_all();bpy.ops.wm.save_as_mainfile(filepath=str(OUT/('CrownedDogs-Beards-Pilot.blend' if PILOT else 'CrownedDogs-Beards-Master.blend')))
if REPAIR_M4:
 for name,signature in preserved.items():
  if mesh_signature(bpy.data.objects[name])!=signature:raise RuntimeError('Untargeted beard changed: '+name)
 (OUT/'m4-repair-report.json').write_text(json.dumps({'preservedMeshSha256':preserved,'allOtherTwelveFitsUnchanged':True,'rearLimit':'-46mm at low cheek, eased to -37mm at upper sideburn','maxRootGridEdge':.0045},indent=2))
(OUT/('pilot-geometry-report.json' if PILOT else 'geometry-report.json')).write_text(json.dumps({'headMatrixBlender':[list(r) for r in HEAD],'authoredMatrixBlender':[list(r) for r in FIT],'styles':stats},indent=2))
if not PILOT:
 bpy.ops.object.select_all(action='DESELECT')
 for o in beards:o.hide_set(False);o.hide_viewport=False;o.select_set(True)
 for suffix,transform in [('male',HEAD.inverted()),('authored',FIT)]:
  for o in beards:o.matrix_world=transform
  bpy.ops.export_scene.gltf(filepath=str(PUBLIC/f'beards-{suffix}.glb'),export_format='GLB',use_selection=True,export_yup=True,export_materials='EXPORT',export_animations=False,export_cameras=False,export_lights=False,export_extras=True)
 print('EXPORTED',PUBLIC,flush=True)
 finish=ROOT/'tools/beards/finish_export.py'
 exec(compile(finish.read_text(encoding='utf-8-sig'),str(finish),'exec'),{'__file__':str(finish),'__name__':'__main__'})
