// Fail the production build when a model lost its GPU-compressed variant.
// A missing optional HTTP download must not silently turn into a full-size model for every player.
import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const args = process.argv.slice(2);
const supplied = args[0] === "--dist" ? args[1] : undefined;
if ((args.length > 0 && args[0] !== "--dist") || (args[0] === "--dist" && args.length !== 2)) {
  console.error("Usage: node scripts/verify-production-assets.ts [--dist <directory>]");
  process.exit(2);
}
const dist = resolve(supplied ?? fileURLToPath(new URL("../dist", import.meta.url)));
const models = join(dist, "models");
const problems: string[] = [];
const sources = existsSync(models)
  ? readdirSync(models).filter((file) => file.endsWith(".glb") && !file.endsWith(".k.glb"))
  : [];

if (sources.length === 0) problems.push("no source .glb models found");

for (const file of sources) {
  const compressed = join(models, file.replace(/\.glb$/, ".k.glb"));
  const name = file.replace(/\.glb$/, ".k.glb");
  if (!existsSync(compressed)) {
    problems.push(`${name} is missing`);
    continue;
  }
  if (statSync(compressed).size < 12 || readFileSync(compressed).subarray(0, 4).toString() !== "glTF") {
    problems.push(`${name} is not a valid GLB`);
  }
}

if (problems.length > 0) {
  console.error(`KTX2 production verification failed:\n- ${problems.join("\n- ")}`);
  console.error("Run npm run models:ktx2 -w @crowned-dogs/web and rebuild.");
  process.exit(1);
}

console.log(`KTX2 production verification: ${sources.length} compressed model variants present`);
