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
}

export interface Look {
  exposure: number;
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
}

/** The bodycam of Art's style: sharp middle, rainbow smear and a fisheye bend towards the edges, dark corners. */
const BODYCAM = { smear: 0.045, soft: 0.0025, barrel: 0, vignette: 0.97, glow: 0.45, contrast: 0.3, near: 2.5 };

export interface Frame {
  id: string;
  name: string;
  /** Picture, relative to the page, and a double-size copy for the high graphics levels. */
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
}

/** Art's grey salt hall (style 2): overcast white light through the broken roof, salt haze, wet floor. */
const GREY_HALL = {
  w: 941, h: 1672, top: 0, mapId: "saltHall", time: "day",
  cal: { fov: 55, camH: 2.0, feet: [0.572, 0.608], height: 0.212, me: { x: -0.15, y: 0.17, z: -0.66 } },
  look: {
    exposure: 1.05, rim: 0xe8eef2, fill: 0xc7ccd0, fog: 0x8e9497, sky: 0xc3cacd, ground: 0x2b2d2e, puddle: 0,
    // light shafts from the broken roof, salt haze lit cold white
    lens: { ...BODYCAM, saturation: 0.55, tint: 0xf1f4f6, dof: 0.055, dofNear: 0.3, rays: 0.9, haze: 0.03, hazeColor: 0xdfe6ea },
  },
} satisfies Omit<Frame, "id" | "name" | "src" | "weather">;

export const FRAMES: Frame[] = [
  { ...GREY_HALL, id: "hall1", name: "Соляной цех", src: "img/grey-hall-1.jpg", depth: "img/grey-hall-1.depth.png", weather: "clear" },
  { ...GREY_HALL, id: "hall2", name: "Цех в дымке", src: "img/grey-hall-2.jpg", src2x: "img/grey-hall-2@2x.jpg", depth: "img/grey-hall-2.depth.png", weather: "fog" },
];

/** Art's fight screen with a painted soldier: the live enemy must cover him exactly. Open with ?cal. */
export const CALIBRATION_FRAME: Frame = { ...GREY_HALL, id: "cal", name: "Сверка", src: "img/cal-grey-hall.jpg", weather: "clear" };
