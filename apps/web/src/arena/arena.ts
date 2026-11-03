// Three.js and scene helpers cross this boundary as opaque SDK handles.
// The Rust arena owns figures, quality, async loading and each animation frame.
import { RustArena } from "../../../../packages/runtime/src/index.ts";
import * as THREE from "three";
import { clone } from "three/addons/utils/SkeletonUtils.js";
import { dress, KIT_A } from "./dress.ts";
import { Dust } from "./dust.ts";
import { Lens, type View } from "./lens.ts";
import { ProjectedStage } from "./projected-stage.ts";
import { tune } from "./materials.ts";
import { Motion, Pose, type Stance } from "./motion.ts";
import { QUALITY } from "./quality.ts";
import { armsOnly, findBones, pitch } from "./rig.ts";
import { Collapse, FALL_TIME } from "./fall.ts";
import { Flash, HoldOut } from "./hold.ts";
import { Feet, gaitOf } from "./ik.ts";
import { Viewmodel } from "./viewmodel.ts";
import { TextureTiers } from "./textures.ts";
import { loadModel } from "./load-model.ts";
import { Hand, gripOf, loadGuns } from "./weapons.ts";

// ?gpu=1 draws the arena with WebGPU where the browser has it (see gpu.ts); ?gpu=0 keeps WebGL 2
const GPU_FLAG = new URLSearchParams(location.search).get("gpu");
const GPU = GPU_FLAG !== null && GPU_FLAG !== "0" && "gpu" in navigator ? await import("./gpu.ts") : null;

export type Who = "me" | "enemy";
export type HitKind = "hit" | "blocked" | "grenade";

export class Arena extends RustArena {

  constructor(canvas: HTMLCanvasElement) {
    const query = new URLSearchParams(location.search);
    super({
      THREE: GPU?.THREE ?? THREE, clone, dress, KIT_A, Dust: GPU?.Dust ?? Dust, Lens, ProjectedStage, tune: GPU?.tune ?? tune, Motion, Pose,
      armsOnly, findBones, pitch, Collapse, FALL_TIME, Flash, HoldOut,
      Feet, gaitOf, Viewmodel, TextureTiers, loadModel, Hand, gripOf, loadGuns,
      quality: QUALITY.medium,
    }, canvas, {
      fpview: query.get("fpview")?.split(",").map(Number),
      wide: Number(query.get("wide") ?? 0.62), fpkey: Number(query.get("fpkey") ?? 2.5),
      motionDebug: query.has("motionDebug"),
    });
  }
  declare readonly gl: WebGL2RenderingContext;
  declare onFrame: (() => void) | null;
  override resize(width: number, height: number, view: View = { x: 0, y: 0, w: 1, h: 1 }): void { super.resize(width, height, view); }
  override arm(who: Who, gun: Stance, mods: Partial<Record<string, string>> = {}): void { super.arm(who, gun, mods); }
}
