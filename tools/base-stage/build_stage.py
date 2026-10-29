"""Measured, traced projection set for the operator base; no depth-map displacement.
Run in Blender to export the same meshes to the game and an editable native scene.
"""
import json, math
from pathlib import Path
import bpy
from mathutils import Vector
from mathutils.geometry import tessellate_polygon

REPO = Path(__file__).resolve().parents[2]
WEB = REPO / 'apps/web/public/img/base-stage'
OUT = REPO / 'release-local/base-stage'
WEB.mkdir(parents=True, exist_ok=True)
OUT.mkdir(parents=True, exist_ok=True)
ASPECT, FOV, HEIGHT, HORIZON = 1882/3344, 55, 2.0, .30
F = .5/math.tan(math.radians(FOV/2))
parts, repairs = [], []

def ray(u,v,d):
    return [(u-.5)*ASPECT*d/F, HEIGHT+(HORIZON-v)*d/F, -d]

def mesh(name, vertices, faces, material='equipment'):
    indices = []
    for face in faces:
        if len(face) == 3: indices.extend(face); continue
        vectors = [Vector(vertices[i]) for i in face]
        for triangle in tessellate_polygon([vectors]):
            indices.extend(face[v if isinstance(v,int) else vectors.index(v)] for v in triangle)
    parts.append(dict(name=name,material=material,positions=[round(x,6) for p in vertices for x in p],indices=indices))

def shell(name, outline, depths, thickness=.25, rounded=0):
    # Front silhouette follows the photograph; the volume behind it is independent.
    if isinstance(depths,(int,float)): depths=[depths]*len(outline)
    n=len(outline); verts=[ray(u,v,d) for (u,v),d in zip(outline,depths)]
    if rounded:
        center=[sum(x[i] for x in outline)/n for i in (0,1)]
        verts.append(ray(*center,sum(depths)/n-rounded))
        front=[(n,i,(i+1)%n) for i in range(n)]
    else: front=[tuple(range(n))]
    mesh(name+' · projection',verts,front)
    back=[ [p[0],p[1],p[2]-thickness] for p in verts[:n] ]
    side=verts[:n]+back
    mesh(name+' · side volume',side,[(i,(i+1)%n,(i+1)%n+n,i+n) for i in range(n)]+[tuple(range(n,2*n))],'side')
    repairs.append(outline)

# Floor and roof are metric planes. Walls and successive tunnel bays have depth.
mesh('Salt floor',[[-16,0,8],[16,0,8],[16,0,-45],[-16,0,-45]],[(0,1,2,3)],'floor')
mesh('Salt ceiling',[[-16,4.7,8],[-16,4.7,-45],[16,4.7,-45],[16,4.7,8]],[(0,1,2,3)],'roof')
mesh('Left rock wall',[[-2.6,0,8],[-2.6,0,-45],[-2.6,4.7,-45],[-2.6,4.7,8]],[(0,1,2,3)],'wall')
mesh('Right rock wall',[[3.2,0,8],[3.2,4.7,8],[3.2,4.7,-45],[3.2,0,-45]],[(0,1,2,3)],'wall')
mesh('Tunnel far end',[[-16,0,-45],[16,0,-45],[16,12,-45],[-16,12,-45]],[(0,1,2,3)],'clean')
# A continuous arched corridor shell, rather than floating strips of photograph.
section=[(-1.3,0),(-1.3,2.1),(-1.15,2.7),(-.7,3.15),(0,3.3),(.7,3.15),(1.15,2.7),(1.3,2.1),(1.3,0)]
depths=[12,17,23,32,45]
for i,(start,end) in enumerate(zip(depths,depths[1:])):
    vertices=[[x-1.7-.02*(d-12),y,-d] for d in (start,end) for x,y in section]
    count=len(section)
    mesh('Continuous salt tunnel '+str(i),vertices,[(j,j+1,j+1+count,j+count) for j in range(count-1)],'clean')

# Capsule: independent backrest, sloping cushion, foot enclosure and arms.
shell('Capsule backrest',[(.067,.510),(.080,.438),(.111,.411),(.207,.415),(.267,.449),(.270,.548),(.226,.578),(.113,.556)], [4.8,4.8,4.9,4.9,4.7,4.25,4.1,4.5],.30)
shell('Capsule lower body',[(.125,.547),(.262,.537),(.674,.665),(.699,.690),(.688,.742),(.464,.766),(.210,.661)], [4.2,4.3,2.8,2.6,2.55,2.7,3.7],.38)
shell('Capsule cushion',[(.160,.555),(.265,.551),(.585,.648),(.574,.671),(.445,.677),(.242,.601)], [4.25,4.25,2.95,2.85,3.0,3.75],.09)
shell('Left arm rail',[(.081,.490),(.119,.496),(.236,.610),(.213,.654),(.153,.625)], [4.7,4.65,3.6,3.6,3.85],.12)
shell('Right arm rail',[(.275,.514),(.371,.519),(.660,.627),(.681,.655),(.610,.647),(.328,.547)], [4.4,4.4,2.85,2.75,2.9,4.1],.12)
shell('CROWN operator interface',[(.159,.524),(.172,.509),(.210,.497),(.240,.502),(.273,.516),(.289,.536),(.277,.555),(.252,.562),(.213,.555),(.178,.543)],4.25,.20,.15)
# Tables and portable hardware near the camera create visible foreground parallax.
shell('Foreground console',[(0,.760),(.126,.729),(.267,.738),(.437,.776),(.718,.819),(1,.896),(1,1),(0,1)], [2.8,3.0,3.0,2.8,2.3,2.0,1.6,1.6],.30)
shell('Laptop',[(0,.710),(.123,.732),(.105,.825),(0,.826)], [2.0,2.0,1.55,1.55],.07)
shell('Radio rack',[(.722,.779),(.769,.751),(.804,.760),(.824,.792),(.900,.793),(.953,.814),(.951,.920),(.726,.877)], [1.7,1.8,1.8,1.7,1.6,1.5,1.4,1.5],.13)
shell('Antenna left',[(.734,.779),(.733,.729),(.742,.719),(.749,.733),(.749,.789)],1.75,.025)
shell('Antenna right',[(.898,.804),(.896,.745),(.903,.735),(.911,.746),(.911,.812)],1.6,.025)
shell('Left storage cases',[(0,.576),(.065,.581),(.079,.628),(.146,.646),(.159,.714),(.106,.741),(0,.718)], [4,4,3.8,3.2,3.1,3.0,3.1],.35)
shell('Tablet upright',[(.390,.705),(.421,.691),(.450,.697),(.447,.777),(.410,.780),(.394,.764)],2.2,.09)
# Complete docking-bay silhouettes include the parked Personas and their mechanical beds.
shell('Near Persona dock',[(.674,.167),(.841,.127),(.861,.469),(.855,.562),(.752,.570),(.684,.548)], [5.8,5.1,5.1,5.0,5.4,5.8],.40)
shell('Middle Persona dock',[(.548,.223),(.665,.197),(.682,.518),(.581,.523),(.544,.496)], [7.4,6.8,6.8,7.3,7.4],.35)
shell('Far Persona dock',[(.464,.264),(.536,.245),(.548,.481),(.484,.494),(.459,.481)], [9.2,8.6,8.6,9.2,9.2],.30)
# The wall-mounted pipe remains on the wall; its clean-plate image intentionally retains it.
shell('Left workbench',[(0,.354),(.106,.385),(.170,.417),(.096,.470),(0,.473)], [7.0,6.5,6,5.6,5.8],.4)
# One continuous repair region also removes the tiny photographic seams between
# cushion/arms/chassis. Otherwise a one-pixel rail can become a tall floor streak.
repairs.append([(.04,.54),(.055,.43),(.105,.403),(.225,.405),(.285,.45),(.29,.50),(.395,.51),(.69,.625),(.715,.69),(.705,.755),(.48,.79),(.17,.695),(.135,.63),(.065,.61)])

payload=dict(version=1,calibration=dict(fov=FOV,aspect=ASPECT,camHeight=HEIGHT,horizon=HORIZON),
    textures=dict(clean='clean.png',equipment='../base/salt-base.webp',floor='../battle-stage/hall-floor.png',wall='salt-wall.png',roof='salt-wall.png'),
    projectSides=True,repairPolygons=repairs,meshes=parts)
(WEB/'manifest.json').write_text(json.dumps(payload,ensure_ascii=False,separators=(',',':')),encoding='utf-8')

# Native Blender file, with a fixed projector and two separate dolly cameras.
bpy.ops.wm.read_factory_settings(use_empty=True)
scene=bpy.context.scene;scene.name='База оператора · Camera Projection'
scene.render.engine='CYCLES';scene.cycles.samples=16
scene.render.resolution_x=941;scene.render.resolution_y=1672;scene.render.resolution_percentage=60
scene.view_settings.view_transform='Standard';scene.view_settings.look='None'
def camera(name):
    c=bpy.data.objects.new(name,bpy.data.cameras.new(name));scene.collection.objects.link(c)
    c.data.sensor_fit='VERTICAL';c.data.sensor_height=36;c.data.lens=36*F;c.data.shift_y=HORIZON-.5
    c.location=(0,0,HEIGHT);c.rotation_euler=(math.pi/2,0,0);return c
projector=camera('PROJECTOR · fixed');projector.hide_select=True
projector.lock_location=(True,)*3;projector.lock_rotation=(True,)*3
images={k:bpy.data.images.load(str(WEB/v),check_existing=True) for k,v in payload['textures'].items()}
for image in images.values():image.pack()
materials={}
for kind in ['clean','equipment','side','floor','wall','roof']:
    m=bpy.data.materials.new(kind);m.use_nodes=True;nodes=m.node_tree.nodes;links=m.node_tree.links;nodes.clear()
    out=nodes.new('ShaderNodeOutputMaterial');em=nodes.new('ShaderNodeEmission');links.new(em.outputs[0],out.inputs[0])
    coord=nodes.new('ShaderNodeTexCoord');coord.object=projector
    sep=nodes.new('ShaderNodeSeparateXYZ');links.new(coord.outputs['Object'],sep.inputs[0])
    def mathnode(op,a,b):
        n=nodes.new('ShaderNodeMath');n.operation=op
        for i,x in enumerate([a,b]):
            if isinstance(x,(int,float)):n.inputs[i].default_value=x
            else:links.new(x,n.inputs[i])
        return n.outputs[0]
    d=mathnode('MULTIPLY',sep.outputs['Z'],-1)
    u=mathnode('ADD',mathnode('MULTIPLY',mathnode('DIVIDE',sep.outputs['X'],d),F/ASPECT),.5)
    v=mathnode('ADD',mathnode('MULTIPLY',mathnode('DIVIDE',sep.outputs['Y'],d),F),1-HORIZON)
    uv=nodes.new('ShaderNodeCombineXYZ');links.new(u,uv.inputs['X']);links.new(v,uv.inputs['Y'])
    tex=nodes.new('ShaderNodeTexImage');tex.image=images['equipment' if kind in ['equipment','side'] else 'clean'];tex.extension='EXTEND'
    links.new(uv.outputs[0],tex.inputs[0]);links.new(tex.outputs['Color'],em.inputs['Color'])
    materials[kind]=m
for part in parts:
    coords=part['positions'];vs=[(coords[i],-coords[i+2],coords[i+1]) for i in range(0,len(coords),3)]
    ids=part['indices'];fs=[ids[i:i+3] for i in range(0,len(ids),3)]
    data=bpy.data.meshes.new(part['name']);data.from_pydata(vs,[],fs);data.update()
    ob=bpy.data.objects.new(part['name'],data);scene.collection.objects.link(ob);data.materials.append(materials[part['material']])
for name,target,position in [('VIEW · CROWN',ray(.235,.527,4.25),[-.40,1.40,-2.55]),('VIEW · TUNNEL',ray(.275,.36,18),[-.9,1.9,-6.8])]:
    c=camera(name);crown='CROWN' in name;target[1]+=.38 if crown else 2.0
    for frame in range(1,61):
        t=(frame-1)/59;e=t*t*t*(t*(6*t-15)+10)
        p=[a+(b-a)*e for a,b in zip([0,2,0],position)]
        if not crown:p[0]+=1.4*math.sin(math.pi*e)
        aim=[a+(b-a)*e for a,b in zip([0,2,-8],target)]
        c.location=(p[0],-p[2],p[1]);look=Vector((aim[0],-aim[2],aim[1]))
        c.rotation_euler=(look-c.location).to_track_quat('-Z','Y').to_euler()
        c.keyframe_insert(data_path='location',frame=frame);c.keyframe_insert(data_path='rotation_euler',frame=frame)
    if 'CROWN' in name:scene.camera=c
scene.frame_end=60;scene.frame_set(1)
bpy.ops.wm.save_as_mainfile(filepath=str(OUT/'CrownedDogs-Base-Projection.blend'),compress=True)
print('BASE_STAGE_READY',len(parts),'meshes',sum(len(p['indices'])//3 for p in parts),'triangles',flush=True)
