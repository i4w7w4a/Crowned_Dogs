import "./asset-cache.ts";
import init, { initSync, dispatch, graphics_texture, engine_version, audio_samples, motion_compute } from "../wasm/game_wasm.js";
export { RustBattle, RustArena, RustLens, RustTextureTiers, BrowserScroll, BrowserWeapon, RustItemViewer, RustPersonaViewer, RustMawLoader } from "../wasm/game_wasm.js";
export { BrowserBase, mount_sound_settings, show_sound_settings, equipment_icon, equipment_item_icon, fitting_room_icon, navigation_back_arrow } from "../wasm/game_wasm.js";
export { BrowserGameAudio, BrowserArenaAudio } from "../wasm/game_wasm.js";
export { RustApplication, mount_application, unmount_application, activate_after_ready, run_battle_loading, RustFightLog, liquid_glass, RustRadio } from "../wasm/game_wasm.js";
export { RustGrip, RustFeet, rig_set_world_quat, rig_turn, rig_reach, rig_gait_of, RustMotion, RustPose, RustCollapse, RustViewmodel, rig_find_bones, rig_axes_pitch, rig_axes_roll, rig_names, RustArmsOnly, RustHoldOut, RustFlash, RustDust } from "../wasm/game_wasm.js";
export { sceneLoadModel, sceneLoadGuns, sceneGripOf, RustSceneHand, sceneDress, sceneTune, sceneInstallRelief, sceneReliefHook, sceneSetReliefPreview, sceneTurntableMaterial, sceneTuneFittingMaterial, sceneSilhouetteDeck, sceneApplyLook, sceneMakeFittingStage, sceneMakeSling, sceneMakeCarry, scenePrepareVaryag, sceneFitVaryag, RustDepthBackdrop } from "../wasm/game_wasm.js";
export { RustDuelService, RustOperatorRegistry, bind_server_bot, run_server_main } from "../wasm/game_wasm.js";
export { RustMusicDirector, RustVariantPicker, RustShuffle, audio_pick_other, audio_jitter } from "../wasm/game_wasm.js";
export { RustLocalDuel, store_get, store_set, load_local_loadout, save_local_loadout, load_local_look, save_local_look, guess_quality, start_quality, keep_quality } from "../wasm/game_wasm.js";
export { sceneAuthoredWeaponFit } from "../wasm/game_wasm.js";
export { sceneSetBeardFirstPerson } from "../wasm/game_wasm.js";
export { RustProjectedStage, RustHomeStage, RustRegionMap } from "../wasm/game_wasm.js";

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
