// Makes each figure model's GPU-compressed copy for the phone qualities: the same .glb with
// its textures in KTX2 (Basis Universal). On the GPU a map then takes a quarter of the video
// memory or less, and the phone skips decoding WebP and building mipmaps. Every map goes to
// ETC1S, normal maps at top quality with the normal-map preset: UASTC normals look cleaner but
// came out ten times the size of the WebP (a 1K map ~1 MB against 75 KB). Colour maps are at top
// quality too: at 128 the camouflage took a purple cast on the phone qualities.
// The copies land in apps/web/.ktx2/ (scripts/ktx2-cache.ts); only changed models are encoded
// again. Several models encode at once, one process each; the encoder's own chatter is dropped.
//
//   npm run models:ktx2 -w @crowned-dogs/web             every model that has no current copy
//   npm run models:ktx2 -w @crowned-dogs/web -- knife     only the named ones

import { spawn } from "node:child_process";
import { mkdirSync, readdirSync, renameSync, rmSync, writeFileSync } from "node:fs";
import { availableParallelism } from "node:os";
import { basename, join } from "node:path";
import { fileURLToPath } from "node:url";
import { NodeIO, type Texture } from "@gltf-transform/core";
import { ALL_EXTENSIONS, KHRTextureBasisu } from "@gltf-transform/extensions";
import { encodeToKTX2 } from "ktx2-encoder";
import { MeshoptDecoder, MeshoptEncoder } from "meshoptimizer";
import sharp from "sharp";
import { KTX2_CACHE, ktx2Copy } from "./ktx2-cache.ts";

const MODELS = fileURLToPath(new URL("../models", import.meta.url));
const args = process.argv.slice(2);

if (args[0] === "--one") {
  await compress(join(MODELS, `${args[1]}.glb`), args[2]!);
} else {
  mkdirSync(KTX2_CACHE, { recursive: true });
  const names = readdirSync(MODELS).filter((f) => f.endsWith(".glb")).map((f) => f.replace(/\.glb$/, ""))
    .filter((name) => !args.length || args.includes(name));
  const cached = new Set(readdirSync(KTX2_CACHE));
  const todo = names.filter((name) => !cached.has(basename(ktx2Copy(join(MODELS, `${name}.glb`)))));
  // Copies of models that changed since are dropped.
  for (const file of cached) {
    const name = file.replace(/\.[0-9a-f]{12}\.glb$/, "");
    if (names.includes(name) && file !== basename(ktx2Copy(join(MODELS, `${name}.glb`)))) rmSync(join(KTX2_CACHE, file));
  }
  console.log(todo.length ? `сжимаю текстуры: ${todo.join(", ")}` : "сжатые копии моделей уже есть");
  const lanes = Math.max(1, Math.min(todo.length, availableParallelism() - 1, 6));
  let failed = false;
  const lane = async () => {
    for (let name = todo.shift(); name; name = todo.shift()) {
      const out = ktx2Copy(join(MODELS, `${name}.glb`));
      const code = await new Promise<number | null>((done) =>
        spawn(process.execPath, [fileURLToPath(import.meta.url), "--one", name, out], { stdio: ["ignore", "ignore", "inherit"] }).on("exit", done));
      if (code !== 0) { failed = true; console.error(`не сжалась модель ${name}`); }
    }
  };
  await Promise.all(Array.from({ length: lanes }, lane));
  if (failed) process.exitCode = 1;
}

async function compress(glb: string, out: string): Promise<void> {
  await Promise.all([MeshoptDecoder.ready, MeshoptEncoder.ready]);
  const io = new NodeIO().registerExtensions(ALL_EXTENSIONS)
    .registerDependencies({ "meshopt.decoder": MeshoptDecoder, "meshopt.encoder": MeshoptEncoder });
  const document = await io.read(glb);
  const root = document.getRoot();
  const imageDecoder = async (buffer: Uint8Array) => {
    const { data, info } = await sharp(buffer).ensureAlpha().raw().toBuffer({ resolveWithObject: true });
    return { data: new Uint8Array(data), width: info.width, height: info.height };
  };
  const slots = (texture: Texture) => texture.getGraph().listParentEdges(texture).filter((e) => e.getParent() !== root).map((e) => e.getName());
  const started = performance.now();
  let encoded = 0;
  for (const texture of root.listTextures()) {
    const image = texture.getImage();
    const used = slots(texture);
    if (!image || !used.length || !/^image\/(png|jpeg|webp)$/.test(texture.getMimeType())) continue;
    // The head masks on the male hair pack's hidden quads (tools/hair-hd/scalp_mask.py) stay PNG: each channel
    // is another face's mask, which ETC1S would bleed into the others, and the shader reads them without mipmaps.
    if (texture.listParents().some((p) => p.propertyType === "Material" && /^M_Hair_(Scalp|Line_)/.test(p.getName()))) continue;
    const normal = used.includes("normalTexture");
    const colour = used.some((slot) => /^(baseColor|emissive|sheenColor|specularColor)Texture$/.test(slot));
    const ktx2 = await encodeToKTX2(image, normal
      ? { isUASTC: false, qualityLevel: 255, compressionLevel: 1, isNormalMap: true, generateMipmap: true, imageDecoder }
      : { isUASTC: false, qualityLevel: colour ? 255 : 128, compressionLevel: 1, isPerceptual: colour, isSetKTX2SRGBTransferFunc: colour, generateMipmap: true, imageDecoder });
    texture.setImage(ktx2).setMimeType("image/ktx2");
    encoded++;
  }
  if (encoded) document.createExtension(KHRTextureBasisu).setRequired(true);
  writeFileSync(`${out}.part`, await io.writeBinary(document));
  renameSync(`${out}.part`, out);
  console.error(`${basename(glb)}: ${encoded} текстур за ${Math.round((performance.now() - started) / 1000)} с`);
}
