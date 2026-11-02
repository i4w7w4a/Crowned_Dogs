import * as THREE from "three";
import { callRust, sceneSetReliefPreview, sceneInstallRelief, sceneReliefHook } from "../../../../packages/runtime/src/index.ts";
import type { Quality } from "./quality.ts";
const host = { THREE, callRust, baseUrl: import.meta.env?.BASE_URL ?? "/" };
export function setSurfaceReliefPreview(enabled: boolean): void { sceneSetReliefPreview(enabled); }
export function installSurfaceRelief(root: THREE.Object3D, modelUrl: string): Promise<void> { return sceneInstallRelief(host, root, modelUrl); }
export function surfaceReliefHook(material: THREE.Material, quality: Quality): { compile: (shader: THREE.WebGLProgramParametersWithUniforms) => void; key: string } | undefined {
  return sceneReliefHook(host, material, quality);
}
