// Painted backdrops the 3D figures stand in. Each frame is a pre-rendered picture
// plus how its painted camera sees the floor, so live figures line up with it.

import type { TimeOfDay, Weather } from "@crowned-dogs/core";
import type { LensLook } from "./lens.ts";

export interface Calibration {
  /** Vertical lens angle, degrees. */
  fov: number;
  /** Camera height, metres. */
  camH: number;
  /** Where the feet of a 1.8 m figure 7 steps away are, as fractions of the picture. */
  feet: [number, number];
  /** How tall that figure looks, as a fraction of the picture height. */
  height: number;
  /** Where our own Persona stands in front of the camera, metres (x right, z forward is negative). */
  me: { x: number; y: number; z: number };
  /** How far the live camera has stepped back from the painted one, metres: both figures stand that much
   *  further away (ours shrinks the most, the enemy barely), the hall itself is far enough not to change. */
  back: number;
  /** First person (bodycam): the camera's height, our eyes, and how much closer than the painted 7 steps
   *  the enemy stands at 7 steps. The painted horizon stays where it is. */
  fp: { camH: number; near: number };
}

export interface Look {
  exposure: number;
  /** Dry surfaces have no planar fighter reflection, regardless of quality. */
  reflection?: boolean;
  /** The low light from behind the enemy, the narrow spot on the enemy, the haze colour. */
  rim: number;
  fill: number;
  fog: number;
  /** Soft light from above and from the floor. */
  sky: number;
  ground: number;
  /** Teal glow from a puddle at the enemy's feet, 0 for none. */
  puddle: number;
  /** The bodycam lens over the whole frame, and how the live figures are graded to sit in the painting. */
  lens: LensLook;
  /** Intensities belong to the physical lights, independently of their colours. */
  lighting?: { sky?: number; rim?: number; fill?: number; key?: number };
  /** Direction towards the distant source in the shared battle coordinates. */
  sun?: [number, number, number];
}

export interface StageProjection {
  manifest: string;
  /** Optional plate overrides are page-relative; geometry stays in the shared manifest. */
  textures?: Partial<Record<"clean" | "equipment" | "floor" | "wall" | "roof", string>>;
  textures2x?: Partial<Record<"clean" | "equipment" | "floor" | "wall" | "roof", string>>;
  /** Only the real surfaces outside the photographed frustum use these. */
  extensionTint?: number;
  extensionExposure?: number;
  /** Hidden geometry has independent lighting; legacy scenes retain their authored colours. */
  sideTint?: number;
  sideExposure?: number;
}

/** The bodycam of Art's style: sharp middle, rainbow smear and a fisheye bend towards the edges, dark corners. */
const BODYCAM = { smear: 0.045, soft: 0.0025, barrel: 0, vignette: 0.97, glow: 0.45, contrast: 0.3, near: 2.5 };

export interface Frame {
  id: string;
  name: string;
  /** Picture, relative to the page, and its best available copy for high graphics. */
  src: string;
  src2x?: string;
  /** Its depth map (1/z, made with Depth Anything and fitted to the painted floor), for the blur behind the enemy. */
  depth?: string;
  w: number;
  h: number;
  /** Share of the screen height the picture starts below (room for the HUD). */
  top: number;
  mapId: string;
  weather: Weather;
  time: TimeOfDay;
  cal: Calibration;
  look: Look;
  projectedStage?: StageProjection;
}

const BACK = Number(new URLSearchParams(location.search).get("back") ?? 0.45);

/** Art's grey salt hall (style 2): overcast white light through the broken roof, salt haze, wet floor. */
const GREY_HALL = {
  w: 941, h: 1672, top: 0, mapId: "saltHall", time: "day",
  cal: { fov: 55, camH: 2.0, feet: [0.572, 0.608], height: 0.212, me: { x: -0.15, y: 0.17, z: -0.66 }, back: BACK, fp: { camH: 1.65, near: 0.88 } },
  look: {
    exposure: 1.05, rim: 0xe8eef2, fill: 0xc7ccd0, fog: 0x8e9497, sky: 0xc3cacd, ground: 0x2b2d2e, puddle: 0,
    // light shafts from the broken roof, salt haze lit cold white
    lens: { ...BODYCAM, saturation: 0.55, tint: 0xf1f4f6, dof: 0.035, dofNear: 0.3, rays: 0.9, haze: 0.03, hazeColor: 0xdfe6ea },
  },
} satisfies Omit<Frame, "id" | "name" | "src" | "weather">;

const stage = (time: "morning" | "evening" | "night", extensionTint: number, extensionExposure: number): StageProjection => ({
  manifest: "img/battle-stage/manifest.json",
  textures: { clean: `img/battle-times/${time}/clean.webp`, equipment: `img/battle-times/${time}/hall.webp` },
  textures2x: { clean: `img/battle-times/${time}/clean.png`, equipment: `img/battle-times/${time}/hall.png` },
  extensionTint, extensionExposure,
});

const MORNING_LOOK: Look = {
  exposure: 0.94, rim: 0xc9ddf2, fill: 0xafc1d5, fog: 0x6d7b8a, sky: 0xabc2d8, ground: 0x222831, puddle: 0,
  lighting: { sky: 0.42, rim: 2.8, fill: 1.7, key: 3.7 }, sun: [-0.65, 0.27, -1],
  lens: { ...GREY_HALL.look.lens, contrast: 0.27, glow: 0.30, saturation: 0.58, tint: 0xdfeaf6, rays: 0.28, haze: 0.018, hazeColor: 0xa7bacd },
};
const EVENING_LOOK: Look = {
  exposure: 0.95, rim: 0xffd3a0, fill: 0xaebdca, fog: 0x928274, sky: 0xc4beb5, ground: 0x302a26, puddle: 0,
  lighting: { sky: 0.35, rim: 4.2, fill: 1.6, key: 3.4 }, sun: [0.74, 0.15, -1],
  lens: { ...GREY_HALL.look.lens, contrast: 0.28, glow: 0.40, saturation: 0.64, tint: 0xf5e6d5, rays: 0.58, haze: 0.019, hazeColor: 0xc9b094 },
};
const NIGHT_LOOK: Look = {
  exposure: 0.68, rim: 0xa7b9cb, fill: 0x8b98a5, fog: 0x141a21, sky: 0x6a7c8e, ground: 0x0b0e12, puddle: 0,
  lighting: { sky: 0.075, rim: 0.48, fill: 0.24, key: 0.52 }, sun: [0.5, 0.66, -1],
  lens: { ...GREY_HALL.look.lens, contrast: 0.19, vignette: 0.90, glow: 0.08, saturation: 0.40, tint: 0xd5dce3, rays: 0.015, haze: 0.003, hazeColor: 0x505d6a },
};

/** The approved daytime courtyard remains the lighting reference for the other three variants. */
const BLOCKS_LOOK: Look = {
  reflection: false,
  exposure: 0.98, rim: 0xf1e7d3, fill: 0xb6c2cc, fog: 0x9d9b93, sky: 0xc8d0d7, ground: 0x36332e, puddle: 0,
  lighting: { sky: 0.75, rim: 3.4, fill: 1.65, key: 4.4 }, sun: [0.10, 0.96, -0.5],
  lens: { ...GREY_HALL.look.lens, saturation: 0.58, tint: 0xe7eaf0, rays: 0.22, glow: 0.22, haze: 0.014, hazeColor: 0xc2c0b9 },
};

const blocksStage = (time: "morning" | "evening" | "night", extensionTint: number, extensionExposure: number, sideTint: number, sideExposure: number): StageProjection => ({
  manifest: "img/blocks-stage/manifest.json",
  textures: { clean: `img/blocks-times/${time}/clean.webp`, equipment: `img/blocks-times/${time}/hall.webp` },
  textures2x: { clean: `img/blocks-times/${time}/clean.png`, equipment: `img/blocks-times/${time}/hall.png` },
  extensionTint, extensionExposure, sideTint, sideExposure,
});

const BLOCKS_MORNING_LOOK: Look = {
  ...BLOCKS_LOOK,
  exposure: 0.94, rim: 0xe9d9bc, fill: 0xb3c3d2, fog: 0x989a94, sky: 0xb5c7d6, ground: 0x2e2b27,
  lighting: { sky: 0.55, rim: 2.9, fill: 1.4, key: 3.5 }, sun: [-0.55, 0.28, -1],
  lens: { ...BLOCKS_LOOK.lens, contrast: 0.26, saturation: 0.56, tint: 0xe1e7e9, rays: 0.17, glow: 0.17, haze: 0.011, hazeColor: 0xb5c2cc },
};
const BLOCKS_EVENING_LOOK: Look = {
  ...BLOCKS_LOOK,
  exposure: 0.88, rim: 0xffcf98, fill: 0x9faebc, fog: 0x817265, sky: 0xb3b8be, ground: 0x29231e,
  lighting: { sky: 0.32, rim: 3.5, fill: 1.0, key: 2.8 }, sun: [0.68, 0.12, -1],
  lens: { ...BLOCKS_LOOK.lens, contrast: 0.28, saturation: 0.58, tint: 0xf0dfcc, rays: 0.32, glow: 0.26, haze: 0.010, hazeColor: 0xb89770 },
};
const BLOCKS_NIGHT_LOOK: Look = {
  ...BLOCKS_LOOK,
  // Unaided exposure stays dark. The canonical NVG branch supplies its own gain.
  exposure: 0.62, rim: 0x8c949d, fill: 0x626970, fog: 0x0c1015, sky: 0x525c68, ground: 0x07090c,
  lighting: { sky: 0.045, rim: 0.25, fill: 0.115, key: 0.28 }, sun: [0.10, 0.72, -1],
  lens: { ...BLOCKS_LOOK.lens, contrast: 0.16, vignette: 0.88, saturation: 0.30, tint: 0xd6dce2, rays: 0, glow: 0.025, haze: 0.0015, hazeColor: 0x39434c },
};

export const FRAMES: Frame[] = [
  { ...GREY_HALL, id: "hall1", name: "Соляной цех", src: "img/grey-hall-1.jpg", src2x: "img/grey-hall-1@2x.jpg", depth: "img/grey-hall-1.depth.png", weather: "clear", projectedStage: { manifest: "img/battle-stage/manifest.json" } },
  { ...GREY_HALL, id: "hall-morning", name: "Соляной цех", src: "img/battle-times/morning/hall.webp", src2x: "img/battle-times/morning/hall.png", depth: "img/grey-hall-1.depth.png", weather: "clear", time: "morning", look: MORNING_LOOK, projectedStage: stage("morning", 0xc2d4ec, 0.64) },
  { ...GREY_HALL, id: "hall-evening", name: "Соляной цех", src: "img/battle-times/evening/hall.webp", src2x: "img/battle-times/evening/hall.png", depth: "img/grey-hall-1.depth.png", weather: "clear", time: "evening", look: EVENING_LOOK, projectedStage: stage("evening", 0xe7c09a, 0.65) },
  { ...GREY_HALL, id: "hall-night", name: "Соляной цех", src: "img/battle-times/night/hall.webp", src2x: "img/battle-times/night/hall.png", depth: "img/grey-hall-1.depth.png", weather: "clear", time: "night", look: NIGHT_LOOK, projectedStage: stage("night", 0x788898, 0.10) },
  { ...GREY_HALL, id: "hall2", name: "Цех в дымке", src: "img/grey-hall-2.jpg", src2x: "img/grey-hall-2@2x.jpg", depth: "img/grey-hall-2.depth.png", weather: "fog" },
  { ...GREY_HALL, id: "blocks-day", name: "Пятиэтажки", mapId: "blocks", time: "day", weather: "clear",
    src: "img/blocks-day/hall.webp", src2x: "img/blocks-day/hall.png", look: BLOCKS_LOOK,
    projectedStage: { manifest: "img/blocks-stage/manifest.json", extensionTint: 0xa9a99e, extensionExposure: 0.80 } },
  { ...GREY_HALL, id: "blocks-morning", name: "Пятиэтажки", mapId: "blocks", time: "morning", weather: "clear",
    src: "img/blocks-times/morning/hall.webp", src2x: "img/blocks-times/morning/hall.png", look: BLOCKS_MORNING_LOOK,
    projectedStage: blocksStage("morning", 0xb5c3ce, 0.55, 0xc0cdd6, 0.90) },
  { ...GREY_HALL, id: "blocks-evening", name: "Пятиэтажки", mapId: "blocks", time: "evening", weather: "clear",
    src: "img/blocks-times/evening/hall.webp", src2x: "img/blocks-times/evening/hall.png", look: BLOCKS_EVENING_LOOK,
    projectedStage: blocksStage("evening", 0xd7ac78, 0.42, 0xd7ac78, 0.60) },
  { ...GREY_HALL, id: "blocks-night", name: "Пятиэтажки", mapId: "blocks", time: "night", weather: "clear",
    src: "img/blocks-times/night/hall.webp", src2x: "img/blocks-times/night/hall.png", look: BLOCKS_NIGHT_LOOK,
    projectedStage: blocksStage("night", 0x7f8790, 0.06, 0x7f8790, 0.06) },
];

/** Art's fight screen with a painted soldier: the live enemy must cover him exactly. Open with ?cal. */
export const CALIBRATION_FRAME: Frame = { ...GREY_HALL, id: "cal", name: "Сверка", src: "img/cal-grey-hall.jpg", weather: "clear" };
