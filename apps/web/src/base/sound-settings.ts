/** Dependencies for the Rust settings panel. */
import { mount_sound_settings, show_sound_settings } from "../../../../packages/runtime/src/index.ts";
import { gameAudio } from "../audio/arena-audio.ts";
import { liquidGlass } from "../hud/liquid.ts";
import { QUALITY, QUALITY_IDS, keepQuality } from "../arena/quality.ts";
import { store } from "../store.ts";
export function mountSoundSettings(): void { mount_sound_settings({ gameAudio, liquidGlass, QUALITY, QUALITY_IDS, keepQuality, store }); }
export function showSoundSettings(on: boolean): void { show_sound_settings(on); }
