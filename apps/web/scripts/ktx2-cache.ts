// Where the GPU-compressed copies of the figure models live, and the key each copy is filed
// under: a hash of its source .glb and of the encoder settings, so a changed model or new
// settings mean a new copy. scripts/compress-models.ts fills the folder, vite.config.ts ships it.
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

/** Bump when the encoder settings change: every model is encoded again. */
export const KTX2_SETTINGS = "etc1s-colour-q255-q128-normals-q255-v2";
export const KTX2_CACHE = process.env.CD_KTX2_CACHE ?? fileURLToPath(new URL("../.ktx2", import.meta.url));

export function ktx2Copy(glb: string): string {
  const key = createHash("sha1").update(KTX2_SETTINGS).update(readFileSync(glb)).digest("hex").slice(0, 12);
  return join(KTX2_CACHE, `${glb.split(/[\\/]/).pop()!.replace(/\.glb$/, "")}.${key}.glb`);
}
