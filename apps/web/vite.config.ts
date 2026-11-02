import { createHash } from "node:crypto";
import { cpSync, createReadStream, existsSync, mkdirSync, readdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { join, normalize, sep } from "node:path";
import { fileURLToPath } from "node:url";
import { defineConfig, type Plugin } from "vite";
import { ktx2Copy } from "./scripts/ktx2-cache.ts";

// The sound package keeps its files in packages/audio/assets. The game serves them
// next to the page (sfx/, amb/, music/, voice/), in dev and in the build alike, so the built
// folder is laid out like the arena artifact.
// The effects and ambience are cut from the Sonniss bundle, whose licence keeps the files
// out of a public repository: git ignores sfx/ and amb/. They live on Art's PC in
// private-audio/ next to the clone (or wherever CD_PRIVATE_AUDIO points); without them the
// game builds and plays silent.
const AUDIO = fileURLToPath(new URL("../../packages/audio/assets", import.meta.url));
const PRIVATE_AUDIO = process.env.CD_PRIVATE_AUDIO ?? fileURLToPath(new URL("../../../private-audio", import.meta.url));
const AUDIO_DIRS = ["sfx", "amb", "music", "voice"];
const audioRoot = (dir: string) => [AUDIO, PRIVATE_AUDIO].find((root) => existsSync(join(root, dir)));

function audioAssets(): Plugin {
  return {
    name: "crowned-dogs-audio",
    configureServer(server) {
      server.middlewares.use((req, res, next) => {
        const path = decodeURIComponent((req.url ?? "").split("?")[0]!).replace(/^\/+/, "");
        const root = AUDIO_DIRS.includes(path.split("/")[0]!) ? audioRoot(path.split("/")[0]!) : undefined;
        const file = root && normalize(join(root, path));
        if (!root || !file || !file.startsWith(normalize(root) + sep) || !existsSync(file) || !statSync(file).isFile()) return next();
        res.setHeader("Content-Type", "audio/mpeg");
        res.setHeader("Content-Length", statSync(file).size);
        createReadStream(file).pipe(res);
      });
    },
    writeBundle(opts) {
      for (const d of AUDIO_DIRS) {
        const root = audioRoot(d);
        if (root) cpSync(join(root, d), join(opts.dir!, d), { recursive: true });
        else console.warn(`crowned-dogs-audio: no ${d}/ in packages/audio/assets or ${PRIVATE_AUDIO}; the build has no ${d} sounds`);
      }
    },
  };
}

// Figure models live in models/ as .glb. Artifact pages do not serve .glb, so the game
// loads each one as models/<name>.json with the .glb inside as base64, in dev and in
// the build alike. A model whose GPU-compressed copy is made (npm run models:ktx2) also
// ships as models/<name>.k.json, for the phone qualities; a stale copy is left out.
const MODELS = fileURLToPath(new URL("./models", import.meta.url));
const wrap = (file: string) => JSON.stringify({ data: readFileSync(file).toString("base64") });
const wrapModel = (glb: string) => wrap(join(MODELS, glb));
const glbs = () => readdirSync(MODELS).filter((f) => f.endsWith(".glb"));
const compressed = (glb: string): string | null => {
  const copy = ktx2Copy(join(MODELS, glb));
  return existsSync(copy) ? copy : null;
};
// KTX2Loader's transcoder, which three.js ships but does not bundle.
const BASIS = fileURLToPath(new URL("../../node_modules/three/examples/jsm/libs/basis", import.meta.url));
const BASIS_FILES = ["basis_transcoder.js", "basis_transcoder.wasm"];

function figureModels(): Plugin {
  return {
    name: "crowned-dogs-models",
    configureServer(server) {
      server.middlewares.use((req, res, next) => {
        const path = (req.url ?? "").split("?")[0]!;
        const basis = /^\/basis\/([\w.]+)$/.exec(path);
        if (basis && BASIS_FILES.includes(basis[1]!)) {
          res.setHeader("Content-Type", basis[1]!.endsWith(".wasm") ? "application/wasm" : "text/javascript");
          res.end(readFileSync(join(BASIS, basis[1]!)));
          return;
        }
        const m = /^\/models\/([\w-]+)(\.k)?\.json$/.exec(path);
        if (!m || !glbs().includes(m[1] + ".glb")) return next();
        const copy = m[2] ? compressed(m[1] + ".glb") : null;
        if (m[2] && !copy) return next();
        res.setHeader("Content-Type", "application/json");
        res.end(copy ? wrap(copy) : wrapModel(m[1] + ".glb"));
      });
    },
    writeBundle(opts) {
      mkdirSync(join(opts.dir!, "models"), { recursive: true });
      for (const f of glbs()) {
        writeFileSync(join(opts.dir!, "models", f.replace(/\.glb$/, ".json")), wrapModel(f));
        const copy = compressed(f);
        if (copy) writeFileSync(join(opts.dir!, "models", f.replace(/\.glb$/, ".k.json")), wrap(copy));
      }
      mkdirSync(join(opts.dir!, "basis"), { recursive: true });
      for (const f of BASIS_FILES) cpSync(join(BASIS, f), join(opts.dir!, "basis", f));
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
        for (const d of AUDIO_DIRS) {
          const root = audioRoot(d);
          if (root) for (const f of filesUnder(root, d)) versions[f] = hash(join(root, f));
        }
        for (const f of glbs()) {
          versions[`models/${f.replace(/\.glb$/, ".json")}`] = hash(join(MODELS, f));
          const copy = compressed(f);
          if (copy) versions[`models/${f.replace(/\.glb$/, ".k.json")}`] = hash(copy);
        }
        for (const f of BASIS_FILES) versions[`basis/${f}`] = hash(join(BASIS, f));
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
