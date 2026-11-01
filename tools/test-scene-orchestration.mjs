// Real Three scene and shader parity; no GPU or audio device is opened.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import ts from "typescript";
import * as THREE from "three";
import { callRust, RustSceneHand, RustDepthBackdrop, sceneGripOf, sceneDress, sceneMakeFittingStage, sceneMakeSling, sceneTuneFittingMaterial, sceneLoadModel, sceneLoadGuns, sceneInstallRelief, sceneAuthoredWeaponFit } from "../packages/runtime/src/index.ts";
const root = resolve(import.meta.dirname, "..");
let fixture = 0;
async function original(file, bindings) {
  const key = `__sceneFixture${fixture++}`; globalThis[key] = bindings;
  const source = readFileSync(resolve(root, "crates/game-wasm/reference/browser90", file), "utf8");
  const output = ts.transpileModule(source, { compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 } }).outputText.replace(/^import .*?;\s*$/gm, "");
  const result = await import(`data:text/javascript;base64,${Buffer.from(`const {${Object.keys(bindings)}}=globalThis.${key};\n${output}`).toString("base64")}`);
  delete globalThis[key]; return result;
}
const host = { THREE, callRust, graphicsTexture: (_kind, size) => new Uint8Array(size * size * 4) };
const quality = callRust("client.data", []).QUALITY;
const oldMaterial = await original("arena/materials.ts", { THREE, callRust, surfaceReliefHook: () => undefined });
const oldTurntable = await original("base/turntable-material.ts", { THREE, callRust, graphicsTexture: host.graphicsTexture, tune: oldMaterial.tune });
const oldStage = await original("base/fitting-stage.ts", { THREE, callRust, graphicsTexture: host.graphicsTexture, ...oldTurntable });
const oldWeapons = await original("arena/weapons.ts", { THREE, loadModel: async () => { throw Error("missing"); } });
const oldDress = await original("arena/dress.ts", { THREE });
function snapshot(node) {
  const g = node.geometry, m = node.material;
  return {name:node.name,type:node.type,position:node.position.toArray(),rotation:node.rotation.toArray(),scale:node.scale.toArray(),visible:node.visible,cast:node.castShadow,receive:node.receiveShadow,
    geometry:g && {type:g.type,index:g.index && [...g.index.array],attributes:Object.fromEntries(Object.entries(g.attributes).map(([k,a])=>[k,[...a.array]]))},
    material:m && {type:m.type,color:m.color?.getHex(),roughness:m.roughness,metalness:m.metalness,opacity:m.opacity,bumpScale:m.bumpScale},children:node.children.map(snapshot)};
}
function close(actual,expected) {assert.equal(actual.length,expected.length);actual.forEach((v,i)=>assert.ok(Math.abs(v-expected[i])<1e-10,`${v} != ${expected[i]}`));}
function shader(material,kind="standard") {const t=THREE.ShaderLib[kind];const s={vertexShader:t.vertexShader,fragmentShader:t.fragmentShader,uniforms:{}};material.onBeforeCompile(s,{});return s;}
function compareShaders(a,b) {assert.equal(a.vertexShader.replace(/\s+/g,""),b.vertexShader.replace(/\s+/g,""));assert.equal(a.fragmentShader.replace(/\s+/g,""),b.fragmentShader.replace(/\s+/g,""));assert.deepEqual(Object.keys(a.uniforms),Object.keys(b.uniforms));}

// Authored geometry and all three POM shader passes remain the same at every quality.
const next=sceneMakeFittingStage(host),previous=oldStage.makeFittingStage();
assert.deepEqual(snapshot(next.fixed),snapshot(previous.fixed));assert.deepEqual(snapshot(next.rotating),snapshot(previous.rotating));
const newMesh=next.rotating.getObjectByName("Silhouette_POM_Finite_Volume"),oldMesh=previous.rotating.getObjectByName("Silhouette_POM_Finite_Volume");assert.ok(newMesh && oldMesh);
for(const id of ["low","medium","high","ultra"]){next.setQuality(id);previous.setQuality(id);sceneTuneFittingMaterial(host,newMesh.material,quality[id]);oldTurntable.tuneFittingMaterial(oldMesh.material,quality[id]);assert.equal(newMesh.visible,oldMesh.visible);assert.equal(newMesh.material.customProgramCacheKey(),oldMesh.material.customProgramCacheKey());compareShaders(shader(newMesh.material),shader(oldMesh.material));compareShaders(shader(newMesh.customDepthMaterial,"depth"),shader(oldMesh.customDepthMaterial,"depth"));compareShaders(shader(newMesh.customDistanceMaterial,"distance"),shader(oldMesh.customDistanceMaterial,"distance"));}
const camera=new THREE.PerspectiveCamera(45,.7,.1,30);camera.position.set(2,1.8,4);camera.lookAt(0,1,0);camera.updateMatrixWorld();
next.rotating.rotation.y=previous.rotating.rotation.y=.72;next.rotating.updateMatrixWorld(true);previous.rotating.updateMatrixWorld(true);for(const mesh of [newMesh,oldMesh])mesh.onBeforeRender(null,null,camera);
const a=shader(newMesh.material),b=shader(oldMesh.material);for(const key of ["cdPomCameraLocal","cdPomRayLocal","cdPomModelView","cdPomModelWorld","cdPomProjection","cdPomNormalMatrix","cdPomViewToLocal"])close(a.uniforms[key].value.toArray(),b.uniforms[key].value.toArray());
const shadowCamera=new THREE.OrthographicCamera(-4,4,4,-4,.1,30);shadowCamera.position.set(-2,5,3);shadowCamera.lookAt(0,0,0);shadowCamera.updateMatrixWorld();for(const mesh of [newMesh,oldMesh])mesh.onBeforeShadow(null,null,null,shadowCamera);assert.equal(a.uniforms.cdPomOrthographic.value,true);close(a.uniforms.cdPomCameraLocal.value.toArray(),b.uniforms.cdPomCameraLocal.value.toArray());
let shadowDisposals=0;newMesh.customDepthMaterial.addEventListener("dispose",()=>shadowDisposals++);newMesh.customDistanceMaterial.addEventListener("dispose",()=>shadowDisposals++);newMesh.material.dispose();assert.equal(shadowDisposals,2);
for(const pistol of [false,true]){const mount=new THREE.Group();mount.position.set(.2,1.2,-.3);mount.rotation.set(.1,-.3,.4);assert.deepEqual(snapshot(sceneMakeSling(host,mount,pistol)),snapshot(oldStage.makeSling(mount,pistol)));}
// User-edited Blender asset stays byte-identical; its complete geometry and placement match.
const layout=JSON.parse(readFileSync(resolve(root,"apps/web/src/base/user-fitting-layout.json"),"utf8"));
const oldAuthored=await original("base/user-fitting-layout.ts",{THREE,layout});
const authoredMount=new THREE.Group(),originalMount=new THREE.Group();
const authored=sceneAuthoredWeaponFit(host,authoredMount,layout),authoredReference=oldAuthored.authoredWeaponFit(originalMount);
assert.deepEqual(snapshot(authored),snapshot(authoredReference));
authoredMount.updateMatrix();originalMount.updateMatrix();close(authoredMount.matrix.elements,originalMount.matrix.elements);close(authoredMount.position.toArray(),layout.weaponMatrix.slice(12,15));

// Hand frames and every muzzle retain the original real-Three result.
function persona(){const model=new THREE.Group();model.rotation.set(.2,.3,-.1);model.position.set(2,3,4);for(const side of ["l","r"]){const hand=new THREE.Bone();hand.name=`hand_${side}`;hand.position.set(.4,1,.1);model.add(hand);for(const[name,p]of[["middle_01",[.1,.03,0]],["index_01",[.08,.04,.02]],["pinky_01",[.08,-.02,-.02]]]){const bone=new THREE.Bone();bone.name=`${name}_${side}`;bone.position.fromArray(p);hand.add(bone);}}return model;}
for(const side of ["l","r"])close(sceneGripOf(host,persona(),side).elements,oldWeapons.gripOf(persona(),side).elements);assert.equal(sceneGripOf(host,new THREE.Group(),"r"),null);
const modelA=persona(),modelB=persona(),metal=new THREE.MeshStandardMaterial();const handA=new RustSceneHand(host,modelA,sceneGripOf(host,modelA,"r"),metal,{}),handB=new oldWeapons.Hand(modelB,oldWeapons.gripOf(modelB),metal);
for(const held of [null,"pistol","rifle","shotgun","sniper","knife","grenade"]){handA.hold(held);handB.hold(held);assert.deepEqual(snapshot(handA.mount),snapshot(handB.mount));const outA=new THREE.Vector3(),outB=new THREE.Vector3();assert.equal(handA.muzzle(outA),handB.muzzle(outB));close(outA.toArray(),outB.toArray());}handA.free();
const gun = new THREE.Group(), socket = new THREE.Group(); socket.name="socket_muzzle"; socket.position.x=.5; gun.add(socket);
const factory = new THREE.Mesh(new THREE.BoxGeometry(.2,.1,.1),metal); factory.name="factory_muzzle";factory.userData.factory=true;socket.add(factory);
const mod = new THREE.Mesh(new THREE.BoxGeometry(.6,.05,.05),metal);mod.name="muzzle_tishina";mod.userData={factory:false,slot:"muzzle",replaces:"factory_muzzle"};gun.add(mod);
const guns=await sceneLoadGuns({...host,loadModel:async()=>({scene:gun})});assert.equal(mod.parent,socket);
const realA=new RustSceneHand(host,persona(),new THREE.Matrix4(),metal,{guns}),realB=new oldWeapons.Hand(persona(),new THREE.Matrix4(),metal,{guns});
// The real pistol sits a size up and forward in the palm, which the original never did (HOLDS in scene_assets.rs).
{const pistol=realB.items.get("pistol");pistol.scale.setScalar(1.2);pistol.position.set(.03,.02,0);}
for(const mods of [{},{muzzle:"muzzle-tishina"},{}]){realA.fit(mods);realB.fit(mods);realA.hold("rifle");realB.hold("rifle");assert.deepEqual(snapshot(realA.mount),snapshot(realB.mount));const a=new THREE.Vector3(),b=new THREE.Vector3();assert.equal(realA.muzzle(a),realB.muzzle(b));close(a.toArray(),b.toArray());}realA.free();
function dressed(){const root=new THREE.Group(),g=new THREE.BufferGeometry();g.setAttribute("position",new THREE.Float32BufferAttribute([0,0,0,1,0,0,0,1,0,1,1,0,2,1,0,2,2,0],3));g.setAttribute("_suit",new THREE.Float32BufferAttribute([0,0,1,0,0,0],1));g.setIndex([0,1,2,3,4,5]);root.add(new THREE.Mesh(g,metal));for(const name of ["Suit","Vest","Helmet","Hair","Hair_Helmet"]){const mesh=new THREE.Mesh(new THREE.BoxGeometry(),metal);mesh.name=name;root.add(mesh);}return root;}
for(const wear of [{suit:true,body:true,head:true},{suit:true,body:false,head:false},{suit:false,body:false,head:true}]){const a=dressed(),b=dressed();sceneDress(host,a,wear);oldDress.dress(b,wear);assert.deepEqual(snapshot(a),snapshot(b));}

// Bytes are not damaged by base64 decoding, and optional guns may fail independently.
const savedFetch=globalThis.fetch;let parsed;globalThis.fetch=async()=>({ok:true,json:async()=>({data:Buffer.from([0,127,128,255]).toString("base64")})});
class GLTFLoader{setMeshoptDecoder(v){assert.equal(v,"decoder");return this;}async parseAsync(bytes,path){parsed=[...new Uint8Array(bytes)];assert.equal(path,"");return{scene:new THREE.Group()};}}
await sceneLoadModel({...host,GLTFLoader,MeshoptDecoder:"decoder"},"other.json");assert.deepEqual(parsed,[0,127,128,255]);assert.deepEqual(await sceneLoadGuns({...host,loadModel:async()=>{throw Error("missing");}}),{});globalThis.fetch=savedFetch;

// Late loads after disposal and a successful half of a failed pair are released.
function deferred(){let resolve,reject;const promise=new Promise((a,b)=>{resolve=a;reject=b;});return{promise,resolve,reject};}
const pending=[],renderers=[];class TextureLoader{loadAsync(url){const d={...deferred(),url};pending.push(d);return d.promise;}}
class Renderer{constructor(){this.renders=0;this.disposals=0;renderers.push(this);}setPixelRatio(v){this.ratio=v;}setSize(w,h){this.size=[w,h];}render(){this.renders++;}dispose(){this.disposals++;}}
const backdropHost={...host,THREE:{...THREE,TextureLoader,WebGLRenderer:Renderer}},flush=()=>new Promise(resolve=>setImmediate(resolve));
let disposed=0;const tex=()=>{const t=new THREE.Texture();t.addEventListener("dispose",()=>disposed++);return t;};
const entry={eligible:true,relief:"tex/relief/height.png",depthMeters:.0004,midpoint:.5,resolution:[1024,1024],uvSet:0};
globalThis.fetch=async()=>({ok:true,json:async()=>({version:1,models:{varyag:{unitsPerMeter:100,materials:{steel:entry}}}})});
const reliefMaterial=()=>{const m=new THREE.MeshStandardMaterial({normalMap:new THREE.Texture()});m.name="steel";return m;};
const staleMaterial=reliefMaterial(),staleRoot=new THREE.Mesh(new THREE.BoxGeometry(),staleMaterial);
const stale=sceneInstallRelief({...host,THREE:{...THREE,TextureLoader},baseUrl:"/"},staleRoot,"models/varyag.json");await flush();staleMaterial.dispose();pending.shift().resolve(tex());await stale;assert.equal(staleMaterial.bumpMap,null);assert.equal(disposed,1);
const activeMaterial=reliefMaterial(),activeRoot=new THREE.Mesh(new THREE.BoxGeometry(),activeMaterial);
const active=sceneInstallRelief({...host,THREE:{...THREE,TextureLoader},baseUrl:"/"},activeRoot,"models/varyag.json");await flush();pending.shift().resolve(tex());await active;assert.ok(activeMaterial.bumpMap);assert.equal(activeMaterial.userData.cdSurfaceRelief.depthUnits,.04);activeMaterial.bumpMap.dispose();assert.equal(disposed,2);disposed=0;globalThis.fetch=savedFetch;
const early=new RustDepthBackdrop(backdropHost,{clientWidth:400,clientHeight:700},"a","b",9/16,1.1);await flush();early.dispose();early.free();pending.splice(0).forEach(p=>p.resolve(tex()));await flush();assert.equal(disposed,2);assert.equal(renderers[0].renders,0);
const failure=new RustDepthBackdrop(backdropHost,{clientWidth:400,clientHeight:700},"a","b",9/16,1.1);const failed=assert.rejects(failure.ready,/unavailable/);await flush();pending.shift().resolve(tex());pending.shift().reject(Error("unavailable"));await failed;assert.equal(disposed,3);failure.free();
const backdrop=new RustDepthBackdrop(backdropHost,{clientWidth:400,clientHeight:700},"a","b",9/16,1.1);await flush();pending.splice(0).forEach(p=>p.resolve(tex()));await backdrop.ready;backdrop.draw(2,3);backdrop.draw(2,3);assert.equal(renderers[2].renders,2);backdrop.dispose();backdrop.draw(5,6);assert.equal(renderers[2].renders,2);backdrop.free();assert.equal(disposed,5);assert.equal(renderers[2].disposals,1);
console.log("Rust scene parity passed: podium/POM/shadows, sling, grip/weapons, dress, binary loading and backdrop lifetimes.");

