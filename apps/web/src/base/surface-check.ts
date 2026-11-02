// Standalone, silent development comparison; excluded from production Vite entry.
import * as THREE from "three";
import { RoomEnvironment } from "three/addons/environments/RoomEnvironment.js";
import { loadModel } from "../arena/load-model.ts";
import { tune } from "../arena/materials.ts";
import { QUALITY } from "../arena/quality.ts";
import { setSurfaceReliefPreview } from "../arena/surface-relief.ts";
import { callRust } from "../../../../packages/runtime/src/index.ts";

document.head.insertAdjacentHTML("beforeend", "<style>body{margin:0;background:#101315;color:#dce1e3;font:14px system-ui}main{max-width:1200px;margin:auto;padding:20px}h1{font-size:20px}nav{display:flex;gap:8px;flex-wrap:wrap;align-items:center}button,select{padding:9px 12px;color:inherit;background:#242b2e;border:1px solid #4e5a5f;border-radius:8px}canvas{display:block;width:100%;height:500px;margin-top:14px;border:1px solid #3e484b;border-radius:12px;background:#181c1f}.pair-labels{display:grid;grid-template-columns:1fr 1fr;background:#4e5a5f;gap:1px;margin-top:14px}.pair-labels[hidden]{display:none}.pair-labels span{background:#181c1f;padding:12px;text-align:center;font-weight:bold}.pair-labels:not([hidden])+canvas{margin-top:0;border-radius:0 0 12px 12px}pre{white-space:pre-wrap;font-size:12px}</style>");
const canvas = document.querySelector<HTMLCanvasElement>("#surface")!;
const status = document.querySelector<HTMLPreElement>("#result")!;
const selector = document.querySelector<HTMLSelectElement>("#model")!;
const renderer = new THREE.WebGLRenderer({ canvas, alpha: true, antialias: true });
renderer.outputColorSpace = THREE.SRGBColorSpace; renderer.toneMapping = THREE.ACESFilmicToneMapping;
renderer.setClearColor(0x15191c, 0);
const errors: string[] = [];
renderer.debug.onShaderError = (gl, program, vertex, fragment) => {
  errors.push([gl.getProgramInfoLog(program), gl.getShaderInfoLog(vertex), gl.getShaderInfoLog(fragment)].filter(Boolean).join("\n"));
  status.textContent = errors.join("\n");
};
const scene = new THREE.Scene();
const room=new RoomEnvironment(), pmrem=new THREE.PMREMGenerator(renderer);
scene.environment=pmrem.fromScene(room,0.04).texture;room.dispose();pmrem.dispose();
scene.add(new THREE.HemisphereLight(0xd9e5ed, 0x303437, 1.0));
for (const [x, y, z, power] of [[-3, 1.5, 1, 3.6], [3, 1, 2, 0.7], [1, 2, -3, 1.4]]) {
  const light = new THREE.DirectionalLight(0xe9eef2, power); light.position.set(x!, y!, z!); scene.add(light);
}
const camera = new THREE.PerspectiveCamera(33, 1, .01, 20); camera.position.set(0, 0, 3.8);
const pivot = new THREE.Group(); scene.add(pivot);
let root: THREE.Object3D | undefined, enabled = true, serial = 0, paired = false;
let loaded = "";
function materials(): THREE.Material[] {
  const all = new Set<THREE.Material>();
  root?.traverse(node => { if (node instanceof THREE.Mesh) for (const mat of Array.isArray(node.material) ? node.material : [node.material]) all.add(mat); });
  return [...all];
}
function apply(value: boolean): void {
  enabled = value; setSurfaceReliefPreview(value);
  for (const material of materials()) tune(material, QUALITY.ultra);
}
function draw(): void {
  renderer.setPixelRatio(Math.min(devicePixelRatio, 2)); renderer.setSize(canvas.clientWidth, canvas.clientHeight, false);
  camera.position.z = 3.8 / Number(document.querySelector<HTMLSelectElement>("#zoom")!.value);
  camera.aspect = canvas.clientWidth / (paired ? 2 : 1) / canvas.clientHeight; camera.updateProjectionMatrix();
  document.querySelector<HTMLElement>(".pair-labels")!.hidden = !paired;
  renderer.setScissorTest(paired);
  if (paired) {
    const saved = enabled, half = canvas.clientWidth / 2, height = canvas.clientHeight;
    for (const [index, value] of [false, true].entries()) {
      renderer.setViewport(index * half, 0, half, height); renderer.setScissor(index * half, 0, half, height);
      apply(value); renderer.render(scene, camera);
    }
    apply(saved);
  } else { renderer.setViewport(0, 0, canvas.clientWidth, canvas.clientHeight); renderer.render(scene, camera); }
  if (!errors.length) status.textContent = JSON.stringify({ model: selector.value, relief: enabled, materials: materials().map(m => ({ name: m.name, height: m.userData.cdSurfaceRelief ?? null })), shaderErrors: errors }, null, 2);
}
async function show(): Promise<void> {
  const request = ++serial; status.textContent = "Загрузка…";
  try {
    const requested = selector.value;
    const result = requested === "control" ? { scene: makeControl() } : await loadModel(`${import.meta.env.BASE_URL}preview/${requested}.json`);
    if (request !== serial) return;
    if (root) dispose(root);
    root = result.scene; loaded = requested; pivot.clear(); pivot.rotation.set(.1, -.25, 0); pivot.add(root);
    root.updateMatrixWorld(true);
    const bounds = new THREE.Box3().setFromObject(root), centre = bounds.getCenter(new THREE.Vector3()), size = bounds.getSize(new THREE.Vector3());
    const scale = 1.8 / Math.max(size.x, size.y, size.z);
    root.scale.setScalar(scale); root.position.copy(centre).multiplyScalar(-scale);
    apply(enabled); draw();
  } catch (error) { status.textContent = String(error); }
}
function verify(): void {
  if (loaded !== selector.value) {status.textContent="Дождитесь загрузки модели";return;}
  if (loaded === "control") { verifyControl(); return; }
  const target = new THREE.WebGLRenderTarget(768, 768), before = new Uint8Array(768 * 768 * 4), after = new Uint8Array(before.length);
  const previous = enabled;
  renderer.setScissorTest(false); camera.aspect = 1; camera.updateProjectionMatrix(); renderer.setRenderTarget(target);
  apply(false); renderer.render(scene, camera); renderer.readRenderTargetPixels(target, 0, 0, 768, 768, before);
  apply(true); renderer.render(scene, camera); renderer.readRenderTargetPixels(target, 0, 0, 768, 768, after);
  let changed = 0, maxDifference = 0, silhouetteMismatch = 0;
  for (let i = 0; i < before.length; i += 4) {
    if (before[i + 3] !== after[i + 3]) silhouetteMismatch++;
    const difference = Math.max(...[0, 1, 2].map(c => Math.abs(before[i + c]! - after[i + c]!)));
    if (difference > 0) changed++;
    maxDifference = Math.max(maxDifference, difference);
  }
  renderer.setRenderTarget(null); target.dispose(); apply(previous); draw();
  const configured = materials().filter(m => m.userData.cdSurfaceRelief).length;
  const result = { model: selector.value, configured, changedPixels: changed, maxChannelDifference: maxDifference, silhouetteMismatch, shaderErrors: errors,
    passed: configured > 0 && changed > 0 && silhouetteMismatch === 0 && errors.length === 0 };
  status.textContent = JSON.stringify(result, null, 2); status.dataset.passed = String(result.passed);
}
function dispose(object: THREE.Object3D): void {
  const textures = new Set<THREE.Texture>(), mats = new Set<THREE.Material>();
  object.traverse(node => {
    if (!(node instanceof THREE.Mesh)) return;
    node.geometry.dispose(); for (const mat of Array.isArray(node.material) ? node.material : [node.material]) mats.add(mat);
  });
  for (const mat of mats) { for (const value of Object.values(mat)) if (value instanceof THREE.Texture) textures.add(value); mat.dispose(); }
  for (const texture of textures) texture.dispose();
}
selector.addEventListener("change", () => void show());
document.querySelector("#original")!.addEventListener("click", () => { paired=false;apply(false); draw(); });
document.querySelector("#relief")!.addEventListener("click", () => { paired=false;apply(true); draw(); });
document.querySelector("#compare")!.addEventListener("click", () => { paired=true;draw(); });
document.querySelector("#zoom")!.addEventListener("change", draw);
document.querySelector("#probe")!.addEventListener("click", probeUv);
document.querySelector("#turn")!.addEventListener("click", () => { pivot.rotation.y += .4; draw(); });
document.querySelector("#grazing")!.addEventListener("click", () => { pivot.rotation.set(.05, 1.35, .1); draw(); });
document.querySelector("#verify")!.addEventListener("click", verify);
window.addEventListener("resize", draw);
void show();

function probeUv():void {
  const replacements:{mesh:THREE.Mesh;material:THREE.Material|THREE.Material[]}[]=[];
  const probe=new THREE.ShaderMaterial({vertexShader:"varying vec2 testUv;void main(){testUv=uv;gl_Position=projectionMatrix*modelViewMatrix*vec4(position,1.);}",fragmentShader:"varying vec2 testUv;void main(){gl_FragColor=vec4(testUv,0.,1.);}",side:THREE.DoubleSide});
  root?.traverse(node=>{if(node instanceof THREE.Mesh){replacements.push({mesh:node,material:node.material});node.material=probe;}});
  const target=new THREE.WebGLRenderTarget(512,512),pixels=new Uint8Array(512*512*4);
  renderer.setScissorTest(false);camera.aspect=1;camera.updateProjectionMatrix();renderer.setRenderTarget(target);renderer.render(scene,camera);renderer.readRenderTargetPixels(target,0,0,512,512,pixels);renderer.setRenderTarget(null);
  for(const {mesh,material}of replacements)mesh.material=material;
  probe.dispose();target.dispose();draw();
  const samples=[];
  for(const y of [.3,.5,.7])for(const x of [.3,.5,.7]){const i=(Math.round(y*511)*512+Math.round(x*511))*4;samples.push({screen:[x,y],atlas:[Math.round(pixels[i]!/255*1024),Math.round(pixels[i+1]!/255*1024)]});}
  status.textContent=JSON.stringify({model:loaded,samples},null,2);
}

function controlMap(): THREE.DataTexture {
  const n=256, pixels=new Uint8Array(n*n*4);
  for(let y=0;y<n;y++) for(let x=0;x<n;x++) {
    const h=.5+.22*Math.sin((x+.5)/n*Math.PI*6)*Math.cos((y+.5)/n*Math.PI*4), i=(y*n+x)*4;
    pixels[i]=pixels[i+2]=Math.round(h*255);pixels[i+1]=pixels[i+3]=255;
  }
  const map=new THREE.DataTexture(pixels,n,n);map.minFilter=map.magFilter=THREE.LinearFilter;map.needsUpdate=true;return map;
}
function makeControl(): THREE.Group {
  const normal=new THREE.DataTexture(new Uint8Array([128,128,255,255]),1,1);normal.needsUpdate=true;
  const material=new THREE.MeshStandardMaterial({color:0x919aa0,roughness:.52,metalness:.15,normalMap:normal,bumpMap:controlMap(),bumpScale:0,side:THREE.DoubleSide});
  material.userData.cdSurfaceRelief={depthUnits:.09,detailDepthUnits:.09,midpoint:128/255,source:"analytic-control"};
  const group=new THREE.Group();group.add(new THREE.Mesh(new THREE.PlaneGeometry(1.4,1.4),material));return group;
}
function verifyControl(): void {
  // Compare the same height texture ray-marched on two triangles and displaced
  // on a dense reference mesh. Encoded UV colour measures intersection accuracy.
  const map=controlMap(), common=callRust<{common:string}>("graphics.surfacePom",{quality:"ultra"}).common;
  const uniforms={height:{value:map},depth:{value:.09},cdSurfaceHeight:{value:map},cdSurfaceDepth:{value:.09},cdSurfaceDetailDepth:{value:0},cdSurfaceFabricDepth:{value:0},cdSurfaceFabricPeriod:{value:0},cdSurfaceMidpoint:{value:128/255},cdSurfaceSteps:{value:64},cdSurfaceRefine:{value:7},cdSurfaceShadowSteps:{value:16},testLight:{value:new THREE.Vector3(0,0,1)}};
  const vertex=(displaced:boolean)=>`uniform sampler2D height;uniform float depth;varying vec2 testUv;varying vec3 testEye;varying float cdSurfaceScale;varying vec3 cdSurfaceLocal;void main(){testUv=uv;cdSurfaceLocal=position;vec3 p=position;${displaced?'p.z+=(texture2D(height,uv).r-128.0/255.0)*depth;':''} vec4 view=modelViewMatrix*vec4(p,1.);testEye=-view.xyz;cdSurfaceScale=1.;gl_Position=projectionMatrix*view;}`;
  const fragment=(pom:boolean)=>`precision highp float;varying vec2 testUv;varying vec3 testEye;${pom?common:''}void main(){vec2 uv=testUv;${pom?'uv=cdSurfaceReliefUv(uv,dFdx(uv),dFdy(uv),-testEye,testEye);':''} gl_FragColor=vec4(uv,.5,1.);}`;
  const flat=new THREE.Mesh(new THREE.PlaneGeometry(1.4,1.4),new THREE.ShaderMaterial({uniforms,vertexShader:vertex(false),fragmentShader:fragment(true),side:THREE.DoubleSide}));
  const dense=new THREE.Mesh(new THREE.PlaneGeometry(1.4,1.4,256,256),new THREE.ShaderMaterial({uniforms,vertexShader:vertex(true),fragmentShader:fragment(false),side:THREE.DoubleSide}));
  const target=new THREE.WebGLRenderTarget(384,384), a=new Uint8Array(384*384*4), b=new Uint8Array(a.length);
  const checkScene=new THREE.Scene(), checkCamera=new THREE.PerspectiveCamera(40,1,.01,10);
  const results:{angle:number;pixels:number;meanUvError:number;badFraction:number}[]=[];
  renderer.setScissorTest(false);renderer.setRenderTarget(target);renderer.setClearColor(0,0);
  for(const angle of [0,35,65]) {
    const theta=angle*Math.PI/180;checkCamera.position.set(Math.sin(theta)*2,.18,Math.cos(theta)*2);checkCamera.lookAt(0,0,0);checkCamera.updateMatrixWorld();
    checkScene.clear();checkScene.add(flat);renderer.render(checkScene,checkCamera);renderer.readRenderTargetPixels(target,0,0,384,384,a);
    checkScene.clear();checkScene.add(dense);renderer.render(checkScene,checkCamera);renderer.readRenderTargetPixels(target,0,0,384,384,b);
    let pixels=0,error=0,bad=0;
    for(let i=0;i<a.length;i+=4) if(a[i+3]===255&&b[i+3]===255&&b[i]!>25&&b[i]!<230&&b[i+1]!>25&&b[i+1]!<230){
      const difference=Math.max(Math.abs(a[i]!-b[i]!),Math.abs(a[i+1]!-b[i+1]!));error+=difference/255;bad+=Number(difference>3);pixels++;
    }
    results.push({angle,pixels,meanUvError:error/Math.max(1,pixels),badFraction:bad/Math.max(1,pixels)});
  }
  const uvMaterial=flat.material;
  flat.material=new THREE.ShaderMaterial({uniforms,vertexShader:vertex(false),fragmentShader:`precision highp float;varying vec2 testUv;varying vec3 testEye;uniform vec3 testLight;${common}void main(){cdSurfaceReliefUv(testUv,dFdx(testUv),dFdy(testUv),-testEye,testEye);gl_FragColor=vec4(vec3(cdSurfaceLightVisibility(testLight)),1.);}`,side:THREE.DoubleSide});
  checkCamera.position.set(.8,.18,1.6);checkCamera.lookAt(0,0,0);checkCamera.updateMatrixWorld();
  checkScene.clear();checkScene.add(flat);
  const shadows=[];let previousShadow:Uint8Array|undefined;
  for(const direction of [new THREE.Vector3(0,0,1),new THREE.Vector3(.98,0,.16),new THREE.Vector3(-.98,0,.16)]) {
    uniforms.testLight.value.copy(direction).transformDirection(checkCamera.matrixWorldInverse);
    renderer.render(checkScene,checkCamera);renderer.readRenderTargetPixels(target,0,0,384,384,a);
    let covered=0,dark=0,changed=0;
    for(let i=0;i<a.length;i+=4) if(a[i+3]===255){covered++;dark+=Number(a[i]!<200);if(previousShadow&&Math.abs(a[i]!-previousShadow[i]!)>8)changed++;}
    shadows.push({direction:direction.toArray(),covered,dark,changed});previousShadow=a.slice();
  }
  renderer.setRenderTarget(null);renderer.setClearColor(0x15191c,0);target.dispose();flat.geometry.dispose();dense.geometry.dispose();flat.material.dispose();uvMaterial.dispose();dense.material.dispose();map.dispose();draw();
  const passed=errors.length===0&&results.every(r=>r.pixels>1000&&r.meanUvError<.004&&r.badFraction<.03)&&shadows[0]!.dark===0&&shadows[1]!.dark>1000&&shadows[2]!.changed>1000;
  status.textContent=JSON.stringify({control:"POM UV versus displaced reference mesh; per-light self-shadow",results,shadows,shaderErrors:errors,passed},null,2);status.dataset.passed=String(passed);
}
