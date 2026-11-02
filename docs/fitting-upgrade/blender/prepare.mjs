// Build an editable copy from the exact runtime pose; no browser or source asset writes.
import fs from 'node:fs/promises';
import path from 'node:path';
import * as THREE from 'three';
import { GLTFLoader } from 'three/addons/loaders/GLTFLoader.js';
import { MeshoptDecoder } from 'three/addons/libs/meshopt_decoder.module.js';
import { dress, KIT_A } from '../../../apps/web/src/arena/dress.ts';
import { relaxArms, makeSling, prepareVaryag, fitVaryag, PLATFORM_TOP } from '../../../apps/web/src/base/fitting-stage.ts';

const out = path.resolve(process.argv[2]);
await fs.mkdir(out, { recursive: true });
await MeshoptDecoder.ready;

function pack(json, binary) {
  const text = Buffer.from(JSON.stringify(json));
  const jlen = (text.length + 3) & ~3, blen = (binary.length + 3) & ~3;
  const glb = Buffer.alloc(28 + jlen + blen);
  glb.writeUInt32LE(0x46546c67, 0); glb.writeUInt32LE(2, 4); glb.writeUInt32LE(glb.length, 8);
  glb.writeUInt32LE(jlen, 12); glb.writeUInt32LE(0x4e4f534a, 16); glb.fill(32, 20, 20 + jlen); text.copy(glb, 20);
  glb.writeUInt32LE(blen, 20 + jlen); glb.writeUInt32LE(0x004e4942, 24 + jlen); binary.copy(glb, 28 + jlen);
  return glb;
}
async function readAsset(file, label) {
  let bytes = await fs.readFile(file);
  if (file.endsWith('.json')) bytes = Buffer.from(JSON.parse(bytes).data, 'base64');
  const length = bytes.readUInt32LE(12), json = JSON.parse(bytes.subarray(20, 20 + length));
  const binary = bytes.subarray(28 + length), blocks = []; let offset = 0;
  for (const view of json.bufferViews) {
    const ext = view.extensions?.EXT_meshopt_compression;
    let block;
    if (ext) {
      block = Buffer.alloc(ext.count * ext.byteStride);
      MeshoptDecoder.decodeGltfBuffer(block, ext.count, ext.byteStride, binary.subarray(ext.byteOffset ?? 0, (ext.byteOffset ?? 0) + ext.byteLength), ext.mode, ext.filter ?? 'NONE');
      delete view.extensions.EXT_meshopt_compression;
      if (!Object.keys(view.extensions).length) delete view.extensions;
    } else block = binary.subarray(view.byteOffset ?? 0, (view.byteOffset ?? 0) + view.byteLength);
    view.buffer = 0; view.byteOffset = offset; view.byteLength = block.length;
    const padding = Buffer.alloc(((block.length + 3) & ~3) - block.length);
    blocks.push(block, padding); offset += block.length + padding.length;
  }
  json.buffers = [{ byteLength: offset }];
  for (const key of ['extensionsUsed', 'extensionsRequired']) if (json[key]) json[key] = json[key].filter(e => e !== 'EXT_meshopt_compression');
  json.animations = [];
  const bin = Buffer.concat(blocks);
  await fs.writeFile(path.join(out, `${label}-source.glb`), pack(json, bin));
  const geometryOnly = structuredClone(json);
  delete geometryOnly.images; delete geometryOnly.textures; delete geometryOnly.samplers;
  geometryOnly.materials = json.materials.map((m, i) => ({ name: m.name ?? `Material_${i}`, doubleSided: !!m.doubleSided }));
  const buffer = pack(geometryOnly, bin);
  const gltf = await new GLTFLoader().parseAsync(buffer.buffer.slice(buffer.byteOffset, buffer.byteOffset + buffer.length), '');
  return { scene: gltf.scene, source: `${label}-source.glb` };
}
function meshData(mesh) {
  const positions = [], uv = [], point = new THREE.Vector3();
  const geometry = mesh.geometry, vertices = geometry.getAttribute('position'), tex = geometry.getAttribute('uv');
  for (let i = 0; i < vertices.count; i++) {
    mesh.getVertexPosition(i, point).applyMatrix4(mesh.matrixWorld); positions.push(...point.toArray());
    if (tex) uv.push(tex.getX(i), tex.getY(i));
  }
  return { name: mesh.name, material: mesh.material.name, positions, uv, indices: geometry.index ? [...geometry.index.array] : Array.from({length: vertices.count}, (_, i) => i) };
}
function meshes(root) {
  root.updateMatrixWorld(true);
  root.traverse(o => { if (o.isSkinnedMesh) o.skeleton.update(); });
  const result = []; root.traverseVisible(o => { if (o.isMesh) result.push(meshData(o)); }); return result;
}
const figure = await readAsset('apps/web/models/persona-male.glb', 'persona');
dress(figure.scene, KIT_A); relaxArms(figure.scene); figure.scene.updateMatrixWorld(true);
let bounds = new THREE.Box3().setFromObject(figure.scene, true);
figure.scene.scale.multiplyScalar(1.8 / Math.max(.1, bounds.max.y - bounds.min.y));
figure.scene.updateMatrixWorld(true); bounds = new THREE.Box3().setFromObject(figure.scene, true);
const centre = bounds.getCenter(new THREE.Vector3());
figure.scene.position.set(-centre.x, PLATFORM_TOP - bounds.min.y, -centre.z);
const gun = await readAsset('apps/web/public/preview/varyag.json', 'varyag');
prepareVaryag(gun.scene); fitVaryag(gun.scene, null);
const gunMeshes = meshes(gun.scene);
const mount = new THREE.Group(); mount.position.set(-.035, 1.18, .215); mount.rotation.set(0, 0, -.96);
const sling = makeSling(mount);
sling.position.y = PLATFORM_TOP; sling.updateMatrixWorld(true);
const slingMeshes = meshes(sling);
const clips = sling.children.map(clip => ({ name: clip.name, position: clip.getWorldPosition(new THREE.Vector3()).toArray(), rotation: clip.getWorldQuaternion(new THREE.Quaternion()).toArray() }));
const data = {
  unit: 'metre', coordinates: 'three: X right, Y up, Z towards camera; blender: X, -Z, Y',
  sources: [figure.source, gun.source], persona: meshes(figure.scene), gun: gunMeshes,
  mount: { position: [-.035, 1.18 + PLATFORM_TOP, .215], rotation: mount.quaternion.toArray() },
  sling: slingMeshes, clips,
};
await fs.writeFile(path.join(out, 'fitting-layout.json'), JSON.stringify(data));
console.log(JSON.stringify({ directory: out, personaMeshes: data.persona.length, gunMeshes: data.gun.length, clips: clips.length }));
