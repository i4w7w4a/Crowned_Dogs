// Puts a Tripo item preview (knife, grenade) into the hand mount frame: x forward, y up, z across.
// usage: node held_item.mjs IN.glb OUT.glb NAME SCALE TURN_Z TX TY TZ   (knife: 0.27 1.5707963 0.0216 0.066 0; grenade: 0.11 0 0.0083 -0.042 0)
import { NodeIO } from '@gltf-transform/core';
import { ALL_EXTENSIONS } from '@gltf-transform/extensions';
import { MeshoptDecoder, MeshoptEncoder } from 'meshoptimizer';
await MeshoptDecoder.ready; await MeshoptEncoder.ready;
const io = new NodeIO().registerExtensions(ALL_EXTENSIONS).registerDependencies({ 'meshopt.decoder': MeshoptDecoder, 'meshopt.encoder': MeshoptEncoder });
const [src, out, name, s, rz, tx, ty, tz] = process.argv.slice(2);
const doc = await io.read(src);
const scene = doc.getRoot().listScenes()[0];
const root = doc.createNode(name).setScale([+s, +s, +s]).setTranslation([+tx, +ty, +tz])
  .setRotation([0, 0, Math.sin(+rz / 2), Math.cos(+rz / 2)]);
for (const n of scene.listChildren()) { scene.removeChild(n); root.addChild(n); }
scene.addChild(root);
await io.write(out, doc);
console.log('wrote', out);
