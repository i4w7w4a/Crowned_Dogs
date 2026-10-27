import { defineConfig } from "vite";

// Relative paths, so the built game works from any folder or static host.
export default defineConfig({
  base: "./",
  build: { target: "es2022", chunkSizeWarningLimit: 900 },
});
