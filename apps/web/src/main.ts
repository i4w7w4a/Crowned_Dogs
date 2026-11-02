/** Browser dependencies for the Rust battle controller. */
import { RADIO_PLAYLIST, TIMING, TRACKS } from "@crowned-dogs/audio";
import { VETERAN_KITS, WEAPONS, WEAPON_IDS, type Loadout } from "@crowned-dogs/core";
import { RustBattle } from "../../../packages/runtime/src/index.ts";
import { Arena } from "./arena/arena.ts";
import { CALIBRATION_FRAME, FRAMES } from "./arena/frames.ts";
import { startQuality } from "./arena/quality.ts";
import { gameAudio } from "./audio/arena-audio.ts";
import { LocalDuel } from "./game/local-duel.ts";
import { FightLog, icon } from "./hud/log.ts";
import { liquidGlass } from "./hud/liquid.ts";
import { store } from "./store.ts";
import "./style.css";

const battle = new RustBattle({
  Arena, LocalDuel, FightLog, gameAudio, liquidGlass, icon, store, startQuality,
  frames: /[?&]cal\b/.test(location.search) ? [CALIBRATION_FRAME, ...FRAMES] : FRAMES,
  weaponIds: WEAPON_IDS, weapons: WEAPONS, veteranKits: VETERAN_KITS,
  playlist: RADIO_PLAYLIST, tracks: TRACKS, timing: TIMING,
  myModel: import.meta.env.VITE_MODEL ?? "models/persona-male.json",
  foeModel: import.meta.env.VITE_MODEL ?? "models/persona-female.json",
});

export function startBattle(
  kit: Loadout, onExit: () => void, beforeEnter: () => Promise<void> = async () => {}, signal?: AbortSignal,
): Promise<void> {
  return battle.start(kit, onExit, beforeEnter, signal) as Promise<void>;
}
