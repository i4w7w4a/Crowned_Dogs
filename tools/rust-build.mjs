import { spawnSync } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, writeFileSync, cpSync, readdirSync, statSync, rmSync, realpathSync } from "node:fs";
import { join, resolve, sep } from "node:path";
import { homedir } from "node:os";
import { createHash } from "node:crypto";

const root = resolve(import.meta.dirname, "..");
const bin = join(homedir(), ".cargo", "bin");
const env = { ...process.env, PATH: bin + (process.platform === "win32" ? ";" : ":") + process.env.PATH };
function run(command, args) {
  const result = spawnSync(command, args, { cwd: root, env, stdio: "inherit", shell: false });
  if (result.error) throw new Error(`${command}: ${result.error.message}. Install Rust and wasm-bindgen-cli 0.2.129 first.`);
  if (result.status !== 0) process.exit(result.status ?? 1);
}
const exe = name => join(bin, name + (process.platform === "win32" ? ".exe" : ""));
const mode = process.argv[2] ?? "wasm";
const hash = path => createHash("sha256").update(readFileSync(path)).digest("hex");

if (mode === "wasm" || mode === "local") {
  run(exe("cargo"), ["build", "--locked", "-p", "game-wasm", "--target-dir", "target/wasm-build", "--target", "wasm32-unknown-unknown", "--release"]);
  const wasm = join(root, "target/wasm-build/wasm32-unknown-unknown/release/game_wasm.wasm");
  const pkg = join(root, "packages/runtime/wasm");
  const signature = join(pkg, "source.sha256");
  if (!existsSync(signature) || readFileSync(signature, "utf8") !== hash(wasm)) {
    run(exe("wasm-bindgen"), [wasm, "--target", "web", "--out-dir", pkg, "--out-name", "game_wasm"]);
    writeFileSync(signature, hash(wasm));
  }
}
if (mode === "check") {
  run(exe("cargo"), ["test", "--locked", "--workspace", "--release"]);
}
if (mode === "local") {
  run(process.execPath, ["node_modules/vite/bin/vite.js", "build", "apps/web", "--config", "apps/web/vite.config.ts"]);
  run(exe("cargo"), ["build", "--locked", "-p", "crowned-dogs-desktop", "--release"]);
  const output = join(root, "release-local/Crowned-Dogs");
  mkdirSync(output, { recursive: true });
  // This is generated output only. Resolve the target before removing it so a
  // junction cannot turn a packaging refresh into deletion outside the release.
  const gameOutput = join(output, "game");
  if (existsSync(gameOutput)) {
    const actual = realpathSync(gameOutput);
    const allowed = realpathSync(output) + sep;
    if (!actual.startsWith(allowed)) throw new Error(`Unsafe generated output path: ${actual}`);
    rmSync(gameOutput, { recursive: true });
  }
  for (const file of ["CrownedDogs.exe", "CrownedDogs-Web.exe"]) cpSync(join(root, "target/release", file), join(output, file));
  cpSync(join(root, "apps/web/dist"), gameOutput, { recursive: true });
  writeFileSync(join(output, "ПРОЧТИ.txt"), `CROWNED DOGS — локальная сборка Rust\n\nCrownedDogs.exe — отдельная Windows-программа.\nCrownedDogs-Web.exe — та же игра в обычном браузере.\nЗакройте первый вариант перед запуском второго: оба используют локальный порт 5197.\n\nПапка game должна оставаться рядом с программами. Интернет для игры не нужен.\nВ браузерном варианте закройте окно сервера, чтобы остановить игру.\nКомплект и настройки сохраняются отдельно в каждом клиенте.\n\nИгровое ядро, AI, комплекты, звук и расчёты движения — Rust/WASM.\nРендер и связь с браузером — Three.js/WebGL; шейдеры — GLSL.\nSilhouette POM работает на подиуме при средней графике и выше.\n\nСборка личная: звуковые файлы не распространять отдельно от игры.\n`);
  const files = [];
  function visit(path) { for (const entry of readdirSync(path, { withFileTypes: true })) {
    const full = join(path, entry.name); if (entry.isDirectory()) visit(full);
    else if (entry.name !== "MANIFEST.json") files.push({ path: full.slice(output.length + 1).replaceAll("\\", "/"), size: statSync(full).size, sha256: hash(full) });
  }}
  visit(output);
  writeFileSync(join(output, "MANIFEST.json"), JSON.stringify({ version: "0.2.0", builtAt: new Date().toISOString(), files }, null, 2));
  console.log(`\nLocal game: ${output}`);
}
