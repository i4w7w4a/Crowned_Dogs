import * as THREE from "three";
import { FullScreenQuad } from "three/addons/postprocessing/Pass.js";
import { RustLens } from "../../../../packages/runtime/src/index.ts";

export interface LensLook {
  /** Radial smear and colour fan at the edges, as a share of the distance from the centre. */
  smear: number;
  /** Soft blur in the corners, as a share of the picture's height. */
  soft: number;
  /** Fisheye bend: how much further out the corners are sampled. */
  barrel: number;
  /** How much the edges darken, 0..1: the corners go to black at 1. */
  vignette: number;
  /** Glow bleeding from the bright windows and sky over everything near them. */
  glow: number;
  /** Extra contrast of the whole frame, 0..1. */
  contrast: number;
  /** How soft our own figure is: it is drawn at 1/near of the size. */
  near: number;
  /** Colour of the live figures against the painting: saturation (1 = as lit) and a tint. */
  saturation: number;
  tint: number;
  /** Depth of field around the enemy: blur radius (share of the picture height) per 1/m behind him, and how much of it in front. */
  dof: number;
  dofNear: number;
  /** Light shafts from the openings, 0..1, and the haze in the air (density per metre, colour of its light). */
  rays: number;
  haze: number;
  hazeColor: number;
}

/** The visible part of the picture, as fractions of the canvas from its top left. */
export interface View { x: number; y: number; w: number; h: number }

/** What the haze needs from the scene each frame. */
export interface HazeScene {
  /** Distance to the enemy along the view, metres. */
  enemyDepth: number;
  /** The light whose shadows show in the haze. */
  light: THREE.DirectionalLight | null;
}

/** Three.js is the renderer; Rust owns the render graph and texture lifetimes. */
export class Lens extends RustLens {
  // a WebGPU renderer (gpu.ts) brings its own THREE
  constructor(renderer: THREE.WebGLRenderer & { three?: object }) { super({ THREE: renderer.three ?? THREE, FullScreenQuad }, renderer); }
}
