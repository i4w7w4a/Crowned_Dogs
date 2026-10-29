import init, { initSync, dispatch, graphics_texture, engine_version, audio_samples, motion_compute } from "../wasm/game_wasm.js";
export { RustBattle, RustArena, RustLens, RustTextureTiers, BrowserScroll, BrowserWeapon, RustItemViewer, RustPersonaViewer, RustMawLoader } from "../wasm/game_wasm.js";
export { BrowserBase, mount_sound_settings, show_sound_settings, equipment_icon, equipment_item_icon, fitting_room_icon, navigation_back_arrow } from "../wasm/game_wasm.js";
export { BrowserGameAudio, BrowserArenaAudio } from "../wasm/game_wasm.js";

const wasmUrl = new URL("../wasm/game_wasm_bg.wasm", import.meta.url);
const node = (globalThis as { process?: { versions?: { node?: string } } }).process?.versions?.node;

/** Initialization is a module dependency, so no rule or screen can accidentally run before Rust. */
export const engineReady = (async () => {
  if (node) {
    const fs = await import(/* @vite-ignore */ ["node", "fs/promises"].join(":"));
    initSync({ module: await fs.readFile(wasmUrl) });
  } else {
    await init({ module_or_path: wasmUrl });
    document.documentElement.dataset.engine = engine_version();
  }
})();
await engineReady;

export function callRust<T>(method: string, args: unknown): T {
  try { return JSON.parse(dispatch(method, JSON.stringify(args))) as T; }
  catch (error) { throw error instanceof Error ? error : new Error(String(error)); }
}

export function graphicsTexture(kind: string, size: number): Uint8Array {
  return graphics_texture(kind, size);
}

export function audioSamples(kind: string, rate: number, seconds: number, seed: number): Float32Array<ArrayBuffer> {
  return audio_samples(kind, rate, seconds, seed) as Float32Array<ArrayBuffer>;
}

export const engineVersion = engine_version();

export function motionCompute(kind: string, input: Float64Array): Float64Array<ArrayBuffer> {
  return motion_compute(kind, input) as Float64Array<ArrayBuffer>;
}
