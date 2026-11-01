import * as THREE from "three";
import type { Loadout } from "@crowned-dogs/core";
import { callRust, graphicsTexture, sceneMakeCarry, sceneMakeFittingStage, sceneMakeSling, scenePrepareVaryag, sceneFitVaryag } from "../../../../packages/runtime/src/index.ts";
import type { QualityId } from "../arena/quality.ts";
export const PLATFORM_TOP = 0.145;
export interface FittingStage { fixed: THREE.Group; rotating: THREE.Group; feet: THREE.Group; setQuality: (quality: QualityId) => void }
const host = { THREE, callRust, graphicsTexture };
export function makeFittingStage(): FittingStage { return sceneMakeFittingStage(host); }
export function makeSling(mount: THREE.Object3D, pistol = false): THREE.Mesh { return sceneMakeSling(host, mount, pistol); }
/** Moves the gun to where this Persona's fitting pose carries it; the group holds straps, clips and holster. */
export function makeCarry(mount: THREE.Object3D, sex: string, weapon: string): THREE.Group | null { return sceneMakeCarry(host, mount, sex, weapon); }
export function prepareVaryag(gun: THREE.Object3D): void { scenePrepareVaryag(gun); }
export function fitVaryag(gun: THREE.Object3D, loadout: Loadout | null): void { sceneFitVaryag(host, gun, loadout); }
