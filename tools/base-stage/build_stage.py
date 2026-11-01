"""Reconstruct a bounded photographic set, bake it, export the SAME set/cameras.

Blender 5.2 --background --python tools/base-stage/build_stage.py -- --bake
No depth-map displacement. Ground contacts constrain the hand-modelled volumes.
The source photograph is a projector, never a texture for unseen side faces.
"""
import argparse, json, math, sys
from pathlib import Path
import bpy, bmesh, numpy as np
from mathutils import Vector
from mathutils.geometry import tessellate_polygon

REPO = Path(__file__).resolve().parents[2]
WEB = REPO / 'apps/web/public/img/base-stage'
OUT = REPO / 'release-local/base-stage'
OUT.mkdir(parents=True, exist_ok=True)
cli = argparse.ArgumentParser()
cli.add_argument('--bake', action='store_true')
cli.add_argument('--render', action='store_true')
cli.add_argument('--atlas-size', type=int, default=4096)
args = cli.parse_args(sys.argv[sys.argv.index('--')+1:] if '--' in sys.argv else [])
ASPECT, FOV, HEIGHT, HORIZON = 1882/3344, 55., 2., .30
F = .5 / math.tan(math.radians(FOV/2))

def ray(u, v, d): return Vector(((u-.5)*ASPECT*d/F, HEIGHT+(HORIZON-v)*d/F, -d))
def floor_depth(v):
    assert v > HORIZON
    return HEIGHT*F/(v-HORIZON)
def cv(p): return (p[0], -p[2], p[1])
def uncv(p): return (p[0], p[2], -p[1])
def lerp(a,b,t): return [x+(y-x)*t for x,y in zip(a,b)]

bpy.ops.wm.read_factory_settings(use_empty=True)
scene=bpy.context.scene;scene.name='BASE · Grounded camera projection'
scene.render.engine='CYCLES';scene.cycles.samples=8
scene.cycles.use_denoising=True
scene.render.resolution_x=564;scene.render.resolution_y=1002
scene.render.resolution_percentage=100;scene.render.fps=30
scene.view_settings.view_transform='Standard';scene.view_settings.look='None'
scene.view_settings.exposure=0;scene.view_settings.gamma=1
scene.unit_settings.system='METRIC'
scene.world=bpy.data.worlds.new('World');scene.world.use_nodes=True
scene.world.node_tree.nodes['Background'].inputs[0].default_value=(.018,.02,.022,1)
scene.world.node_tree.nodes['Background'].inputs[1].default_value=.2

def collection(name):
    c=bpy.data.collections.new(name);scene.collection.children.link(c);return c
architecture=collection('01 · Continuous rock, floor and tunnel')
equipment=collection('02 · Capsule, supports and docks')
cameras=collection('00 · Fixed projector and authored paths')
objects=[];repair_polygons=[];contacts=[]

def camera(name):
    data=bpy.data.cameras.new(name);data.sensor_fit='VERTICAL';data.sensor_height=36
    data.lens=36*F;data.shift_y=HORIZON-.5;data.clip_start=.05;data.clip_end=150
    ob=bpy.data.objects.new(name,data);cameras.objects.link(ob)
    ob.location=(0,0,HEIGHT);ob.rotation_euler=(math.pi/2,0,0)
    return ob
projector=camera('PROJECTOR · fixed photograph')
projector.hide_select=True;projector.lock_location=(True,)*3;projector.lock_rotation=(True,)*3
images={}
for key,path in [('original','../base/salt-base.webp'),('clean','clean.png'),('wall','salt-wall.png'),('floor','../battle-stage/hall-floor.png')]:
    im=bpy.data.images.load(str(WEB/path),check_existing=True);im.colorspace_settings.name='sRGB';images[key]=im
if (WEB/'floor-clean.png').exists():
    images['floor_clean']=bpy.data.images.load(str(WEB/'floor-clean.png'),check_existing=True)
    images['floor_clean'].colorspace_settings.name='sRGB'

def mathnode(ns,ls,op,a,b=0):
    n=ns.new('ShaderNodeMath');n.operation=op
    for i,x in enumerate([a,b]):
        if isinstance(x,(int,float)):n.inputs[i].default_value=x
        else:ls.new(x,n.inputs[i])
    return n.outputs[0]

def projected_material(name, environment=False, tile='wall'):
    m=bpy.data.materials.new(name);m.use_nodes=True
    ns,ls=m.node_tree.nodes,m.node_tree.links;ns.clear()
    out=ns.new('ShaderNodeOutputMaterial');em=ns.new('ShaderNodeEmission');ls.new(em.outputs[0],out.inputs[0])
    tc=ns.new('ShaderNodeTexCoord');tc.object=projector
    sep=ns.new('ShaderNodeSeparateXYZ');ls.new(tc.outputs['Object'],sep.inputs[0])
    d=mathnode(ns,ls,'MAXIMUM',.05,mathnode(ns,ls,'MULTIPLY',sep.outputs['Z'],-1))
    u=mathnode(ns,ls,'ADD',mathnode(ns,ls,'MULTIPLY',mathnode(ns,ls,'DIVIDE',sep.outputs['X'],d),F/ASPECT),.5)
    v=mathnode(ns,ls,'ADD',mathnode(ns,ls,'MULTIPLY',mathnode(ns,ls,'DIVIDE',sep.outputs['Y'],d),F),1-HORIZON)
    uv=ns.new('ShaderNodeCombineXYZ');ls.new(u,uv.inputs[0]);ls.new(v,uv.inputs[1])
    photo=ns.new('ShaderNodeTexImage');photo.image=images['original'];photo.extension='EXTEND';ls.new(uv.outputs[0],photo.inputs[0])
    color=photo.outputs['Color']
    if environment:
        clean=ns.new('ShaderNodeTexImage');clean.image=images.get('floor_clean',images['clean']);clean.extension='EXTEND';ls.new(uv.outputs[0],clean.inputs[0])
        mask=ns.new('ShaderNodeTexImage');mask.name='Occluded areas only';mask.extension='EXTEND';ls.new(uv.outputs[0],mask.inputs[0])
        mix=ns.new('ShaderNodeMixRGB');ls.new(mask.outputs[0],mix.inputs[0]);ls.new(color,mix.inputs[1]);ls.new(clean.outputs[0],mix.inputs[2]);color=mix.outputs[0]
        # The original floor contains rails/cables far outside the solid-object
        # outlines. Only the empty plate belongs on this horizontal surface.
        if tile=='floor':color=clean.outputs[0]
        # Beyond the source frustum: spatial UVs on the same physical surface.
        coord=ns.new('ShaderNodeTexCoord');xyz=ns.new('ShaderNodeSeparateXYZ');ls.new(coord.outputs['Object'],xyz.inputs[0])
        tuv=ns.new('ShaderNodeCombineXYZ')
        for i,key in enumerate(('X','Y') if tile=='floor' else ('Y','Z')):
            ls.new(mathnode(ns,ls,'MULTIPLY',xyz.outputs[key],.28),tuv.inputs[i])
        til=ns.new('ShaderNodeTexImage');til.image=images[tile]
        if tile=='floor':ls.new(tuv.outputs[0],til.inputs[0])
        else:
            scale=ns.new('ShaderNodeVectorMath');scale.operation='SCALE';scale.inputs[3].default_value=.28;ls.new(coord.outputs['Object'],scale.inputs[0])
            til.projection='BOX';til.projection_blend=.3;ls.new(scale.outputs['Vector'],til.inputs[0])
        distance=mathnode(ns,ls,'MINIMUM',mathnode(ns,ls,'MINIMUM',u,mathnode(ns,ls,'SUBTRACT',1,u)),mathnode(ns,ls,'MINIMUM',v,mathnode(ns,ls,'SUBTRACT',1,v)))
        inside=mathnode(ns,ls,'MINIMUM',1,mathnode(ns,ls,'MAXIMUM',0,mathnode(ns,ls,'MULTIPLY',distance,12)))
        tint=ns.new('ShaderNodeMixRGB');tint.blend_type='MULTIPLY';tint.inputs[0].default_value=1;tint.inputs[2].default_value=(.48,.48,.48,1);ls.new(til.outputs[0],tint.inputs[1])
        mix=ns.new('ShaderNodeMixRGB');ls.new(inside,mix.inputs[0]);ls.new(tint.outputs[0],mix.inputs[1]);ls.new(color,mix.inputs[2]);color=mix.outputs[0]
        if tile=='floor':
            ao=ns.new('ShaderNodeAmbientOcclusion');ao.inputs['Distance'].default_value=.25;ao.samples=16
            mul=ns.new('ShaderNodeMixRGB');mul.blend_type='MULTIPLY';mul.inputs[0].default_value=.65;ls.new(color,mul.inputs[1]);ls.new(ao.outputs['Color'],mul.inputs[2]);color=mul.outputs[0]
    ls.new(color,em.inputs[0]);return m

photo=projected_material('SOURCE · visible photographed surfaces')
rock=projected_material('SOURCE · rock with repaired occlusions',True)
floor_mat=projected_material('SOURCE · grounded floor with repaired occlusions',True,'floor')

def steel(name,color):
    m=bpy.data.materials.new(name);m.use_nodes=True;ns,ls=m.node_tree.nodes,m.node_tree.links;ns.clear()
    out=ns.new('ShaderNodeOutputMaterial');em=ns.new('ShaderNodeEmission');ls.new(em.outputs[0],out.inputs[0])
    noise=ns.new('ShaderNodeTexNoise');noise.inputs['Scale'].default_value=120
    ramp=ns.new('ShaderNodeValToRGB');ramp.color_ramp.elements[0].color=(*[x*.65 for x in color],1);ramp.color_ramp.elements[1].color=(*color,1)
    ls.new(noise.outputs['Fac'],ramp.inputs[0]);ls.new(ramp.outputs[0],em.inputs[0]);return m
metal=steel('Unseen graphite steel · independent UV material',(.005,.006,.0065))
dark=steel('Underside and foot contact',(.001,.0013,.0015))

def mesh(name,vs,faces,mat=photo,col=equipment,subdivide=0):
    data=bpy.data.meshes.new(name);data.from_pydata([cv(p) for p in vs],[],faces);data.update()
    ob=bpy.data.objects.new(name,data);col.objects.link(ob);data.materials.append(mat)
    if subdivide:
        bm=bmesh.new();bm.from_mesh(data);bmesh.ops.subdivide_edges(bm,edges=list(bm.edges),cuts=subdivide,use_grid_fill=True);bm.to_mesh(data);bm.free()
    objects.append(ob);return ob

def box(name,lo,hi,mat=metal):
    x,y,z=lo;X,Y,Z=hi
    return mesh(name,[(x,y,z),(X,y,z),(X,Y,z),(x,Y,z),(x,y,Z),(X,y,Z),(X,Y,Z),(x,Y,Z)],[(0,3,2,1),(4,5,6,7),(0,1,5,4),(1,2,6,5),(2,3,7,6),(3,0,4,7)],mat)

def surface(name,outline,depth,thickness=.16,bulge=0):
    """Closed ray-calibrated shell; rear faces NEVER receive the source photograph."""
    n=len(outline)
    if not callable(depth):
        dep=depth
        depth=(lambda u,v: float(dep)) if isinstance(dep,(int,float)) else None
    points=[ray(u,v,depth(u,v) if depth else dep[i]) for i,(u,v) in enumerate(outline)]
    # Subdivide the front before adding a gentle 3D relief, retaining exact edges.
    tris=tessellate_polygon([[Vector((u,v,0)) for u,v in outline]])
    def index(v):return v if isinstance(v,int) else min(range(n),key=lambda i:(outline[i][0]-v.x)**2+(outline[i][1]-v.y)**2)
    faces=[tuple(index(p) for p in tri) for tri in tris]
    ob=mesh(name+' · calibrated front',points,faces,photo,subdivide=5)
    if bulge:
        # Rounded profile only in the middle, no lateral cutout drift at silhouette.
        cx=sum(u for u,v in outline)/n;cy=sum(v for u,v in outline)/n
        rx=max(abs(u-cx) for u,v in outline);ry=max(abs(v-cy) for u,v in outline)
        for ve in ob.data.vertices:
            x,y,z=uncv(ve.co);d=-z;u=.5+x*F/(ASPECT*d);v=HORIZON+(HEIGHT-y)*F/d
            weight=max(0,1-((u-cx)/rx)**2-((v-cy)/ry)**2)
            ve.co=cv(ray(u,v,d-bulge*weight))
    # A back shell must stay inside the source silhouette. An arbitrary Z
    # extrusion exposes unpainted caps even in the original calibrated view.
    front=[Vector(uncv(v.co)) for v in ob.data.vertices]
    rear=[]
    for p in front:
        d=-p.z;u=.5+p.x*F/(ASPECT*d);v=HORIZON+(HEIGHT-p.y)*F/d
        rear.append(ray(u,v,d+thickness))
    size=len(front)
    # Use the SAME tessellation on both skins. A single non-planar rear ngon
    # can triangulate through its own front and cover the Persona's legs.
    back_faces=[tuple(i+size for i in reversed(face.vertices)) for face in ob.data.polygons]
    edge_count={}
    for face in ob.data.polygons:
        ids=list(face.vertices)
        for a,b in zip(ids,ids[1:]+ids[:1]):
            key=tuple(sorted((a,b)));edge_count[key]=edge_count.get(key,0)+1
    rims=[(a,b,b+size,a+size) for (a,b),count in edge_count.items() if count==1]
    sides=mesh(name+' · closed rear and rim',front+rear,back_faces+rims,metal)
    repair_polygons.append(outline)
    return ob,sides

# Continuous cross-sections. Every wall foot shares vertices with the floor.
# The slightly oblique tunnel axis is read from the photographic vanishing point.
axis=-.135
sections=[]
for i in range(141):
    d=-3+i*.34
    narrow=max(0,min(1,(d-10)/8))
    left=-1.40+.50*narrow;right=2.65-1.30*narrow;roof=4.35-1.05*narrow
    x0=axis*d
    section=[(left,0),(left-.12,.18),(left-.20,.75),(left-.20,roof-.65),(left+.08,roof-.20),(left+.65,roof),((left+right)/2,roof+.05),(right-.6,roof),(right-.12,roof-.18),(right+.12,roof-.65),(right+.15,.65),(right+.05,.14),(right,0)]
    sections.append([(x+x0,y,-d) for x,y in section])
count=len(sections[0]);vs=[p for section in sections for p in section]
faces=[(i*count+j,(i+1)*count+j,(i+1)*count+j+1,i*count+j+1) for i in range(len(sections)-1) for j in range(count-1)]
mesh('Rock vault · continuous manifold corridor',vs,faces,rock,architecture)
vs=[p for section in sections for p in (section[0],section[-1])]
mesh('Floor · shared wall contacts, y = 0',vs,[(i*2,i*2+1,i*2+3,i*2+2) for i in range(len(sections)-1)],floor_mat,architecture)
mesh('Distant corridor end',sections[-1],[tuple(range(count))],rock,architecture)

# Capsule: one hull with explicitly shared corners. The bottom is constrained to
# the floor, rather than positioned by eye several decimetres above it.
anchors={
    'A':(.065,.507,6.2),'B':(.080,.438,6.5),'C':(.111,.411,6.5),'D':(.207,.415,6.5),'E':(.267,.449,6.4),
    'F':(.270,.508,.75),'G':(.371,.519,.75),'H':(.660,.627,.65),'I':(.681,.655,.42),'J':(.699,.690,.26),
    'K':(.688,.742,0),'L':(.464,.766,0),'M':(.210,.661,0),'N':(.153,.625,0),'O':(.081,.550,0),
    'P':(.113,.556,.49),'Q':(.270,.548,.49),'R':(.242,.620,.42),'S':(.464,.674,.42),'T':(.610,.647,.42)}
depths={k: height if k in 'ABCDE' else (HEIGHT-height)*F/(v-HORIZON) for k,(u,v,height) in anchors.items()}
for name,keys in [('raked backrest','ABCDE FQPO'.replace(' ','')),('right rail','FGHITQ'),('continuous seat and left rail','PQTISR'),('grounded side chassis','OPRSLMN'),('grounded front enclosure','SIJKL')]:
    surface('Capsule · '+name,[(anchors[k][0],anchors[k][1]) for k in keys],[depths[k] for k in keys],.10)
contacts.append(dict(name='capsule chassis',floor=0,minY=min(ray(anchors[k][0],anchors[k][1],depths[k]).y for k in 'KLMNO')))
surface('CROWN · rounded interface',[(.158,.524),(.171,.509),(.209,.495),(.241,.501),(.273,.516),(.290,.537),(.277,.555),(.252,.562),(.213,.555),(.178,.542)],5.90,.27,.27)

# Dock platforms are physical supports. Backplates stand on their platforms;
# the three front panels are finite thickness photographic relief, not sprites.
def dock(name,outline,left,right):
    a=ray(*left,floor_depth(left[1]));b=ray(*right,floor_depth(right[1]))
    # Vertical plane through the measured floor contacts. Every image ray is
    # intersected with that plane, so feet and sockets cannot move independently.
    normal=Vector((b.z-a.z,0,a.x-b.x));constant=normal.dot(a)
    def dep(u,v):
        direction=ray(u,v,1)-Vector((0,HEIGHT,0))
        return constant/normal.dot(direction)
    depths=[floor_depth(v) if i>=len(outline)-3 else dep(u,v) for i,(u,v) in enumerate(outline)]
    surface(name,outline,depths,.24,.035)
    front=[a,b,b+Vector((0,0,-.30)),a+Vector((0,0,-.30))]
    top=[p+Vector((0,.02,0)) for p in front]
    mesh(name+' · platform and floor contact',front+top,[(0,3,2,1),(0,1,5,4),(1,2,6,5),(2,3,7,6),(3,0,4,7),(4,5,6,7)],metal)
    contacts.append(dict(name=name,floor=0,minY=min(p.y for p in front)))
dock('Near Persona and docking cradle',[(.674,.168),(.841,.130),(.861,.469),(.851,.561),(.754,.574),(.684,.548)],(.684,.548),(.851,.561))
dock('Middle Persona and docking cradle',[(.548,.223),(.665,.197),(.682,.510),(.607,.525),(.544,.501)],(.544,.501),(.682,.510))
dock('Far Persona and docking cradle',[(.464,.264),(.536,.245),(.550,.481),(.495,.497),(.459,.485)],(.459,.485),(.550,.481))

# Independent near-field equipment. Countertop has four legs reaching the floor.
surface('Foreground console',[(0,.760),(.126,.729),(.267,.738),(.437,.776),(.718,.819),(1,.896),(1,1),(0,1)],lambda u,v: (HEIGHT-.82)*F/(max(v,.72)-HORIZON),.075)
for u,v in [(.06,.79),(.85,.90),(.1,.98),(.75,.99)]:
    p=ray(u,v,(HEIGHT-.82)*F/(v-HORIZON));box('Console leg',p+Vector((-.035,-.82,-.035)),p+Vector((.035,0,.035)))
surface('Laptop',[(0,.710),(.123,.732),(.105,.825),(0,.826)],[2.62,2.62,2.17,2.17],.05)
surface('Radio bank',[(.722,.779),(.769,.751),(.804,.760),(.824,.792),(.900,.793),(.953,.814),(.951,.920),(.726,.877)],[2.4,2.4,2.4,2.4,2.2,2.2,2.15,2.2],.13)
surface('Radio antenna left',[(.734,.779),(.733,.729),(.742,.719),(.749,.733),(.749,.789)],2.4,.025)
surface('Radio antenna right',[(.898,.804),(.896,.745),(.903,.735),(.911,.746),(.911,.812)],2.2,.025)
surface('Left storage cases',[(0,.576),(.065,.581),(.079,.628),(.146,.646),(.159,.714),(.106,.741),(0,.718)],lambda u,v:floor_depth(.74),.25)
surface('Upright tablet',[(.390,.705),(.421,.691),(.450,.697),(.447,.777),(.410,.780),(.394,.764)],2.98,.07)
surface('Left workbench',[(0,.354),(.106,.385),(.170,.417),(.096,.470),(0,.473)],[8.0,8.0,7.8,7.8,8],.25)

# The far stores used to be painted on the horizontal floor. Bring their
# vertical faces forward and retain bare floor behind them for the tunnel pass.
for name,poly,ground in [
    ('Distant storage lockers',[(.295,.362),(.323,.358),(.329,.352),(.387,.357),(.388,.414),(.302,.414)],.414),
    ('Field cot and cases',[(.321,.391),(.344,.383),(.410,.387),(.421,.427),(.338,.440),(.320,.425)],.440),
    ('Tunnel mesh screen',[(.388,.308),(.477,.298),(.479,.449),(.441,.468),(.397,.445)],.468),
    ('Tunnel backpack',[(.377,.437),(.400,.422),(.431,.424),(.473,.434),(.471,.471),(.403,.474),(.376,.469)],.474),
]:
    ds=[floor_depth(v) if v>=ground-.012 else floor_depth(ground) for u,v in poly]
    surface(name,poly,ds,.24,.025)

# Repair masks are data, not a new painting: same photographed occlusion contours.
w,h=1024,round(1024/ASPECT)
xx,yy=np.meshgrid((np.arange(w)+.5)/w,1-(np.arange(h)+.5)/h)
mask=np.zeros((h,w),bool)
for poly in repair_polygons:
    inside=np.zeros_like(mask)
    for (x1,y1),(x2,y2) in zip(poly,poly[1:]+poly[:1]):
        inside ^= ((y1>yy)!=(y2>yy)) & (xx < (x2-x1)*(yy-y1)/(y2-y1+1e-20)+x1)
    mask |= inside
pixels=np.ones((h,w,4),dtype=np.float32);pixels[:,:,:3]=mask[:,:,None]
mask_image=bpy.data.images.new('Hidden background repair mask',width=w,height=h)
mask_image.colorspace_settings.name='Non-Color';mask_image.pixels.foreach_set(pixels.ravel())
mask_image.pack()
for mat in [rock,floor_mat]:mat.node_tree.nodes['Occluded areas only'].image=mask_image

# Both runtime and Blender use these exact samples; lens is constant throughout.
paths={}
for route in ['battle','fitting']:
    cam=camera('VIEW · '+route.upper());samples=[]
    end=[-.36,1.66,-3.00] if route=='battle' else [-.76,2.0,-7.00]
    aim=list(ray(.235,.527,5.90)) if route=='battle' else [-1.75,1.65,-17.0]
    # The off-axis camera's screen centre is below its optical axis. Compensate
    # the look point, keeping the destination at mid-frame without digital zoom.
    aim[1]+=.75 if route=='battle' else 2.05
    for frame in range(61):
        t=frame/60;e=t*t*t*(t*(6*t-15)+10)
        p=lerp([0,HEIGHT,0],end,e)
        if route=='fitting':p[0]+=.26*math.sin(math.pi*e)
        target=lerp([0,HEIGHT,-8],aim,e)
        samples.append(dict(position=[round(x,7) for x in p],target=[round(x,7) for x in target]))
        cam.location=cv(p);cam.rotation_euler=(Vector(cv(target))-cam.location).to_track_quat('-Z','Y').to_euler()
        cam.keyframe_insert(data_path='location',frame=frame+1);cam.keyframe_insert(data_path='rotation_euler',frame=frame+1)
    paths[route]=samples
    if route=='battle':scene.camera=cam
scene.frame_start=1;scene.frame_end=61;scene.frame_set(1)
for frame in [1,16,31,46,61]:scene.timeline_markers.new(f'{(frame-1)*100//60}%',frame=frame)

for image in images.values():image.pack()
for ob in objects:
    bm=bmesh.new();bm.from_mesh(ob.data);bmesh.ops.recalc_face_normals(bm,faces=list(bm.faces));bm.to_mesh(ob.data);bm.free()

def export_manifest(baked=False):
    parts=[]
    for ob in objects:
        ob.data.calc_loop_triangles();pos=[];uv=[];indices=[]
        for tri in ob.data.loop_triangles:
            for loop in tri.loops:
                pos.extend(round(x,6) for x in uncv(ob.matrix_world @ ob.data.vertices[ob.data.loops[loop].vertex_index].co))
                if baked:uv.extend(round(x,7) for x in ob.data.uv_layers.active.data[loop].uv)
                indices.append(len(indices))
        p=dict(name=ob.name,material='equipment',positions=pos,indices=indices)
        if baked:p['uvs']=uv
        parts.append(p)
    data=dict(version=1,calibration=dict(fov=FOV,aspect=ASPECT,camHeight=HEIGHT,horizon=HORIZON),cameraPaths=paths,
        textures=dict(clean='clean.png',equipment='../base/salt-base.webp'),meshes=parts,
        reconstruction=dict(version=2,contacts=contacts,source='Blender fixed projector; authored closed volumes; shared baked UVs'))
    if baked:data['bakedTexture']='base-atlas.png'
    (WEB/'manifest.json').write_text(json.dumps(data,ensure_ascii=False,separators=(',',':')),encoding='utf-8')

native=OUT/'CrownedDogs-Base-Projection.blend'
if args.bake:
    # Multi-object UV packing; one atlas and one material in the shipped scene.
    bpy.ops.object.select_all(action='DESELECT')
    for ob in objects:ob.select_set(True)
    bpy.context.view_layer.objects.active=objects[0]
    bpy.ops.object.mode_set(mode='EDIT');bpy.ops.mesh.select_all(action='SELECT')
    bpy.ops.uv.smart_project(angle_limit=math.radians(70),island_margin=.004,area_weight=.5)
    bpy.ops.object.mode_set(mode='OBJECT')
    atlas=bpy.data.images.new('BASE · baked photograph and unseen surfaces',width=args.atlas_size,height=args.atlas_size,alpha=False)
    atlas.colorspace_settings.name='sRGB'
    used=set(mat for ob in objects for mat in ob.data.materials)
    for mat in used:
        node=mat.node_tree.nodes.new('ShaderNodeTexImage');node.image=atlas;node.name='BAKE_TARGET';mat.node_tree.nodes.active=node
    scene.render.bake.margin=12;scene.render.bake.use_clear=True
    print('BAKE_START',len(objects),args.atlas_size,flush=True)
    bpy.ops.object.bake(type='EMIT')
    print('BAKE_DONE',flush=True)
    atlas.filepath_raw=str(WEB/'base-atlas.png');atlas.file_format='PNG';atlas.save();atlas.pack()
    baked=bpy.data.materials.new('RUNTIME · same baked UV atlas');baked.use_nodes=True
    ns,ls=baked.node_tree.nodes,baked.node_tree.links;ns.clear()
    tex=ns.new('ShaderNodeTexImage');tex.image=atlas;em=ns.new('ShaderNodeEmission');out=ns.new('ShaderNodeOutputMaterial')
    ls.new(tex.outputs[0],em.inputs[0]);ls.new(em.outputs[0],out.inputs[0])
    # Keep original authored source materials available for future re-bakes.
    for mat in used:mat.use_fake_user=True
    for ob in objects:ob['source_material']=ob.data.materials[0].name;ob.data.materials.clear();ob.data.materials.append(baked)
    export_manifest(True)
bpy.ops.wm.save_as_mainfile(filepath=str(native),compress=True)
if args.render:
    for route in ['battle','fitting']:
        scene.camera=bpy.data.objects['VIEW · '+route.upper()]
        for frame in [1,16,31,46,61]:
            scene.frame_set(frame);scene.render.filepath=str(OUT/f'{route}-{frame:02d}.png');bpy.ops.render.render(write_still=True)
    scene.camera=bpy.data.objects['VIEW · BATTLE'];scene.frame_set(1)
print('BASE_REBUILT',len(objects),'objects','contacts',contacts,flush=True)
