import { cpSync, createReadStream, existsSync, statSync } from "node:fs";
import { join, normalize, sep } from "node:path";
import { fileURLToPath } from "node:url";
import { defineConfig, type Plugin } from "vite";

// The sound package keeps its files in packages/audio/assets. The game serves them
// next to the page (sfx/, amb/, music/), in dev and in the build alike, so the built
// folder is laid out like the arena artifact.
const AUDIO = fileURLToPath(new URL("../../packages/audio/assets", import.meta.url));
const AUDIO_DIRS = ["sfx", "amb", "music"];

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

// Relative paths, so the built game works from any folder or static host. Fixed file
// names, so publishing a new build replaces the old files instead of piling up.
export default defineConfig({
  base: "./",
  plugins: [audioAssets()],
  build: {
    target: "es2022",
    chunkSizeWarningLimit: 900,
    rolldownOptions: { output: { entryFileNames: "assets/arena.js", chunkFileNames: "assets/[name].js", assetFileNames: "assets/[name][extname]" } },
  },
});
