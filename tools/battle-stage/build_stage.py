"""Photographic projection stage: independent clean architecture and equipment volumes.
No depth displacement, no image projected onto unrelated/occluded equipment.
Shared geometry is exported to Three.js and a packed native Blender scene.
"""
import bpy, math, json, shutil, sys, argparse
from pathlib import Path
from mathutils import Vector
from bpy_extras.object_utils import world_to_camera_view

BASE=Path(__file__).resolve().parent
REPO=BASE.parent.parent
IN_REPO=(REPO/'apps/web/public/img/battle-stage').is_dir()
cli=argparse.ArgumentParser()
cli.add_argument('--source-root',type=Path,default=REPO/'apps/web/public/img/battle-stage' if IN_REPO else BASE)
cli.add_argument('--output-root',type=Path,default=REPO/'release-local/battle-stage' if IN_REPO else BASE)
cli.add_argument('--web-root',type=Path,default=REPO/'apps/web/public/img/battle-stage' if IN_REPO else REPO/'battle-camera-stage/apps/web/public/img/battle-stage')
cli.add_argument('--no-render',action='store_true')
args=cli.parse_args(sys.argv[sys.argv.index('--')+1:] if '--' in sys.argv else [])
SOURCE=args.source_root;ROOT=args.output_root;WEB=args.web_root
ROOT.mkdir(parents=True,exist_ok=True)
WEB.mkdir(parents=True,exist_ok=True)
DATA=json.loads((SOURCE/'assetdata.json').read_text(encoding='utf-8'))
FN=.5/math.tan(math.radians(27.5));ASPECT=941/1672;HZ=.37244444444444447
bpy.ops.wm.read_factory_settings(use_empty=True)
scene=bpy.context.scene;scene.name='01 · БОЕВОЙ ПРОЕЗД — 2 метра'
scene.render.engine='CYCLES';scene.cycles.samples=32;scene.cycles.use_denoising=True
scene.render.resolution_x=941;scene.render.resolution_y=1672;scene.render.resolution_percentage=72
scene.render.fps=24;scene.frame_start=1;scene.frame_end=192;scene.sync_mode='FRAME_DROP'
scene.view_settings.view_transform='Standard';scene.view_settings.look='None'
scene.view_settings.exposure=0;scene.view_settings.gamma=1
scene.unit_settings.system='METRIC'
scene.world=bpy.data.worlds.new('Холодный цех');scene.world.use_nodes=True
scene.world.node_tree.nodes['Background'].inputs[0].default_value=(.15,.18,.21,1)
scene.world.node_tree.nodes['Background'].inputs[1].default_value=.4

def coll(name):
    c=bpy.data.collections.new(name);scene.collection.children.link(c);return c
camcol=coll('00 · Камеры');architecture=coll('01 · Восстановленный цех');equipment=coll('02 · Отдельные объёмы оборудования')
def camera(name):
    d=bpy.data.cameras.new(name);d.sensor_fit='VERTICAL';d.sensor_height=36;d.lens=36*FN
    d.shift_y=HZ-.5;d.clip_start=.05;d.clip_end=100
    o=bpy.data.objects.new(name,d);camcol.objects.link(o)
    o.location=(0,0,2);o.rotation_euler=(math.pi/2,0,0);return o
projector=camera('PROJECTOR · неподвижный источник');projector.hide_select=True
projector.lock_location=(True,)*3;projector.lock_rotation=(True,)*3
view=camera('VIEW · боевой проезд');scene.camera=view
view['Демонстрационный ход, м']=2.0
for frame,y in [(1,0),(97,2),(193,0)]:
    view.location.y=y;view.keyframe_insert(data_path='location',frame=frame)
for frame,name in [(1,'ДАЛЕКО'),(25,'ШАГ · около 0.5 м'),(73,'РЫВОК · около 1.5 м'),(97,'БЛИЗКО · 2 м'),(145,'НАЗАД')]:
    scene.timeline_markers.new(name,frame=frame)
scene.frame_set(1)
bpy.context.view_layer.update()

images={}
for key,file in [('clean','hall-clean.png'),('equipment','hall-equipment.jpg'),('floor','hall-floor.png'),('wall','hall-wall.png'),('roof','hall-roof.png')]:
    im=bpy.data.images.load(str(SOURCE/file));im.colorspace_settings.name='sRGB';im.pack();images[key]=im
    if SOURCE.resolve()!=WEB.resolve():shutil.copy2(SOURCE/file,WEB/file)

def mathn(nodes,links,op,a,b):
    n=nodes.new('ShaderNodeMath');n.operation=op
    for i,val in enumerate([a,b]):
        if isinstance(val,(int,float)):n.inputs[i].default_value=val
        else:links.new(val,n.inputs[i])
    return n.outputs[0]
def projected(key):
    m=bpy.data.materials.new('ПРОЕКЦИЯ · '+key);m.use_nodes=True
    ns,ls=m.node_tree.nodes,m.node_tree.links;ns.clear()
    tc=ns.new('ShaderNodeTexCoord');tc.object=projector;tc.location=(-950,100)
    xyz=ns.new('ShaderNodeSeparateXYZ');ls.new(tc.outputs['Object'],xyz.inputs[0])
    z=mathn(ns,ls,'MULTIPLY',xyz.outputs['Z'],-1)
    xz=mathn(ns,ls,'DIVIDE',xyz.outputs['X'],z);yz=mathn(ns,ls,'DIVIDE',xyz.outputs['Y'],z)
    u=mathn(ns,ls,'MULTIPLY_ADD',xz,FN/ASPECT);u.node.inputs[2].default_value=.5
    v=mathn(ns,ls,'MULTIPLY_ADD',yz,FN);v.node.inputs[2].default_value=1-HZ
    uv=ns.new('ShaderNodeCombineXYZ');ls.new(u,uv.inputs['X']);ls.new(v,uv.inputs['Y'])
    tiled=key in ['floor','wall','roof']
    tex=ns.new('ShaderNodeTexImage');tex.image=images['clean' if tiled else key];tex.extension='EXTEND';ls.new(uv.outputs[0],tex.inputs[0]);tex.location=(0,150)
    color=tex.outputs['Color']
    if tiled:
        # Real world-space materials cover surfaces outside the photographed
        # frustum. There is no mirrored photograph or stretched border fallback.
        tu=mathn(ns,ls,'MULTIPLY',.25,mathn(ns,ls,'MULTIPLY',z,-1) if key=='wall' else xyz.outputs['X'])
        tv=mathn(ns,ls,'MULTIPLY',.25,mathn(ns,ls,'ADD',xyz.outputs['Y'],2) if key=='wall' else mathn(ns,ls,'MULTIPLY',z,-1))
        tuv=ns.new('ShaderNodeCombineXYZ');ls.new(tu,tuv.inputs['X']);ls.new(tv,tuv.inputs['Y'])
        tile=ns.new('ShaderNodeTexImage');tile.image=images[key];tile.extension='REPEAT';ls.new(tuv.outputs[0],tile.inputs[0])
        valid=mathn(ns,ls,'GREATER_THAN',z,0)
        for edgeValue in [u,v,mathn(ns,ls,'SUBTRACT',1,u),mathn(ns,ls,'SUBTRACT',1,v)]:
            edge=ns.new('ShaderNodeMapRange');edge.interpolation_type='SMOOTHSTEP';edge.clamp=True
            ls.new(edgeValue,edge.inputs['Value']);edge.inputs['From Min'].default_value=0;edge.inputs['From Max'].default_value=.08
            valid=mathn(ns,ls,'MULTIPLY',valid,edge.outputs[0])
        mix=ns.new('ShaderNodeMixRGB');ls.new(valid,mix.inputs[0]);ls.new(tile.outputs['Color'],mix.inputs[1]);ls.new(color,mix.inputs[2]);color=mix.outputs[0]
    em=ns.new('ShaderNodeEmission');em.location=(300,150);ls.new(color,em.inputs['Color'])
    out=ns.new('ShaderNodeOutputMaterial');out.location=(530,150);ls.new(em.outputs[0],out.inputs[0])
    m['Источник']=key
    return m
frontmats={key:projected(key) for key in images}
colors={'steel':(.09,.095,.10),'concrete':(.29,.29,.28),'sacking':(.48,.47,.44),'wood':(.15,.13,.10)}
sidemats={}
for kind,color in colors.items():
    m=bpy.data.materials.new('БОКОВИНЫ · '+kind);m.use_nodes=True
    p=m.node_tree.nodes.get('Principled BSDF');p.inputs['Base Color'].default_value=(*color,1)
    p.inputs['Roughness'].default_value=.84
    p.inputs['Metallic'].default_value=.25 if kind=='steel' else 0
    m.diffuse_color=(*color,1);sidemats[kind]=m
    filename={'steel':'worn-steel.png','concrete':'salt-concrete.png','sacking':'salt-sacking.png'}.get(kind)
    if filename and (BASE.parent/'battle-projection/textures-v2'/filename).is_file():
        im=bpy.data.images.load(str(BASE.parent/'battle-projection/textures-v2'/filename),check_existing=True);im.pack()
        tex=m.node_tree.nodes.new('ShaderNodeTexImage');tex.image=im;tex.projection='BOX';tex.projection_blend=.2
        co=m.node_tree.nodes.new('ShaderNodeTexCoord');m.node_tree.links.new(co.outputs['Generated'],tex.inputs[0])
        # Subdued hidden surfaces should not create bright silhouette strips.
        tint=m.node_tree.nodes.new('ShaderNodeMixRGB');tint.blend_type='MULTIPLY';tint.inputs[0].default_value=1
        tint.inputs[2].default_value=(*([.38]*3 if kind=='steel' else [.6]*3),1)
        m.node_tree.links.new(tex.outputs['Color'],tint.inputs[1]);m.node_tree.links.new(tint.outputs[0],p.inputs['Base Color'])

def ray(u,v,d):return Vector(((u-.5)*ASPECT/FN*d,d,2+(HZ-v)/FN*d))
def mesh(name,verts,faces,col,materials,face_mats=None):
    data=bpy.data.meshes.new(name);data.from_pydata(verts,[],faces);data.update()
    ob=bpy.data.objects.new(name,data);col.objects.link(ob)
    for mat in materials:data.materials.append(mat)
    if face_mats:
        for face,mi in zip(data.polygons,face_mats):face.material_index=mi
    uv=data.uv_layers.new(name='CameraUV')
    for l in data.loops:
        p=world_to_camera_view(scene,projector,data.vertices[l.vertex_index].co)
        uv.data[l.index].uv=(p.x,p.y)
    mod=ob.modifiers.new('Исходная камера · UV Project','UV_PROJECT')
    mod.uv_layer='CameraUV';mod.projector_count=1;mod.projectors[0].object=projector
    mod.aspect_x=1;mod.aspect_y=1/ASPECT
    return ob

# Floor, walls and roof provide actual metric geometry behind the equipment.
# The back shell is beyond the farthest support feet; the restored plate fills
# the gaps revealed by the longitudinal camera movement.
mesh('Пол · настоящий Z=0',[(-20,-40,0),(20,-40,0),(20,60,0),(-20,60,0)],[(0,1,2,3)],architecture,[frontmats['floor']])
mesh('Левая кирпичная стена',[(-4.4,-40,0),(-4.4,24,0),(-4.4,24,10),(-4.4,-40,10)],[(0,1,2,3)],architecture,[frontmats['wall']])
mesh('Правая кирпичная стена',[(3.2,-40,0),(3.2,-40,10),(3.2,24,10),(3.2,24,0)],[(0,1,2,3)],architecture,[frontmats['wall']])
mesh('Дальний торец · восстановленная подложка',[(-12,24,0),(12,24,0),(12,24,24),(-12,24,24)],[(0,1,2,3)],architecture,[frontmats['clean']])
mesh('Кровля · проекционная поверхность',[(-4.4,-40,9),(3.2,-40,9),(3.2,24,9),(-4.4,24,9)],[(0,1,2,3)],architecture,[frontmats['roof']])

for part in DATA['parts']:
    d=part['depth']
    kind=part.get('material','steel');kind=kind if kind in sidemats else 'steel'
    poly=part['polygon'];n=len(poly)
    # Preserve the exact photographic silhouette while giving the sack a rounded
    # front surface. All points are unprojected from the same immovable camera.
    if kind=='sacking':
        cu=sum(p[0] for p in poly)/n;cv=sum(p[1] for p in poly)/n
        rings=8;bulge=.32
        verts=[ray(cu,cv,d-bulge)];faces=[]
        for ring in range(1,rings+1):
            r=ring/rings
            for u,v in poly:verts.append(ray(cu+(u-cu)*r,cv+(v-cv)*r,d-bulge*(1-r*r)))
            start=1+(ring-1)*n
            for i in range(n):
                j=(i+1)%n
                if ring==1:faces.append((0,start+j,start+i))
                else:faces.append((start-n+i,start-n+j,start+j,start+i))
        outer=1+(rings-1)*n;back=len(verts)
        verts += [ray(u,v,d+.10) for u,v in poly]
        faces += [(outer+i,outer+(i+1)%n,back+(i+1)%n,back+i) for i in range(n)]
        faces.append(tuple(range(back,back+n)))
    else:
        depths=part.get('depths',[d]*n)
        if len(depths)!=n:raise ValueError('Depth count mismatch: '+part['name'])
        verts=[ray(u,v,pd) for (u,v),pd in zip(poly,depths)]
        thickness=min(part['thickness'],.15) if kind=='wood' else part['thickness']
        verts+= [ray(u,v,pd+thickness) for (u,v),pd in zip(poly,depths)]
        faces=[tuple(reversed(range(n))),tuple(range(n,n*2))]
        faces += [(i,(i+1)%n,(i+1)%n+n,i+n) for i in range(n)]
    # Radial backs constrain every projected side UV to the traced object edge;
    # unlike a parallel extrusion they cannot pick up bright background pixels.
    ob=mesh(part['name'],verts,faces,equipment,[frontmats['equipment'],sidemats[kind]],[0]*len(faces))
    ob['Глубина передней поверхности']=d;ob['Силуэт']='Обведён по исходнику; Z после unprojection не переопределяется'

# Cool ambient fill for the physically exposed sides, not for the baked photo.
ld=bpy.data.lights.new('Свет верхнего пролома','AREA');ld.energy=900;ld.size=8
light=bpy.data.objects.new(ld.name,ld);scene.collection.objects.link(light);light.location=(0,10,9)

# One authoritative geometry payload for the game (Blender +Y forward → Three -Z).
payload={'version':1,'calibration':{'fov':55,'aspect':ASPECT,'camHeight':2,'horizon':HZ},
         'textures':{'clean':'hall-clean.png','equipment':'hall-equipment.jpg','floor':'hall-floor.png','wall':'hall-wall.png','roof':'hall-roof.png'},'meshes':[]}
for ob in scene.objects:
    if ob.type!='MESH':continue
    data=ob.data;data.calc_loop_triangles()
    for mi,mat in enumerate(data.materials):
        tris=[t for t in data.loop_triangles if t.material_index==mi]
        if not tris:continue
        material=mat.get('Источник','side')
        verts=[];indices=[];remap={}
        for tri in tris:
            for index in tri.vertices:
                if index not in remap:
                    remap[index]=len(verts)//3;p=data.vertices[index].co
                    verts.extend([round(p.x,6),round(p.z,6),round(-p.y,6)])
                indices.append(remap[index])
        item={'name':ob.name+(' · боковины' if mi else ''),'material':material,'positions':verts,'indices':indices}
        if ob.name.startswith('Пол ·'):item['occludesFigures']=False
        if material=='side':item['color']=list(mat.diffuse_color[:3])
        payload['meshes'].append(item)
(ROOT/'manifest.json').write_text(json.dumps(payload,ensure_ascii=False,separators=(',',':')),encoding='utf-8')
shutil.copy2(ROOT/'manifest.json',WEB/'manifest.json')

# A separate solid view demonstrates the projection is attached to spatial objects.
overview=bpy.data.scenes.new('02 · ОБЪЁМЫ — проверка геометрии')
overview.collection.children.link(equipment)
overview.world=scene.world;overview.render.engine='BLENDER_WORKBENCH'
overview.render.resolution_x=1400;overview.render.resolution_y=900;overview.render.resolution_percentage=100
overview.display.shading.color_type='RANDOM';overview.display.shading.light='STUDIO'
overview.display.shading.show_shadows=True;overview.display.shading.show_cavity=True
oc=bpy.data.objects.new('Обзор объёмов',bpy.data.cameras.new('Обзор объёмов'));overview.collection.objects.link(oc)
oc.location=(12,-4,12);oc.rotation_euler=(Vector((0,10,2))-oc.location).to_track_quat('-Z','Y').to_euler()
oc.data.type='ORTHO';oc.data.ortho_scale=19;overview.camera=oc
note='''БОЕВАЯ СЦЕНА · V3

Пробел — проезд камеры на2м вперёд и назад. Кадр1 — старт, кадр97 — близко.
Фокусное расстояние постоянно. PROJECTOR неподвижен, VIEW перемещается.

Восстановленная архитектура имеет свою фотографию без оборудования.
Оборудование — отдельные замкнутые объёмы с обведёнными по фото контурами.
Конвейеры имеют переменную глубину; мешки — округлую поверхность.
Проекция боковин ограничена контуром предмета, чтобы не захватывать фон.
Пол является плоскостью Z=0, стены и кровля — отдельные поверхности.
Это декорация для продольного боевого движения, не свободная круговая локация.
Никаких Displace и depth mesh. Камера не ограничена сантиметрами.

manifest.json содержит ту же геометрию для игрового Three.js-рендера.
В игре движение окружения связано с фактическим ходом нашего бойца;
движение только противника не передвигает камеру.
'''
bpy.data.texts.new('ПРОЧИТАЙ · боевой проезд').write(note)
(ROOT/'ПРОЧИТАЙ.txt').write_text(note,encoding='utf-8')
scene.frame_set(1);bpy.context.window.scene=scene
for screen in bpy.data.screens:
    for area in screen.areas:
        if area.type=='VIEW_3D':
            area.spaces.active.region_3d.view_perspective='CAMERA'
            area.spaces.active.shading.type='MATERIAL';area.spaces.active.overlay.show_overlays=False
scene.render.resolution_percentage=100
bpy.ops.wm.save_as_mainfile(filepath=str(ROOT/'CrownedDogs-Battle-StageV3.blend'),compress=True)
report={'objects':len([o for o in scene.objects if o.type=='MESH']),
        'render_meshes':len(payload['meshes']),'triangles':sum(len(m['indices'])//3 for m in payload['meshes']),
        'travel_metres':2,'projector_fixed':True,'clean_background':True,'no_depth_mesh':True}
(ROOT/'stage-check.json').write_text(json.dumps(report,indent=2),encoding='utf-8')
print('STAGE_READY',json.dumps(report),flush=True)
if not args.no_render:
    scene.render.resolution_percentage=72
    for frame,name in [(1,'stage-far'),(97,'stage-near')]:
        scene.frame_set(frame);scene.render.filepath=str(ROOT/(name+'.png'))
        bpy.ops.render.render(write_still=True,scene=scene.name)
    overview.render.filepath=str(ROOT/'stage-geometry.png');bpy.ops.render.render(write_still=True,scene=overview.name)
    print('STAGE_RENDERS_READY',flush=True)
