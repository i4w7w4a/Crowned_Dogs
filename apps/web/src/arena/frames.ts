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
  rim: number;
  fill: number;
  fog: number;
  /** The bodycam lens over the whole frame, and how the live figures are graded to sit in the painting. */
  lens: LensLook;
}

/** The bodycam of the new style: colours part and soften at the edges, the corners darken a little. */
const BODYCAM = { split: 0.014, blur: 3, vignette: 0.3 };

export interface Frame {
  id: string;
  name: string;
  /** Picture, relative to the page. */
  src: string;
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

const SALT_HALL = {
  w: 1024, h: 1536, top: 0.1, mapId: "saltHall", weather: "dust", time: "evening",
  cal: { fov: 45, camH: 1.55, feet: [0.645, 0.612], height: 0.374, me: { x: -0.4, y: -0.08, z: -1.9 } },
  look: { exposure: 1.0, rim: 0xffb06a, fill: 0xd9c3a3, fog: 0x3d342c, lens: { ...BODYCAM, saturation: 1, tint: 0xffffff } },
} satisfies Omit<Frame, "id" | "name" | "src">;

export const FRAMES: Frame[] = [
  { ...SALT_HALL, id: "salt1", name: "Цех", src: "img/salt-hall-empty-1.jpg" },
  { ...SALT_HALL, id: "salt2", name: "Цех в дымке", src: "img/salt-hall-empty-2.jpg" },
];

/** The original concept frame with a painted soldier: the mannequin must cover him exactly. Open with ?cal. */
export const CALIBRATION_FRAME: Frame = { ...SALT_HALL, id: "cal", name: "Сверка", src: "img/cal-salt-hall.jpg" };
