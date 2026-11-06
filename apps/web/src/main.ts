/** Browser dependencies for the Rust battle controller. */
import { RADIO_PLAYLIST, TIMING, TRACKS } from "@crowned-dogs/audio";
import { VETERAN_KITS, WEAPONS, WEAPON_IDS, type Loadout } from "@crowned-dogs/core";
import { RustBattle } from "../../../packages/runtime/src/index.ts";
import { Arena } from "./arena/arena.ts";
import { CALIBRATION_FRAME, FRAMES } from "./arena/frames.ts";
import { startQuality } from "./arena/quality.ts";
import { gameAudio } from "./audio/arena-audio.ts";
import { online } from "./online/session.ts";
import { FightLog, icon } from "./hud/log.ts";
import { liquidGlass } from "./hud/liquid.ts";
import { store } from "./store.ts";
import "./style.css";

const bindings = {
  Arena, FightLog, gameAudio, liquidGlass, icon, store, startQuality, remote: null as any,
  frames: /[?&]cal\b/.test(location.search) ? [CALIBRATION_FRAME, ...FRAMES] : FRAMES,
  weaponIds: WEAPON_IDS, weapons: WEAPONS, veteranKits: VETERAN_KITS,
  playlist: RADIO_PLAYLIST, tracks: TRACKS, timing: TIMING,
  myModel: import.meta.env.VITE_MODEL ?? "models/persona-male.json",
  foeModel: import.meta.env.VITE_MODEL ?? "models/persona-female.json",
};
let battle: RustBattle | undefined;

export async function startBattle(
  kit: Loadout, onExit: () => void, beforeEnter: () => Promise<void> = async () => {}, signal?: AbortSignal,
): Promise<void> {
  bindings.remote = await online.openBattle(signal);
  battle ??= new RustBattle(bindings);
  return battle.start(kit, () => { online.leaveBattle(bindings.remote); onExit(); }, beforeEnter, signal) as Promise<void>;
}
