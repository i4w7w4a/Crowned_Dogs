// The game's own figure, the male Persona, is in models/persona-male.glb and needs
// nothing from here. This puts the old placeholder mannequin (three.js example
// "X Bot") into public/models/ as JSON with the .glb inside as base64, for side-by-side
// checks: run the game with VITE_MODEL=models/mannequin.json to see it.
//
//   npm run assets -w @crowned-dogs/web                      downloads the X Bot
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
