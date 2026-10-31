import { createHash } from "node:crypto";
import { cpSync, createReadStream, existsSync, mkdirSync, readdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { join, normalize, sep } from "node:path";
import { fileURLToPath } from "node:url";
import { defineConfig, type Plugin } from "vite";

// The sound package keeps its files in packages/audio/assets. The game serves them
// next to the page (sfx/, amb/, music/, voice/), in dev and in the build alike, so the built
// folder is laid out like the arena artifact.
const AUDIO = fileURLToPath(new URL("../../packages/audio/assets", import.meta.url));
const AUDIO_DIRS = ["sfx", "amb", "music", "voice"];

function audioAssets(): Plugin {
  return {
    name: "crowned-dogs-audio",
    configureServer(server) {
      server.middlewares.use((req, res, next) => {
        const path = decodeURIComponent((req.url ?? "").split("?")[0]!).replace(/^\/+/, "");
        const file = normalize(join(AUDIO, path));
        if (!AUDIO_DIRS.includes(path.split("/")[0]!) || !file.startsWith(AUDIO + sep) || !existsSync(file) || !statSync(file).isFile()) return next();
        res.setHeader("Content-Type", "audio/mpeg");
        res.setHeader("Content-Length", statSync(file).size);
        createReadStream(file).pipe(res);
      });
    },
    writeBundle(opts) {
      for (const d of AUDIO_DIRS) cpSync(join(AUDIO, d), join(opts.dir!, d), { recursive: true });
    },
  };
}

// Figure models live in models/ as .glb. Artifact pages do not serve .glb, so the game
// loads each one as models/<name>.json with the .glb inside as base64, in dev and in
// the build alike.
const MODELS = fileURLToPath(new URL("./models", import.meta.url));
const wrapModel = (glb: string) => JSON.stringify({ data: readFileSync(join(MODELS, glb)).toString("base64") });
const glbs = () => readdirSync(MODELS).filter((f) => f.endsWith(".glb"));

function figureModels(): Plugin {
  return {
    name: "crowned-dogs-models",
    configureServer(server) {
      server.middlewares.use((req, res, next) => {
        const m = /^\/models\/([\w-]+)\.json$/.exec((req.url ?? "").split("?")[0]!);
        if (!m || !glbs().includes(m[1] + ".glb")) return next();
        res.setHeader("Content-Type", "application/json");
        res.end(wrapModel(m[1] + ".glb"));
      });
    },
    writeBundle(opts) {
      mkdirSync(join(opts.dir!, "models"), { recursive: true });
      for (const f of glbs()) writeFileSync(join(opts.dir!, "models", f.replace(/\.glb$/, ".json")), wrapModel(f));
    },
  };
}

// The build tells packages/runtime/src/asset-cache.ts a content hash for every big file it serves, so the
// browser keeps each file until that file changes. The dev server gets none and caches nothing.
const PUBLIC = fileURLToPath(new URL("./public", import.meta.url));
const WASM = fileURLToPath(new URL("../../packages/runtime/wasm/game_wasm_bg.wasm", import.meta.url));
const hash = (file: string) => createHash("sha1").update(readFileSync(file)).digest("hex").slice(0, 12);
function filesUnder(root: string, dir = ""): string[] {
  return readdirSync(join(root, dir), { withFileTypes: true }).flatMap((e) =>
    e.isDirectory() ? filesUnder(root, join(dir, e.name)) : [join(dir, e.name).split(sep).join("/")]);
}
function assetVersions(): Plugin {
  return {
    name: "crowned-dogs-asset-versions",
    config(_, { command }) {
      const versions: Record<string, string> = {};
      if (command === "build") {
        for (const f of filesUnder(PUBLIC)) versions[f] = hash(join(PUBLIC, f));
        for (const d of AUDIO_DIRS) for (const f of filesUnder(AUDIO, d)) versions[f] = hash(join(AUDIO, f));
        for (const f of glbs()) versions[`models/${f.replace(/\.glb$/, ".json")}`] = hash(join(MODELS, f));
        versions["assets/game_wasm_bg.wasm"] = hash(WASM);
      }
      return { define: { __CD_ASSET_VERSIONS__: JSON.stringify(versions) } };
    },
  };
}

// Relative paths, so the built game works from any folder or static host. Fixed file
// names, so publishing a new build replaces the old files instead of piling up.
export default defineConfig({
  base: "./",
  plugins: [audioAssets(), figureModels(), assetVersions()],
  build: {
    target: "es2022",
    chunkSizeWarningLimit: 900,
    rolldownOptions: { output: { entryFileNames: "assets/arena.js", chunkFileNames: "assets/[name].js", assetFileNames: "assets/[name][extname]" } },
  },
});
