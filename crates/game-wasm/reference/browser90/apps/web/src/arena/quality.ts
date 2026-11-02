// Graphics settings: four levels from a weak phone in Telegram to a strong PC.
// Every level draws the same picture, the higher ones just draw it cleaner and add
// light: the haze, the light shafts from the roof, dust in the air.

import { callRust } from "../../../../packages/runtime/src/index.ts";
import { store } from "../store.ts";

export type QualityId = "low" | "medium" | "high" | "ultra";

export interface Quality {
  id: QualityId;
  name: string;
  /** Device pixels per CSS pixel, at most. */
  maxDpr: number;
  /** At most this many pixels in the drawing buffer, so big screens stay smooth. */
  maxPixels: number;
  /** Supersampling: the frame is drawn this many times larger each way and scaled down. */
  ssaa: number;
  /** Multisampling of the figures' edges. */
  msaa: number;
  /** Shadow map size, 0 = only the soft spot under the feet. */
  shadow: number;
  /** Shadow softness in texels, and how many looks into the map make it smooth. */
  shadowRadius: number;
  shadowSamples: number;
  /** Steps of the lens smear at the edges. */
  taps: number;
  /** The lens reads a blurred copy of the frame for its smear and glow (needs mipmaps of the frame). */
  mips: boolean;
  /** Light shafts from the roof openings and windows of the painting: steps, 0 = none, and their buffer size. */
  rays: number;
  raysScale: number;
  /** Haze in the air, lit by the opening, with the figures' shadows in it: march steps, 0 = none. */
  haze: number;
  hazeScale: number;
  /** Reads of the painting for the blur behind the enemy (1 = one read of a blurred copy). */
  dofTaps: number;
  /** Dust specks floating in the light. */
  dust: number;
  /** Texture size of the figures. */
  textures: 512 | 1024 | 2048 | 4096;
  anisotropy: number;
  /** The figures reflect the hall (light and metal look real). */
  env: boolean;
  /** The enemy's reflection on the wet floor. */
  mirror: boolean;
  /** The double-size painting, drawn with a sharp filter. */
  sharpBackdrop: boolean;
}

export const QUALITY = callRust<{ QUALITY: Record<QualityId, Quality> }>("client.data", []).QUALITY;
export const QUALITY_IDS = Object.keys(QUALITY) as QualityId[];
const KEY = "cd-quality";

/** Only platform facts cross this boundary; selection policy is Rust. */
function deviceFacts(gl: WebGL2RenderingContext | null) {
  if (!gl) return { webgl: false };
  const info = gl.getExtension("WEBGL_debug_renderer_info");
  const nav = navigator as Navigator & { deviceMemory?: number };
  return {
    webgl: true,
    gpu: String(info ? gl.getParameter(info.UNMASKED_RENDERER_WEBGL) : gl.getParameter(gl.RENDERER)),
    memory: nav.deviceMemory ?? 4, cores: nav.hardwareConcurrency ?? 4,
    phone: /android|iphone|ipad|ipod|mobile/i.test(navigator.userAgent) || (window.matchMedia?.("(pointer: coarse)").matches ?? false),
  };
}
export function guessQuality(gl: WebGL2RenderingContext | null): QualityId {
  return callRust("quality.guess", [deviceFacts(gl)]);
}
export function startQuality(gl: WebGL2RenderingContext | null): { id: QualityId; picked: boolean } {
  const url = new URLSearchParams(location.search).get("q");
  return callRust("quality.start", [url, store.get(KEY), deviceFacts(gl)]);
}
export function keepQuality(id: QualityId): void { store.set(KEY, id); }
