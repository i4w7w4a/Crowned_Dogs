// Three.js and scene helpers cross this boundary as opaque SDK handles.
// The Rust arena owns figures, quality, async loading and each animation frame.
import type { Zone } from "@crowned-dogs/core";
import { RustArena } from "../../../../packages/runtime/src/index.ts";
import * as THREE from "three";
import { clone } from "three/addons/utils/SkeletonUtils.js";
import { dress, KIT_A } from "./dress.ts";
import { Dust } from "./dust.ts";
import type { Frame } from "./frames.ts";
import { Lens, type View } from "./lens.ts";
import { tune } from "./materials.ts";
import { Motion, Pose, type Attack, type Stance } from "./motion.ts";
import { QUALITY, type Quality } from "./quality.ts";
import { armsOnly, findBones, pitch } from "./rig.ts";
import { Collapse, FALL_TIME } from "./fall.ts";
import { Flash, HoldOut } from "./hold.ts";
import { Feet, gaitOf } from "./ik.ts";
import { Viewmodel } from "./viewmodel.ts";
import { TextureTiers } from "./textures.ts";
import { loadModel } from "./load-model.ts";
import { Hand, gripOf, loadGuns } from "./weapons.ts";

export type Who = "me" | "enemy";
export type HitKind = "hit" | "blocked" | "grenade";

export class Arena {
  private readonly engine: RustArena;
  constructor(canvas: HTMLCanvasElement) {
    const query = new URLSearchParams(location.search);
    this.engine = new RustArena({
      THREE, clone, dress, KIT_A, Dust, Lens, tune, Motion, Pose,
      armsOnly, findBones, pitch, Collapse, FALL_TIME, Flash, HoldOut,
      Feet, gaitOf, Viewmodel, TextureTiers, loadModel, Hand, gripOf, loadGuns,
      quality: QUALITY.medium,
    }, canvas, {
      fpview: query.get("fpview")?.split(",").map(Number),
      wide: Number(query.get("wide") ?? 0.62), fpkey: Number(query.get("fpkey") ?? 2.5),
    });
  }
  get ready(): boolean { return this.engine.ready; }
  get onFrame(): (() => void) | null { return this.engine.onFrame as (() => void) | null; }
  set onFrame(callback: (() => void) | null) { this.engine.onFrame = callback; }
  get gl(): WebGL2RenderingContext { return this.engine.gl as WebGL2RenderingContext; }
  get texturesDone(): Promise<unknown> { return this.engine.texturesDone; }
  load(meUrl: string, enemyUrl: string, distance: number): Promise<void> { return this.engine.load(meUrl, enemyUrl, distance) as Promise<void>; }
  resume(): void { this.engine.resume(); }
  pause(): void { this.engine.pause(); }
  dispose(): void { this.engine.dispose(); }
  setQuality(quality: Quality): void { this.engine.setQuality(quality); }
  setFrame(frame: Frame): void { this.engine.setFrame(frame); }
  resize(width: number, height: number, view: View = { x: 0, y: 0, w: 1, h: 1 }): void { this.engine.resize(width, height, view); }
  arm(who: Who, gun: Stance, mods: Partial<Record<string, string>> = {}): void { this.engine.arm(who, gun, mods); }
  move(distance: number, meMove: number, enemyMove: number): void { this.engine.move(distance, meMove, enemyMove); }
  attack(who: Who, kind: Attack, inSeconds: number): void { this.engine.attack(who, kind, inSeconds); }
  hit(who: Who, zone: Zone, kind: HitKind): void { this.engine.hit(who, zone, kind); }
  fall(who: Who): void { this.engine.fall(who); }
  setFirstPerson(on: boolean): void { this.engine.setFirstPerson(on); }
  reset(distance: number): void { this.engine.reset(distance); }
  zoneOnScreen(zone: Zone): [number, number] { return this.engine.zoneOnScreen(zone) as [number, number]; }
}
