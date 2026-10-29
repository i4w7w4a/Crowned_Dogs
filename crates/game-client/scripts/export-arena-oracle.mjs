// One-shot golden capture from the pre-migration Git source, never the migrated implementation.
import { execFileSync } from 'node:child_process';
import { writeFileSync } from 'node:fs';
import ts from 'typescript';
import * as THREE from '../../../node_modules/three/build/three.module.js';
const original = execFileSync('git',['show','e8251f1:apps/web/src/arena/arena.ts'],{encoding:'utf8'});
const tree = ts.createSourceFile('original.ts', original, ts.ScriptTarget.Latest, true);
const imports = tree.statements.filter(ts.isImportDeclaration);
let stripped = original;
for (const i of [...imports].reverse()) stripped = stripped.slice(0,i.pos)+stripped.slice(i.end);
stripped = stripped.replace('export class Arena','class Arena');
const compiled = ts.transpileModule(stripped,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.None}}).outputText;
const pitches = [];
const api = new Function('exports','THREE','location','pitch','FALL_TIME',compiled+'\nreturn {Arena,calibrate,travel,applyReactions,placePort,zoneWorld,mirrorShader};')({},THREE,{search:''},(bone,angle)=>pitches.push(angle),1.25);
const cases=[];
function capture(kind,input,value){cases.push({kind,input,value:Array.from(value)});}
let seed=928441;function rnd(){seed=(Math.imul(seed,1664525)+1013904223)>>>0;return seed/2**32;}function r(a,b){return a+(b-a)*rnd();}
for(let i=0;i<100;i++){
 const dur=r(.3,4),t=r(0,dur),dt=r(.001,.05),from=r(1,12),to=r(1,12),mine=r(-3,3),time=Math.min(t+dt,dur),[s,v]=api.travel(time,dur);
 capture('arena.walk',[from,to,t,dur,mine,dt],[time,from+(to-from)*s,(from-to)*v,Math.abs(mine*v),+(time>=dur)]);
 const arena=Object.create(api.Arena.prototype);arena.me={fallT:i%7===0?.1:-1,tilt:new THREE.Group(),group:new THREE.Group(),motion:{attackKind:i%3===0?'grenade':i%3===1?'knife':null}};arena.en={group:new THREE.Group()};arena.fp=i%2===0;arena.camera={position:new THREE.Vector3(r(-1,1),r(1,2),r(-1,1))};arena.myGun='rifle';arena.view={at:new THREE.Vector3(r(-1,1),r(1,2),r(-1,1)),tilt:r(-.5,.5),set:i%5!==0};
 arena.bob={phase:r(-8,8),k:r(0,1),depth:r(.01,.06)};const speed=r(0,3),bob=[arena.bob.phase,arena.bob.k,arena.bob.depth,dt,speed,arena.me.fallT];arena.step(dt,speed);capture('arena.bob',bob,[arena.bob.phase,arena.bob.k,arena.me.tilt.position.x,arena.me.tilt.position.y]);
 arena.me.fallT=-1;arena.me.group.rotation.y=r(-3,3);const act=arena.me.motion.attackKind,want=act==='grenade'?[.17,1.47,.12,.53]:act==='knife'?[-.2,1.36,.12,.3]:[0,1.57,.04,0];const view=[...arena.view.at.toArray(),arena.view.tilt,...want,+arena.view.set,dt,arena.me.group.rotation.y,...arena.camera.position.toArray()];arena.holdView(dt);capture('arena.view',view,[...arena.view.at.toArray(),arena.view.tilt,...arena.me.group.position.toArray()]);
 arena.k={kx:r(-.4,.4),me:{x:r(-2,0),y:r(0,.2),z:r(1,4)},back:r(0,.8)};arena.depth=r(2,12);for(const k of ['rim','fill','meKey'])arena[k]={position:new THREE.Vector3(),target:{position:new THREE.Vector3()}};arena.bounce={position:new THREE.Vector3()};const place=[arena.depth,arena.k.kx,arena.k.me.x,arena.k.me.y,arena.k.me.z,arena.k.back,+arena.fp,...arena.camera.position.toArray()];arena.place();capture('arena.place',place,[...arena.en.group.position.toArray(),arena.en.group.rotation.y,...arena.me.group.position.toArray(),arena.me.group.rotation.y,...arena.rim.position.toArray(),...arena.rim.target.position.toArray(),...arena.fill.position.toArray(),...arena.fill.target.position.toArray(),...arena.meKey.position.toArray(),...arena.meKey.target.position.toArray(),...arena.bounce.position.toArray()]);
 const a=new THREE.Vector3(r(-4,4),r(0,2),r(-6,0)),b=a.clone().add(new THREE.Vector3(r(-.1,.1),r(.1,.3),r(-.1,.1))),yaw=r(-3,3);const fig={bones:{head:{getWorldPosition:o=>o.copy(a)},top:{getWorldPosition:o=>o.copy(b)}},group:{rotation:{y:yaw},visible:true},port:{position:new THREE.Vector3()},fallT:-1};api.placePort(fig);capture('arena.port',[...a.toArray(),...b.toArray(),yaw],fig.port.position.toArray());
}
const lensOriginal=execFileSync('git',['show','e8251f1:apps/web/src/arena/lens.ts'],{encoding:'utf8'});
const lensTree=ts.createSourceFile('lens.ts',lensOriginal,ts.ScriptTarget.Latest,true);
let lensSource=lensOriginal;
for(const i of lensTree.statements.filter(ts.isImportDeclaration).reverse())lensSource=lensSource.slice(0,i.pos)+lensSource.slice(i.end);
lensSource=lensSource.replace('export class Lens','class Lens');
const lensCompiled=ts.transpileModule(lensSource,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.None}}).outputText;
let pixels=[];
const document={createElement:()=>({getContext:()=>({drawImage(){},getImageData:()=>({data:pixels})})})};
const Lens=new Function('exports','THREE','document',lensCompiled+'\nreturn Lens;')({},THREE,document);
const target=()=>({size:[0,0],setSize(w,h){this.size=[w,h];}});
for(let i=0;i<100;i++){
 const lens=Object.create(Lens.prototype);lens.view={x:r(0,.3),y:r(0,.3),w:r(.5,1),h:r(.5,1)};lens.look={barrel:i%8===0?0:r(0,.5),near:r(1,5)};lens.px=new THREE.Vector2(r(320,4096),r(480,4096));lens.css=new THREE.Vector2(r(320,1280),r(480,2160));
 const x=r(-.5,1.5),y=r(-.5,1.5),v=lens.view;
 capture('arena.lens.screen',[x,y,v.x,v.y,v.w,v.h,lens.px.x,lens.px.y,lens.look.barrel],lens.toScreen(x,y));
 lens.renderer={capabilities:{maxTextureSize:i%2===0?4096:16384}};lens.q={ssaa:r(1,2),hazeScale:r(.2,1),raysScale:r(.2,1)};
 for(const key of ['fig','comp','near','haze','raysA','raysB'])lens[key]=target();
 lens.composeMat={uniforms:{nearSize:{value:new THREE.Vector2()},hazeSize:{value:new THREE.Vector2()}}};lens.maskMat={uniforms:{lod:{value:0}}};lens.lensMat={uniforms:{compSize:{value:new THREE.Vector2()},outSize:{value:new THREE.Vector2()},ssaa:{value:0},glowLod:{value:0}}};lens.sizeTargets();
 capture('arena.lens.size',[lens.px.x,lens.px.y,lens.css.x,lens.css.y,lens.renderer.capabilities.maxTextureSize,lens.q.ssaa,lens.look.near,lens.q.hazeScale,lens.q.raysScale],[...lens.fig.size,...lens.near.size,...lens.haze.size,...lens.raysA.size,lens.maskMat.uniforms.lod.value,lens.lensMat.uniforms.ssaa.value,lens.lensMat.uniforms.glowLod.value]);
 if(i<10){const w=48,h=48;pixels=Array.from({length:w*h*4},()=>i===0?0:Math.floor(r(0,256)));lens.sun=new THREE.Vector2(.8,.8);lens.horizon=r(.2,.8);const input=[w,h,lens.horizon,.8,.8,...pixels];lens.findSun({width:w,height:h});capture('arena.lens.sun',input,lens.sun.toArray());}
}
writeFileSync(new URL('../fixtures/arena-oracle.json',import.meta.url),JSON.stringify(cases));
console.log(`Captured ${cases.length} numeric cases from e8251f1.`);
