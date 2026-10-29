import * as THREE from "three";
import { callRust, graphicsTexture, sceneTurntableMaterial, sceneTuneFittingMaterial, sceneSilhouetteDeck } from "../../../../packages/runtime/src/index.ts";
import type { Quality, QualityId } from "../arena/quality.ts";
const host = { THREE, callRust, graphicsTexture };
export function turntableMaterial(): THREE.MeshStandardMaterial { return sceneTurntableMaterial(host); }
export function tuneFittingMaterial(material: THREE.Material, quality: Quality): void { sceneTuneFittingMaterial(host, material, quality); }
export function silhouetteDeck(source: THREE.MeshStandardMaterial, radius: number, height: number): { group: THREE.Group; setQuality: (quality: QualityId) => void } {
  return sceneSilhouetteDeck(host, source, radius, height);
}
