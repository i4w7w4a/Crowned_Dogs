// Standalone, silent development comparison; excluded from production Vite entry.
import * as THREE from "three";
import { loadModel } from "../arena/load-model.ts";
import { tune } from "../arena/materials.ts";
import { QUALITY } from "../arena/quality.ts";
import { setSurfaceReliefPreview } from "../arena/surface-relief.ts";

document.head.insertAdjacentHTML("beforeend", "<style>body{margin:0;background:#101315;color:#dce1e3;font:14px system-ui}main{max-width:1200px;margin:auto;padding:20px}h1{font-size:20px}nav{display:flex;gap:8px;flex-wrap:wrap}button,select{padding:9px 12px;color:inherit;background:#242b2e;border:1px solid #4e5a5f;border-radius:8px}canvas{display:block;width:100%;height:620px;margin-top:14px;border:1px solid #3e484b;border-radius:12px;background:#181c1f}pre{white-space:pre-wrap;font-size:12px}</style>");
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
scene.add(new THREE.HemisphereLight(0xd9e5ed, 0x303437, 2));
for (const [x, y, z, power] of [[-2, 3, 3, 3.6], [3, 1, 2, 1.4], [1, 2, -3, 2.8]]) {
  const light = new THREE.DirectionalLight(0xe9eef2, power); light.position.set(x!, y!, z!); scene.add(light);
}
const camera = new THREE.PerspectiveCamera(33, 1, .01, 20); camera.position.set(0, 0, 3.8);
const pivot = new THREE.Group(); scene.add(pivot);
let root: THREE.Object3D | undefined, enabled = true, serial = 0;
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
  camera.aspect = canvas.clientWidth / canvas.clientHeight; camera.updateProjectionMatrix();
  renderer.render(scene, camera);
  if (!errors.length) status.textContent = JSON.stringify({ model: selector.value, relief: enabled, materials: materials().map(m => ({ name: m.name, height: m.userData.cdSurfaceRelief ?? null })), shaderErrors: errors }, null, 2);
}
async function show(): Promise<void> {
  const request = ++serial; status.textContent = "Загрузка…";
  try {
    const result = await loadModel(`${import.meta.env.BASE_URL}preview/${selector.value}.json`);
    if (request !== serial) return;
    if (root) dispose(root);
    root = result.scene; pivot.clear(); pivot.rotation.set(.1, -.25, 0); pivot.add(root);
    root.updateMatrixWorld(true);
    const bounds = new THREE.Box3().setFromObject(root), centre = bounds.getCenter(new THREE.Vector3()), size = bounds.getSize(new THREE.Vector3());
    const scale = 1.8 / Math.max(size.x, size.y, size.z);
    root.scale.setScalar(scale); root.position.copy(centre).multiplyScalar(-scale);
    apply(enabled); draw();
  } catch (error) { status.textContent = String(error); }
}
function verify(): void {
  const target = new THREE.WebGLRenderTarget(768, 768), before = new Uint8Array(768 * 768 * 4), after = new Uint8Array(before.length);
  const previous = enabled;
  camera.aspect = 1; camera.updateProjectionMatrix(); renderer.setRenderTarget(target);
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
document.querySelector("#original")!.addEventListener("click", () => { apply(false); draw(); });
document.querySelector("#relief")!.addEventListener("click", () => { apply(true); draw(); });
document.querySelector("#turn")!.addEventListener("click", () => { pivot.rotation.y += .4; draw(); });
document.querySelector("#grazing")!.addEventListener("click", () => { pivot.rotation.set(.05, 1.35, .1); draw(); });
document.querySelector("#verify")!.addEventListener("click", verify);
window.addEventListener("resize", draw);
void show();
