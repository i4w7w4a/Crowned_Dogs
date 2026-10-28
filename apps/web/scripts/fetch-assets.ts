// Puts the files the game needs but git does not keep into public/media/:
//   models/mannequin.glb  the placeholder figure (three.js example "X Bot"), downloaded
//   music/*.mp3           Art's tracks, copied from --music <folder>
//   sfx/*.mp3             sound effects, copied from --sfx <folder>
// Music and effects stay out of git: the effects' licence forbids sharing them on their own.
//
//   npm run assets -w @crowned-dogs/web -- --music "C:\...\music\web-128k" --sfx "C:\...\sfx\arena"

import { copyFileSync, existsSync, mkdirSync, readdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { parseArgs } from "node:util";

const MODEL_URL = "https://raw.githubusercontent.com/mrdoob/three.js/r160/examples/models/gltf/Xbot.glb";
const out = join(dirname(fileURLToPath(import.meta.url)), "..", "public", "media");
const { values } = parseArgs({ options: { music: { type: "string" }, sfx: { type: "string" } } });

const model = join(out, "models", "mannequin.glb");
if (existsSync(model)) {
  console.log("модель уже есть:", model);
} else {
  const res = await fetch(MODEL_URL);
  if (!res.ok) throw new Error(`не скачалась модель: ${res.status} ${MODEL_URL}`);
  mkdirSync(dirname(model), { recursive: true });
  writeFileSync(model, Buffer.from(await res.arrayBuffer()));
  console.log("скачал модель:", model);
}

for (const kind of ["music", "sfx"] as const) {
  const from = values[kind];
  if (!from) { console.log(`--${kind} не указан: ${kind} пропускаю, игра будет без него`); continue; }
  const files = readdirSync(from).filter((f) => f.toLowerCase().endsWith(".mp3"));
  mkdirSync(join(out, kind), { recursive: true });
  for (const f of files) copyFileSync(join(from, f), join(out, kind, f));
  console.log(`${kind}: скопировал ${files.length} файлов из ${from}`);
}
