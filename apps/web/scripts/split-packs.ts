// Cuts the beard and hairstyle packs into one small file per piece, so a Persona downloads only
// the beard of its own face and style and only its own hairstyle (beards.rs loads them on demand).
// The whole packs stay the source of truth in apps/web/packs/ (tools/beards, tools/hair-hd write
// them); this writes the pieces into apps/web/models/, where the build, the KTX2 copies
// (npm run models:ktx2) and the gzip copies pick them up like any other model.
//
//   npm run models:packs -w @crowned-dogs/web
//
// beards-male.glb   -> beard-m<face>-<style>.glb     (Beard_m<face>_<style>_cards + _root)
// hair-male.glb     -> hair-male-<style>.glb         (Hair_<style>_*, the head mask Hair_scalp, its Hair_line_<style>)
// hair-female.glb   -> hair-female-<style>.glb       (Hair_<style>_*)

import { readdirSync, rmSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { type Document, NodeIO } from "@gltf-transform/core";
import { ALL_EXTENSIONS } from "@gltf-transform/extensions";
import { MeshoptDecoder, MeshoptEncoder } from "meshoptimizer";

const PACKS = fileURLToPath(new URL("../packs", import.meta.url));
const MODELS = fileURLToPath(new URL("../models", import.meta.url));

await Promise.all([MeshoptDecoder.ready, MeshoptEncoder.ready]);
const io = new NodeIO().registerExtensions(ALL_EXTENSIONS)
  .registerDependencies({ "meshopt.decoder": MeshoptDecoder, "meshopt.encoder": MeshoptEncoder });

// old pieces go first, so a renamed or dropped style leaves nothing behind
for (const f of readdirSync(MODELS)) {
  if (/^(beard-m\d-|hair-(male|female)-)[a-z]+\.glb$/.test(f)) rmSync(join(MODELS, f));
}

async function cut(pack: string, out: string, keep: (name: string) => boolean): Promise<void> {
  const document = await io.read(join(PACKS, pack));
  const root = document.getRoot();
  let kept = 0;
  for (const node of root.listNodes()) {
    if (keep(node.getName())) kept++;
    else node.dispose();
  }
  if (!kept) throw new Error(`${pack}: no meshes for ${out}`);
  drop(document);
  await io.write(join(MODELS, out), document);
}

// Whatever no kept node reaches any more goes: meshes, then their materials and accessors, then textures.
function drop(document: Document): void {
  const root = document.getRoot();
  const orphan = (p: { listParents(): { propertyType: string }[] }) => p.listParents().every((x) => x.propertyType === "Root");
  for (const mesh of root.listMeshes()) {
    if (!orphan(mesh)) continue;
    for (const primitive of mesh.listPrimitives()) {
      for (const target of primitive.listTargets()) target.dispose();
      primitive.dispose();
    }
    mesh.dispose();
  }
  for (const material of root.listMaterials()) if (orphan(material)) material.dispose();
  for (const accessor of root.listAccessors()) if (orphan(accessor)) accessor.dispose();
  for (const texture of root.listTextures()) if (orphan(texture)) texture.dispose();
}

const names = async (pack: string) => (await io.read(join(PACKS, pack))).getRoot().listNodes().map((n) => n.getName());

for (const name of new Set((await names("beards-male.glb")).map((n) => n.match(/^Beard_(m\d)_([a-z]+)_/)?.slice(1, 3).join("-")).filter(Boolean))) {
  const [face, style] = name!.split("-");
  await cut("beards-male.glb", `beard-${face}-${style}.glb`, (n) => n.startsWith(`Beard_${face}_${style}_`));
}
for (const sex of ["male", "female"]) {
  const pack = `hair-${sex}.glb`;
  const styles = new Set((await names(pack)).map((n) => n.match(/^Hair_([a-z]+)_cards$/)?.[1]).filter(Boolean));
  for (const style of styles) {
    await cut(pack, `hair-${sex}-${style}.glb`, (n) => n.startsWith(`Hair_${style}_`) || n === "Hair_scalp" || n === `Hair_line_${style}`);
  }
}
console.log(readdirSync(MODELS).filter((f) => /^(beard-m\d-|hair-(male|female)-)/.test(f)).join(" "));
