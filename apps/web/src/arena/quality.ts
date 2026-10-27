// Graphics settings: four levels from a weak phone in Telegram to a strong PC.
// Every level draws the same picture, the higher ones just draw it cleaner and add
// light: the haze, the light shafts from the roof, dust in the air.

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

export const QUALITY: Record<QualityId, Quality> = {
  low: {
    id: "low", name: "Низкая", maxDpr: 1, maxPixels: 0.5e6, ssaa: 1, msaa: 4,
    shadow: 0, shadowRadius: 0, shadowSamples: 0, taps: 5, mips: false,
    rays: 0, raysScale: 0.5, haze: 0, hazeScale: 0.5, dust: 0,
    textures: 512, anisotropy: 1, env: false, mirror: false, sharpBackdrop: false,
  },
  medium: {
    id: "medium", name: "Средняя", maxDpr: 2, maxPixels: 1.7e6, ssaa: 1, msaa: 4,
    shadow: 1024, shadowRadius: 3, shadowSamples: 8, taps: 8, mips: true,
    rays: 28, raysScale: 0.5, haze: 0, hazeScale: 0.5, dust: 220,
    textures: 1024, anisotropy: 4, env: true, mirror: true, sharpBackdrop: false,
  },
  high: {
    id: "high", name: "Высокая", maxDpr: 3, maxPixels: 3.8e6, ssaa: 1, msaa: 8,
    shadow: 2048, shadowRadius: 5, shadowSamples: 16, taps: 12, mips: true,
    rays: 56, raysScale: 0.5, haze: 24, hazeScale: 0.5, dust: 600,
    textures: 2048, anisotropy: 8, env: true, mirror: true, sharpBackdrop: true,
  },
  ultra: {
    id: "ultra", name: "Ультра", maxDpr: 3, maxPixels: 8e6, ssaa: 1.5, msaa: 8,
    shadow: 4096, shadowRadius: 9, shadowSamples: 32, taps: 18, mips: true,
    rays: 96, raysScale: 0.75, haze: 64, hazeScale: 0.5, dust: 1500,
    textures: 4096, anisotropy: 16, env: true, mirror: true, sharpBackdrop: true,
  },
};

export const QUALITY_IDS = Object.keys(QUALITY) as QualityId[];
const KEY = "cd-quality";

/** What this device can carry: a phone gets low or medium, a computer high. Ultra is only ever picked by hand. */
export function guessQuality(gl: WebGL2RenderingContext | null): QualityId {
  if (!gl) return "low";
  const info = gl.getExtension("WEBGL_debug_renderer_info");
  const gpu = String(info ? gl.getParameter(info.UNMASKED_RENDERER_WEBGL) : gl.getParameter(gl.RENDERER)).toLowerCase();
  if (/swiftshader|llvmpipe|software|basic render/.test(gpu)) return "low";
  const nav = navigator as Navigator & { deviceMemory?: number };
  const memory = nav.deviceMemory ?? 4, cores = nav.hardwareConcurrency ?? 4;
  const phone = /android|iphone|ipad|ipod|mobile/i.test(navigator.userAgent) || (window.matchMedia?.("(pointer: coarse)").matches ?? false);
  if (phone) {
    // old Mali and Adreno 3xx–5xx, little memory or few cores: the light version
    if (memory <= 3 || cores <= 4 || /mali-[t4]|mali-g(5[0-9]|7[0-2])\b|adreno \(tm\) [345]\d\d/.test(gpu)) return "low";
    return "medium";
  }
  return /intel|uhd|hd graphics|iris/.test(gpu) && memory <= 8 ? "medium" : "high";
}

/** The level from the address (?q=ultra, for checks), the one picked in the menu, or the device's own. */
export function startQuality(gl: WebGL2RenderingContext | null): { id: QualityId; picked: boolean } {
  const url = /[?&]q=(\w+)/.exec(location.search)?.[1] as QualityId | undefined;
  if (url && url in QUALITY) return { id: url, picked: true };
  const kept = store.get(KEY) as QualityId | null;
  if (kept && kept in QUALITY) return { id: kept, picked: true };
  return { id: guessQuality(gl), picked: false };
}

export function keepQuality(id: QualityId): void {
  store.set(KEY, id);
}
