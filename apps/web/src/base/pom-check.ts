// Dev-only verification entry; not included in Vite's game production entry.
import * as THREE from "three";
import { silhouetteDeck, tuneFittingMaterial, turntableMaterial } from "./turntable-material.ts";
import { QUALITY, type QualityId } from "../arena/quality.ts";
import { engineVersion } from "../rust/engine.ts";

document.head.insertAdjacentHTML("beforeend", "<style>body{margin:0;background:#101315;color:#dce1e3;font:14px system-ui}main{max-width:1200px;margin:auto;padding:20px}h1{font-size:20px}nav{display:flex;gap:8px;flex-wrap:wrap}button,select{padding:9px 12px;color:inherit;background:#242b2e;border:1px solid #4e5a5f;border-radius:8px}canvas{display:block;width:100%;height:520px;margin-top:14px;border:1px solid #3e484b;border-radius:12px;background:#181c1f}pre{white-space:pre-wrap;font-size:12px}</style>");
const canvas = document.querySelector<HTMLCanvasElement>("#podium")!;
const status = document.querySelector<HTMLPreElement>("#result")!;
const renderer = new THREE.WebGLRenderer({ canvas, alpha: true, antialias: true });
renderer.outputColorSpace = THREE.SRGBColorSpace;
renderer.toneMapping = THREE.ACESFilmicToneMapping;
renderer.setClearColor(0x15191c, 0);
renderer.shadowMap.enabled = true;
renderer.shadowMap.type = THREE.PCFShadowMap;
const errors: string[] = [];
renderer.debug.onShaderError = (gl, program, vertex, fragment) => {
  errors.push([gl.getProgramInfoLog(program), gl.getShaderInfoLog(vertex), gl.getShaderInfoLog(fragment)].filter(Boolean).join("\n"));
  status.textContent = errors.join("\n");
};
const scene = new THREE.Scene();
scene.add(new THREE.HemisphereLight(0xd9e5ed, 0x36302a, 1.5));
const key = new THREE.DirectionalLight(0xf1f4f5, 4); key.position.set(-1.4, 1.8, 1.6); key.castShadow = true;
key.shadow.camera.left = key.shadow.camera.bottom = -1;
key.shadow.camera.right = key.shadow.camera.top = 1;
key.shadow.camera.near = .1; key.shadow.camera.far = 5; key.shadow.mapSize.set(2048, 2048);
scene.add(key);
const source = turntableMaterial();
const deck = silhouetteDeck(source, .745, .016); scene.add(deck.group);
const camera = new THREE.PerspectiveCamera(36, 1, .01, 10);
let quality: QualityId = "ultra";
const draw = (): void => {
  renderer.setPixelRatio(Math.min(devicePixelRatio, 2));
  renderer.setSize(canvas.clientWidth, canvas.clientHeight, false);
  camera.aspect = canvas.clientWidth / canvas.clientHeight; camera.updateProjectionMatrix();
  renderer.render(scene, camera);
  if (!errors.length) status.textContent = `${engineVersion}\nКачество: ${quality}; Silhouette POM: ${quality !== "low"}; ошибок шейдера: 0`;
};
const setQuality = (): void => {
  deck.setQuality(quality);
  deck.group.traverse((o) => { if (o instanceof THREE.Mesh) tuneFittingMaterial(o.material as THREE.Material, QUALITY[quality]); });
};
const view = (id: string): void => {
  camera.position.set(...(id === "grazing" ? [0, .045, 1.65] : id === "top" ? [0, 2.7, .001] : [0, .82, 1.9]) as [number, number, number]);
  camera.lookAt(0, 0, 0); camera.updateMatrixWorld(); draw();
};
for (const button of document.querySelectorAll<HTMLButtonElement>("[data-view]")) button.addEventListener("click", () => view(button.dataset.view!));
document.querySelector("#rotate")!.addEventListener("click", () => { deck.group.rotation.y += .35; draw(); });
document.querySelector<HTMLSelectElement>("#quality")!.addEventListener("change", (e) => { quality = (e.target as HTMLSelectElement).value as QualityId; setQuality(); draw(); });
window.addEventListener("resize", draw);

function unpack(pixel: Uint8Array): number {
  return pixel[0]! / 256 + pixel[1]! / 65536 + pixel[2]! / 16777216 + pixel[3]! / (255 * 16777216);
}
function verify(): void {
  quality = "ultra"; document.querySelector<HTMLSelectElement>("#quality")!.value = quality; setQuality();
  deck.group.rotation.set(0, 0, 0);
  const size = 129, middle = 64;
  const probe = new THREE.OrthographicCamera(-.76, .76, .76, -.76, .1, 2);
  probe.position.set(0, 1, 0); probe.up.set(0, 0, -1); probe.lookAt(0, 0, 0); probe.updateMatrixWorld();
  const target = new THREE.WebGLRenderTarget(size, size);
  target.depthTexture = new THREE.DepthTexture(size, size, THREE.UnsignedIntType);
  renderer.setRenderTarget(target); renderer.render(scene, probe);
  const pixel = new Uint8Array(4), corners: number[] = [];
  for (const [x, y] of [[0,0],[0,size-1],[size-1,0],[size-1,size-1]]) { renderer.readRenderTargetPixels(target, x!, y!, 1, 1, pixel); corners.push(pixel[3]!); }
  renderer.readRenderTargetPixels(target, middle, middle, 1, 1, pixel);
  const centreVisible = pixel[3] === 255;
  const packed = new THREE.WebGLRenderTarget(size, size);
  const packMaterial = new THREE.ShaderMaterial({
    uniforms: { depthImage: { value: target.depthTexture } }, depthTest: false, depthWrite: false,
    vertexShader: "varying vec2 vUv;void main(){vUv=uv;gl_Position=vec4(position.xy,0.,1.);}",
    fragmentShader: "#include <packing>\nuniform sampler2D depthImage;varying vec2 vUv;void main(){gl_FragColor=packDepthToRGBA(texture2D(depthImage,vUv).r);}",
  });
  const packScene = new THREE.Scene(), quad = new THREE.Mesh(new THREE.PlaneGeometry(2, 2), packMaterial); packScene.add(quad);
  renderer.setRenderTarget(packed); renderer.render(packScene, new THREE.Camera());
  renderer.readRenderTargetPixels(packed, middle, middle, 1, 1, pixel);
  const colorDepth = unpack(pixel);
  const mesh = deck.group.getObjectByName("Silhouette_POM_Finite_Volume") as THREE.Mesh;
  scene.overrideMaterial = mesh.customDepthMaterial!;
  renderer.setRenderTarget(target); renderer.render(scene, probe);
  renderer.readRenderTargetPixels(target, middle, middle, 1, 1, pixel);
  const shadowDepth = unpack(pixel); scene.overrideMaterial = null;
  const image = source.bumpMap!.image as { data: Uint8Array; width: number };
  const n = image.width, i = n / 2;
  const sample = (x: number, y: number) => image.data[(y * n + x) * 4]!;
  const height = ((sample(i-1,i-1)+sample(i,i-1)+sample(i-1,i)+sample(i,i)) / 4 / 255 * 2 - 1) * .008;
  const expectedDepth = (1 - height - .1) / 1.9, proxyDepth = (1 - .008 - .1) / 1.9;
  const result = {
    engine: engineVersion, shaderErrors: errors,
    silhouetteCornersEmpty: corners.every((a) => a === 0), centreVisible,
    depthMatchesRelief: Math.abs(colorDepth - expectedDepth) < .00003,
    depthDiffersFromProxy: Math.abs(colorDepth - proxyDepth) > .00001,
    colorAndShadowDepthAgree: Math.abs(colorDepth - shadowDepth) < .00003,
    actualDepth: colorDepth, shadowDepth, expectedDepth, proxyDepth,
  };
  renderer.setRenderTarget(null); target.dispose(); packed.dispose(); quad.geometry.dispose(); packMaterial.dispose();
  draw(); status.textContent = JSON.stringify(result, null, 2);
  status.dataset.passed = String(result.silhouetteCornersEmpty && centreVisible && result.depthMatchesRelief && result.depthDiffersFromProxy && result.colorAndShadowDepthAgree && !errors.length);
}
document.querySelector("#verify")!.addEventListener("click", verify);
setQuality(); view("close");
