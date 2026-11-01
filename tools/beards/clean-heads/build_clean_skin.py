"""Blender material bake of generated shaved skin through authored coverage data.
Source assets stay untouched. Outputs are new companion materials.
"""
import bpy, numpy as np, json
from pathlib import Path
from mathutils import Vector
from mathutils.kdtree import KDTree

REPO=Path(__file__).parents[3]
OUT=REPO.parent/'output/beards-production/clean-heads'
GEN=Path(__file__).parent/'sources'
CANDIDATES={key:GEN/f'{key}-shaved-candidate.png' for key in ('m2','m4')}
def load(path,space):
 im=bpy.data.images.load(str(path),check_existing=False);im.colorspace_settings.name=space;return im
def inode(nt,im):
 n=nt.nodes.new('ShaderNodeTexImage');n.image=im;return n
def look(o,p,t):o.location=p;o.rotation_euler=(Vector(t)-Vector(p)).to_track_quat('-Z','Y').to_euler()
bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.ops.import_scene.gltf(filepath=str(REPO/'apps/web/models/persona-male.glb'))
scene=bpy.context.scene
for ob in scene.objects:
 ob.animation_data_clear()
 if ob.type=='ARMATURE':ob.data.pose_position='REST'
bpy.context.view_layer.update()
scene.render.engine='CYCLES';scene.cycles.samples=1;scene.render.bake.margin=2
materials={};stats={}
for key,path in CANDIDATES.items():
 ob=bpy.data.objects['Head_'+key]
 original_images={}
 for original_material in ob.data.materials:
  for node in original_material.node_tree.nodes:
   if node.type=='TEX_IMAGE' and node.image:
    name=node.image.name
    kind='color' if '_base_' in name else 'normal' if '_normal_' in name else 'orm' if '_rm_' in name else None
    if kind:original_images[kind]=node.image
 if set(original_images)!={'color','normal','orm'}:raise RuntimeError('Missing original maps for '+key)
 bpy.ops.object.select_all(action='DESELECT');ob.select_set(True);bpy.context.view_layer.objects.active=ob
 # New nodes perform a shader composition; existing images are only inputs.
 mat=bpy.data.materials.new('M_Head_'+key+'_clean');mat.use_nodes=True;ob.data.materials.clear();ob.data.materials.append(mat)
 nt=mat.node_tree;nt.nodes.clear()
 output=nt.nodes.new('ShaderNodeOutputMaterial');em=nt.nodes.new('ShaderNodeEmission');nt.links.new(em.outputs[0],output.inputs['Surface'])
 coverage=inode(nt,load(OUT/f'{key}-beard-coverage.png','Non-Color'))
 original=original_images['color'];generated=load(path,'sRGB')
 # Mean skin-tone match from untouched forehead sample vertices, independent of beard.
 def sample(im,uv):
  w,h=im.size;arr=np.array(im.pixels[:]).reshape(h,w,4)
  x=np.clip((uv[:,0]*w).astype(int),0,w-1);y=np.clip((uv[:,1]*h).astype(int),0,h-1)
  return arr[y,x,:3]
 uv=[]
 for loop in ob.data.loops:
  p=ob.data.vertices[loop.vertex_index].co
  if abs(p.x)<.045 and 1.704<p.z<1.732 and p.y<-.035:uv.append(ob.data.uv_layers.active.data[loop.index].uv[:])
 uv=np.array(uv);a=sample(original,uv);b=sample(generated,uv)
 gain=np.clip(np.median(a,axis=0)/np.maximum(np.median(b,axis=0),.001),.7,1.3)
 stats[key]={'colourGain':gain.tolist(),'skinSamples':len(uv)}
 # Correct generated UV-island tone using a smooth 3D skin field sampled from
 # clean upper cheeks, forehead and back neck. The spatial target agrees on
 # both sides of every UV seam; per-corner gains retain fine pore detail.
 positions=np.array([ob.data.vertices[l.vertex_index].co[:] for l in ob.data.loops])
 loopuv=np.array([ob.data.uv_layers.active.data[l.index].uv[:] for l in ob.data.loops])
 orig_rgb=sample(original,loopuv)
 x,y,z=positions.T
 anchor=((np.abs(x)>.025)&(np.abs(x)<.064)&(z>1.652)&(z<1.674)&(y<-.04))|((np.abs(x)<.05)&(z>1.710)&(z<1.735)&(y<-.04))|((y>.058)&(z>1.560)&(z<1.665)&(np.abs(x)<.08))
 ap=positions[anchor][::3];ac=orig_rgb[anchor][::3]
 gp=np.array(generated.pixels[:]).reshape(generated.size[1],generated.size[0],4)[...,:3]
 gx=np.clip((loopuv[:,0]*generated.size[0]).astype(int),0,generated.size[0]-1);gy=np.clip((loopuv[:,1]*generated.size[1]).astype(int),0,generated.size[1]-1)
 # Per-corner source calibration avoids sampling black gutters or another UV island.
 # Mesh density resolves broad colour fields; generated pores between vertices remain.
 base_mean=gp[gy,gx]
 targets=np.empty_like(base_mean)
 for start in range(0,len(positions),512):
  q=positions[start:start+512];dist=((q[:,None,:]-ap[None,:,:])**2).sum(-1);weights=np.exp(-dist/(2*.026**2))+1e-10
  targets[start:start+512]=(weights@ac)/weights.sum(1)[:,None]
 def linear(c):return np.where(c<=.04045,c/12.92,((c+.055)/1.055)**2.4)
 gains=np.clip(linear(targets)/np.maximum(linear(base_mean),.001),.3,2.5)
 attr=ob.data.color_attributes.new(name='CleanSkinGain',type='FLOAT_COLOR',domain='CORNER')
 for i,g in enumerate(gains):attr.data[i].color=(*g,1)
 results={}
 for kind in ['color','normal','orm']:
  space='sRGB' if kind=='color' else 'Non-Color'
  source=original_images[kind]
  src=inode(nt,source);mix=nt.nodes.new('ShaderNodeMixRGB');mix.blend_type='MIX';nt.links.new(coverage.outputs['Color'],mix.inputs[0]);nt.links.new(src.outputs['Color'],mix.inputs[1])
  if kind=='color':
   gen=inode(nt,generated);tint=nt.nodes.new('ShaderNodeVectorMath');tint.operation='MULTIPLY'
   correction=nt.nodes.new('ShaderNodeVertexColor');correction.layer_name='CleanSkinGain';nt.links.new(correction.outputs['Color'],tint.inputs[1])
   nt.links.new(gen.outputs['Color'],tint.inputs[0]);nt.links.new(tint.outputs[0],mix.inputs[2])
  elif kind=='normal':mix.inputs[2].default_value=(.5,.5,1,1)
  else:mix.inputs[2].default_value=(1,.7,0,1)
  nt.links.new(mix.outputs[0],em.inputs['Color'])
  target=bpy.data.images.new(f'{key}-clean-{kind}',width=1024,height=1024,alpha=False);target.colorspace_settings.name=space
  node=inode(nt,target);nt.nodes.active=node
  bpy.ops.object.bake(type='EMIT')
  target.filepath_raw=str(OUT/f'{key}-clean-{kind}.png');target.file_format='PNG';target.save();results[kind]=target
  nt.nodes.remove(node)
 nt.nodes.clear();pr=nt.nodes.new('ShaderNodeBsdfPrincipled');out=nt.nodes.new('ShaderNodeOutputMaterial');nt.links.new(pr.outputs[0],out.inputs['Surface'])
 color=inode(nt,results['color']);nt.links.new(color.outputs['Color'],pr.inputs['Base Color'])
 norm=inode(nt,results['normal']);normal=nt.nodes.new('ShaderNodeNormalMap');nt.links.new(norm.outputs['Color'],normal.inputs['Color']);nt.links.new(normal.outputs[0],pr.inputs['Normal'])
 orm=inode(nt,results['orm']);sep=nt.nodes.new('ShaderNodeSeparateColor');nt.links.new(orm.outputs['Color'],sep.inputs['Color']);nt.links.new(sep.outputs['Green'],pr.inputs['Roughness']);nt.links.new(sep.outputs['Blue'],pr.inputs['Metallic'])
 from io_scene_gltf2.blender.com.material_helpers import create_settings_group,get_gltf_node_name
 ao_group=bpy.data.node_groups.get(get_gltf_node_name()) or create_settings_group(get_gltf_node_name())
 ao=nt.nodes.new('ShaderNodeGroup');ao.node_tree=ao_group;nt.links.new(sep.outputs['Red'],ao.inputs['Occlusion'])
 materials[key]=mat

# Repair beard-region scan seams in a separate companion; source GLBs stay intact.
mask_namespace={'__file__':__file__}
exec((Path(__file__).parent/'bake_masks.py').read_text().split('bpy.ops.wm.read_factory_settings')[0],mask_namespace)
mask_at=mask_namespace['mask_at'];params=mask_namespace['PARAMS']
normal_patches={}
for key in CANDIDATES:
 ob=bpy.data.objects['Head_'+key];mesh=ob.data
 old=np.array([n.vector[:] for n in mesh.corner_normals]);v=np.array([x.co[:] for x in mesh.vertices]);norm=np.array([x.normal[:] for x in mesh.vertices])
 # Weld coincident UV-split vertices for smoothing, otherwise every UV island
 # becomes its own shading patch and introduces new seams.
 # Topology stays intact, but neighbouring UV patch boundaries share a weld group.
 edges={};adj=[set() for _ in v]
 for poly in mesh.polygons:
  ids=list(poly.vertices)
  for a,b in zip(ids,ids[1:]+ids[:1]):
   edge=tuple(sorted((a,b)));edges[edge]=edges.get(edge,0)+1;adj[a].add(b);adj[b].add(a)
 component=np.full(len(v),-1,dtype=int);num=0
 for start in range(len(v)):
  if component[start]>=0:continue
  stack=[start];component[start]=num
  while stack:
   for j in adj[stack.pop()]:
    if component[j]<0:component[j]=num;stack.append(j)
  num+=1
 boundary=sorted({i for edge,n in edges.items() if n==1 for i in edge})
 tree=KDTree(len(boundary))
 for i in boundary:tree.insert(v[i],i)
 tree.balance();parent=list(range(len(v)))
 def find(i):
  while parent[i]!=i:parent[i]=parent[parent[i]];i=parent[i]
  return i
 def union(a,b):
  a,b=find(a),find(b)
  if a!=b:parent[b]=a
 # Exact duplicate points first, then the closest boundary of another UV patch.
 exact={}
 for i,p in enumerate(v):
  k=tuple(np.round(p*1e5).astype(int))
  if k in exact:union(i,exact[k])
  else:exact[k]=i
 gaps=[];nearest={}
 for i in boundary:
  candidates=[(distance,j) for _,j,distance in tree.find_range(v[i],.0015) if component[j]!=component[i]]
  if candidates:
   distance,j=min(candidates);nearest[i]=(j,distance)
 for i,(j,distance) in nearest.items():
  if j in nearest and nearest[j][0]==i:union(i,j);gaps.append(distance)
 _,weld=np.unique([find(i) for i in range(len(v))],return_inverse=True)
 stats[key]['uvComponents']=num;stats[key]['boundaryPairs']=len(gaps);stats[key]['boundaryGapMax']=max(gaps,default=0)
 count=int(weld.max())+1;wnorm=np.zeros((count,3));np.add.at(wnorm,weld,norm);wnorm/=np.maximum(np.linalg.norm(wnorm,axis=1)[:,None],1e-8)
 # UV islands may have different edge tessellation. A fine boundary vertex must
 # meet the other boundary segment, never pull both coarse endpoints into it.
 boundary_edges=np.array([e for e,n in edges.items() if n==1],dtype=int)
 ea=v[boundary_edges[:,0]];eb=v[boundary_edges[:,1]];ev=eb-ea;elen=(ev*ev).sum(1)
 constraints=[]
 for i in boundary:
  if v[i,2]>1.675 or v[i,2]<params[key]['lower']+.008:continue
  t=np.clip(((v[i]-ea)*ev).sum(1)/np.maximum(elen,1e-15),0,1)
  pp=ea+t[:,None]*ev;distance=np.linalg.norm(pp-v[i],axis=1)
  allowed=(component[boundary_edges[:,0]]!=component[i])&(t>.08)&(t<.92)&(distance<.00065)
  allowed&=(norm[boundary_edges[:,0]]@norm[i]>.7)
  if not allowed.any():continue
  j=int(np.argmin(np.where(allowed,distance,np.inf)));a,b=boundary_edges[j]
  if weld[i] in (weld[a],weld[b]):continue
  constraints.append((int(weld[i]),int(weld[a]),int(weld[b]),float(t[j])))
 stats[key]['boundarySegmentConstraints']=len(constraints)
 neighbours=[set() for _ in range(count)]
 for e in mesh.edges:
  a,b=weld[list(e.vertices)];neighbours[a].add(b);neighbours[b].add(a)
 # Stitch coincident/misaligned patch boundaries without positional smoothing.
 # Original positions remain the lookup keys in the exported companion.
 wpos=np.zeros((count,3));tot=np.zeros(count);np.add.at(wpos,weld,v);np.add.at(tot,weld,1);wpos/=tot[:,None]
 original_positions=wpos.copy();smooth=mask_namespace['smooth']
 geom_weight=(1-smooth(1.640,1.675,wpos[:,2]))*(1-smooth(.015,.060,wpos[:,1]))*smooth(params[key]['lower']+.008,params[key]['lower']+.023,wpos[:,2])
 lip=np.sqrt((wpos[:,0]/.026)**2+((wpos[:,2]-params[key]['mouth'])/.0045)**2)
 geom_weight*=smooth(1.4,2.2,lip)
 nose=(1-smooth(.017,.025,np.abs(wpos[:,0])))*smooth(params[key]['mouth']+.020,params[key]['mouth']+.026,wpos[:,2])
 geom_weight*=1-nose
 protected_lips=np.sqrt((v[:,0]/.026)**2+((v[:,2]-params[key]['mouth'])/.0045)**2)<=1.4
 protected_nose=(np.abs(v[:,0])<=.017)&(v[:,2]>=params[key]['mouth']+.026)
 protected_collar=v[:,2]<=params[key]['lower']+.008
 protected=protected_lips|protected_nose|protected_collar|(v[:,2]>=1.675)
 geom_weight[np.unique(weld[protected])]=0
 # Shared group weights keep paired UV copies coincident at full coverage.
 # Apply one common final position to each repaired pair. Multiplying a separate
 # original coordinate by feather weight would reopen the original UV seam.
 target_group=original_positions+(wpos-original_positions)*geom_weight[:,None]
 target_positions=v.copy();active=geom_weight[weld]>.001;target_positions[active]=target_group[weld[active]]
 # T-junctions constrain the FINAL weighted surface, including feather zones.
 for _ in range(4):
  for i,a,b,t in constraints:
   if geom_weight[i]>.001:
    target_group[i]=target_group[a]*(1-t)+target_group[b]*t
  target_positions[active]=target_group[weld[active]]
 if np.linalg.norm(target_positions-v,axis=1).max()>.002:raise RuntimeError('Seam repair exceeded 2 mm budget for '+key)
 for i,vert in enumerate(mesh.vertices):vert.co=target_positions[i]
 mesh.update()
 # Normals follow the stitched surface rather than the old sculpted hair shading.
 wnorm=np.zeros((count,3))
 for poly in mesh.polygons:
  for vertex in poly.vertices:wnorm[weld[vertex]]+=np.array(poly.normal)*poly.area
 wnorm/=np.maximum(np.linalg.norm(wnorm,axis=1)[:,None],1e-8)
 stats[key]['maxPositionDelta']=float(np.linalg.norm(target_positions-v,axis=1).max())
 stats[key]['protectedCollarDelta']=float(np.linalg.norm(target_positions-v,axis=1)[v[:,2]<params[key]['lower']+.008].max(initial=0))
 stats[key]['protectedLipDelta']=float(np.linalg.norm(target_positions-v,axis=1)[protected_lips].max(initial=0))
 stats[key]['protectedNoseDelta']=float(np.linalg.norm(target_positions-v,axis=1)[protected_nose].max(initial=0))
 for _ in range(12):
  mean=np.array([wnorm[list(nn)].mean(0) if nn else wnorm[i] for i,nn in enumerate(neighbours)])
  wnorm=.5*wnorm+.5*mean;wnorm/=np.maximum(np.linalg.norm(wnorm,axis=1)[:,None],1e-8)
 for _ in range(3):
  for i,a,b,t in constraints:
   if geom_weight[i]>.001:
    nn=wnorm[a]*(1-t)+wnorm[b]*t;wnorm[i]=nn/max(np.linalg.norm(nn),1e-9)
 norm=wnorm[weld]
 # Normal blending uses a broad anatomical field, independent of UV islands,
 # beard hairstyle, lips or small colour masks, so no beard-shaped boundary survives.
 smooth=mask_namespace['smooth'];weights=(1-smooth(1.655,1.695,v[:,2]))*(1-smooth(.015,.060,v[:,1]))*smooth(params[key]['lower']+.008,params[key]['lower']+.023,v[:,2])
 weights[protected_lips|protected_nose|protected_collar]=0
 blend=np.array([norm[l.vertex_index]*weights[l.vertex_index]+old[i]*(1-weights[l.vertex_index]) for i,l in enumerate(mesh.loops)])
 blend/=np.maximum(np.linalg.norm(blend,axis=1)[:,None],1e-8)
 mesh.normals_split_custom_set(blend.tolist());normal_patches[key]=(v,[tuple(p.vertices) for p in mesh.polygons],blend,target_positions)

# Neutral material proof using the actual head meshes.
scene.render.engine='BLENDER_EEVEE';scene.render.resolution_x=768;scene.render.resolution_y=900;scene.render.resolution_percentage=100;scene.render.image_settings.file_format='PNG';scene.view_settings.view_transform='Standard'
scene.world=bpy.data.worlds.new('NeutralWorld');scene.world.use_nodes=True;scene.world.node_tree.nodes['Background'].inputs[0].default_value=(.32,.33,.35,1);scene.world.node_tree.nodes['Background'].inputs[1].default_value=.5
for ob in scene.objects:
 if ob.type=='MESH':ob.hide_render=True
cd=bpy.data.cameras.new('Camera');cd.type='ORTHO';cam=bpy.data.objects.new('Camera',cd);scene.collection.objects.link(cam);scene.camera=cam
for key in CANDIDATES:
 ob=bpy.data.objects['Head_'+key];ob.hide_render=False
 pts=[ob.matrix_world@Vector(v) for v in ob.bound_box];lo=Vector(tuple(min(p[i] for p in pts) for i in range(3)));hi=Vector(tuple(max(p[i] for p in pts) for i in range(3)));center=(lo+hi)*.5;size=max(hi-lo);cd.ortho_scale=max(hi.z-lo.z,(hi.x-lo.x)*900/768)*1.14
 for nm,power,offset in [('Key',4,(-.45,-.6,.45)),('Fill',2,(.45,-.4,.1))]:
  ld=bpy.data.lights.new(nm,'AREA');ld.energy=power;ld.size=.5;light=bpy.data.objects.new(nm,ld);scene.collection.objects.link(light);look(light,center+Vector(offset),center)
 for view,offset in [('front',(0,-1,0)),('profile',(-1,0,0))]:
  look(cam,center+Vector(offset)*size*4,center);scene.render.filepath=str(OUT/f'{key}-clean-{view}.png');bpy.ops.render.render(write_still=True)
 mat=materials[key];nt=mat.node_tree;pr=next(n for n in nt.nodes if n.type=='BSDF_PRINCIPLED');out=next(n for n in nt.nodes if n.type=='OUTPUT_MATERIAL');emission=nt.nodes.new('ShaderNodeEmission');nt.links.new(pr.inputs['Base Color'].links[0].from_socket,emission.inputs['Color']);nt.links.new(emission.outputs[0],out.inputs['Surface'])
 look(cam,center+Vector((0,-1,0))*size*4,center);scene.render.filepath=str(OUT/f'{key}-albedo-front.png');bpy.ops.render.render(write_still=True)
 nt.links.new(pr.outputs[0],out.inputs['Surface']);nt.nodes.remove(emission)
 grey=nt.nodes.new('ShaderNodeBsdfPrincipled');grey.inputs['Base Color'].default_value=(.25,.25,.25,1);grey.inputs['Roughness'].default_value=.8;nt.links.new(grey.outputs[0],out.inputs['Surface'])
 scene.render.filepath=str(OUT/f'{key}-grey-front.png');bpy.ops.render.render(write_still=True)
 nt.links.new(pr.outputs[0],out.inputs['Surface']);nt.nodes.remove(grey)
 ob.hide_render=True
 for light in list(scene.objects):
  if light.type=='LIGHT':bpy.data.objects.remove(light,do_unlink=True)

# Carrier geometry transmits only finished materials; runtime hides carriers.
bpy.ops.object.select_all(action='DESELECT')
for key,mat in materials.items():
 mesh=bpy.data.meshes.new('CleanHead_'+key);mesh.from_pydata([(0,0,0),(.001,0,0),(0,.001,0)],[],[(0,1,2)]);mesh.uv_layers.new(name='UVMap')
 for i,uv in enumerate([(0,0),(1,0),(0,1)]):mesh.uv_layers.active.data[i].uv=uv
 mesh.materials.append(mat);ob=bpy.data.objects.new('CleanHead_'+key,mesh);scene.collection.objects.link(ob);ob.select_set(True)
for key,(vertices,faces,normals,targets) in normal_patches.items():
 for authored in (False,True):
  name=('CleanNormalsAuthored_' if authored else 'CleanNormals_')+key
  vv=vertices*.9630493809698595+np.array([-.00429309,-.00425317,.17305149]) if authored else vertices
  mesh=bpy.data.meshes.new(name);mesh.from_pydata(vv.tolist(),[],faces)
  tt=targets*.9630493809698595+np.array([-.00429309,-.00425317,.17305149]) if authored else targets
  positions_attr=mesh.attributes.new(name='_CLEAN_POSITION',type='FLOAT_VECTOR',domain='POINT')
  for item,p in zip(positions_attr.data,tt):item.vector=(p[0],p[2],-p[1])
  for p in mesh.polygons:p.use_smooth=True
  mesh.normals_split_custom_set(normals.tolist());ob=bpy.data.objects.new(name,mesh);scene.collection.objects.link(ob);ob.select_set(True)
export=REPO/'apps/web/models/beards-skin.glb';export.parent.mkdir(parents=True,exist_ok=True)
bpy.ops.export_scene.gltf(filepath=str(export),export_format='GLB',use_selection=True,export_yup=True,export_animations=False,export_image_format='AUTO',export_attributes=True)
(OUT/'clean-skin-stats.json').write_text(json.dumps(stats,indent=2))
print('CLEAN_SKIN_READY',str(export))
