// Painted backdrops the 3D figures stand in. Each frame is a pre-rendered picture
// plus how its painted camera sees the floor, so live figures line up with it.

import type { TimeOfDay, Weather } from "@crowned-dogs/core";

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
}

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
  look: { exposure: 1.0, rim: 0xffb06a, fill: 0xd9c3a3, fog: 0x3d342c },
} satisfies Omit<Frame, "id" | "name" | "src">;

export const FRAMES: Frame[] = [
  { ...SALT_HALL, id: "salt1", name: "Цех", src: "img/salt-hall-empty-1.jpg" },
  { ...SALT_HALL, id: "salt2", name: "Цех в дымке", src: "img/salt-hall-empty-2.jpg" },
];

/** The original concept frame with a painted soldier: the mannequin must cover him exactly. Open with ?cal. */
export const CALIBRATION_FRAME: Frame = { ...SALT_HALL, id: "cal", name: "Сверка", src: "img/cal-salt-hall.jpg" };
