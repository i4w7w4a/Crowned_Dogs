// Puts the figure model the game needs but git does not keep into public/models/:
// the placeholder mannequin (three.js example "X Bot") as JSON with the .glb inside
// as base64, the same file the arena artifact serves (artifact pages do not serve .glb).
// Sounds and music come from packages/audio and need nothing here.
//
//   npm run assets -w @crowned-dogs/web                      downloads the model
//   npm run assets -w @crowned-dogs/web -- --glb model.glb   or wraps a .glb you have

import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { parseArgs } from "node:util";

const MODEL_URL = "https://raw.githubusercontent.com/mrdoob/three.js/r160/examples/models/gltf/Xbot.glb";
const out = join(dirname(fileURLToPath(import.meta.url)), "..", "public", "models", "mannequin.json");
const { values } = parseArgs({ options: { glb: { type: "string" } } });

if (existsSync(out) && !values.glb) {
  console.log("модель уже есть:", out);
} else {
  let bytes: Buffer;
  if (values.glb) {
    bytes = readFileSync(values.glb);
  } else {
    const res = await fetch(MODEL_URL);
    if (!res.ok) throw new Error(`не скачалась модель: ${res.status} ${MODEL_URL}`);
    bytes = Buffer.from(await res.arrayBuffer());
  }
  mkdirSync(dirname(out), { recursive: true });
  writeFileSync(out, JSON.stringify({ data: bytes.toString("base64") }));
  console.log("модель готова:", out);
}
