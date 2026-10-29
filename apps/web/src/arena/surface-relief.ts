import * as THREE from "three";
import { callRust } from "../rust/engine.ts";
import type { Quality } from "./quality.ts";

interface HeightEntry {
  relief: string; eligible: boolean; depthMeters: number; midpoint: number;
}
interface ReliefManifest {
  version: number;
  models: Record<string, { unitsPerMeter?: number; materials: Record<string, HeightEntry> }>;
}
interface ReliefData { depthUnits: number; midpoint: number; source: string }
interface Program { common: string; steps: number; refine: number; version: string }
const key = "cdSurfaceRelief";
const supported = new Set(["varyag", "tripo-helmet", "tripo-vest", "persona-authored"]);
const textureSlots = ["map", "normalMap", "roughnessMap", "metalnessMap", "aoMap", "emissiveMap", "specularColorMap", "specularIntensityMap"] as const;
let manifest: Promise<ReliefManifest | null> | undefined;
// Newly derived normal-to-height maps need human visual acceptance. Keep the
// original PBR default until that review; the comparison page opts in explicitly.
let previewEnabled = typeof location !== "undefined" && new URLSearchParams(location.search).get("relief") === "1";
export function setSurfaceReliefPreview(enabled: boolean): void { previewEnabled = enabled; }

/** Optional derived maps. A missing or unsuitable map always keeps original PBR. */
export async function installSurfaceRelief(root: THREE.Object3D, modelUrl: string): Promise<void> {
  const name = /([\w-]+)\.(?:json|glb)(?:\?.*)?$/.exec(modelUrl)?.[1];
  if (!name || !supported.has(name)) return;
  const base = import.meta.env?.BASE_URL ?? "/";
  manifest ??= fetch(`${base}tex/relief/manifest.json`).then(r => r.ok ? r.json() as Promise<ReliefManifest> : null).catch(() => null);
  const catalog = await manifest;
  const model = catalog?.version === 1 ? catalog.models[name] : undefined;
  if (!model) return;
  const materials = new Set<THREE.MeshStandardMaterial>();
  root.traverse(node => {
    if (!(node instanceof THREE.Mesh) || !node.geometry.getAttribute("uv")) return;
    for (const material of Array.isArray(node.material) ? node.material : [node.material]) {
      if ((material as THREE.MeshStandardMaterial).isMeshStandardMaterial) materials.add(material as THREE.MeshStandardMaterial);
    }
  });
  await Promise.all([...materials].map(async material => {
    const entry = model.materials[material.name];
    if (!entry?.eligible || !entry.relief || !material.normalMap || material.bumpMap || !compatibleUv(material)) return;
    if (!(entry.depthMeters > 0 && entry.depthMeters <= .001) || !(entry.midpoint >= .4 && entry.midpoint <= .6)) return;
    try {
      const height = await new THREE.TextureLoader().loadAsync(`${base}${entry.relief}`);
      const source = material.normalMap;
      height.flipY = source.flipY;
      height.channel = source.channel;
      height.wrapS = height.wrapT = THREE.ClampToEdgeWrapping;
      height.colorSpace = THREE.NoColorSpace;
      height.minFilter = THREE.LinearMipmapLinearFilter;
      height.magFilter = THREE.LinearFilter;
      height.generateMipmaps = true;
      height.anisotropy = 4;
      height.needsUpdate = true;
      // MeshStandardMaterial copies bumpMap on clone and existing resource collectors
      // own it. The original normalMap takes precedence; no second bump is applied.
      material.bumpMap = height; material.bumpScale = 0;
      material.userData[key] = { depthUnits: entry.depthMeters * (model.unitsPerMeter ?? 1), midpoint: entry.midpoint, source: entry.relief } satisfies ReliefData;
      material.needsUpdate = true;
    } catch { /* Derived relief is optional; keep the embedded original material. */ }
  }));
}

/** Require the same UV atlas and transform for every PBR channel that will shift. */
function compatibleUv(material: THREE.MeshStandardMaterial): boolean {
  const reference = material.normalMap!;
  if (reference.channel !== 0 || material.normalMapType !== THREE.TangentSpaceNormalMap) return false;
  reference.updateMatrix();
  for (const slot of textureSlots) {
    const texture = (material as unknown as Record<string, THREE.Texture | null>)[slot];
    if (!texture) continue;
    texture.updateMatrix();
    if (texture.channel !== reference.channel || texture.flipY !== reference.flipY || !texture.matrix.equals(reference.matrix)) return false;
  }
  // Current authored assets use identity transforms. Refuse unknown transform-scale
  // semantics instead of applying the physical relief depth to a different atlas.
  return reference.matrix.equals(new THREE.Matrix3());
}

/** Composes with the battle's shader tuning and survives Three material cloning. */
export function surfaceReliefHook(material: THREE.Material, quality: Quality): {
  compile: (shader: THREE.WebGLProgramParametersWithUniforms) => void; key: string;
} | undefined {
  const m = material as THREE.MeshStandardMaterial;
  const data = m.userData[key] as ReliefData | undefined;
  if (!previewEnabled || !data || !m.bumpMap || !m.normalMap || quality.id === "low" || !compatibleUv(m)) return;
  const program = callRust<Program>("graphics.surfacePom", { quality: quality.id });
  m.bumpMap.anisotropy = quality.anisotropy;
  return {
    key: `${program.version}-${quality.id}`,
    compile(shader) {
      Object.assign(shader.uniforms, {
        cdSurfaceHeight: { value: m.bumpMap }, cdSurfaceDepth: { value: data.depthUnits },
        cdSurfaceMidpoint: { value: data.midpoint }, cdSurfaceSteps: { value: program.steps }, cdSurfaceRefine: { value: program.refine },
      });
      shader.vertexShader = `varying float cdSurfaceScale;\n${shader.vertexShader}`.replace("#include <begin_vertex>", `#include <begin_vertex>
        cdSurfaceScale = (length(modelMatrix[0].xyz) + length(modelMatrix[1].xyz) + length(modelMatrix[2].xyz)) / 3.0;`);
      shader.fragmentShader = shader.fragmentShader.replace("#include <common>", `#include <common>\n${program.common}`)
        .replace("void main() {", `void main() {
          vec2 cdSurfaceDx = dFdx(vNormalMapUv), cdSurfaceDy = dFdy(vNormalMapUv);
          vec2 cdSurfaceUv = cdSurfaceReliefUv(vNormalMapUv, cdSurfaceDx, cdSurfaceDy, -vViewPosition, vViewPosition);`);
      // All appearance maps move together. Explicit original-UV gradients prevent
      // mip instability at divergent raymarch steps. The tangent frame continues to
      // use the original atlas derivatives; the authored normal is sampled at the hit.
      const chunks = ["map_fragment", "normal_fragment_maps", "roughnessmap_fragment", "metalnessmap_fragment", "aomap_fragment", "emissivemap_fragment", "lights_physical_fragment"] as const;
      for (const chunk of chunks) shader.fragmentShader = shader.fragmentShader.replace(`#include <${chunk}>`, THREE.ShaderChunk[chunk]);
      shader.fragmentShader = shader.fragmentShader.replace(/texture2D\(\s*(map|normalMap|roughnessMap|metalnessMap|aoMap|emissiveMap|specularColorMap|specularIntensityMap)\s*,\s*v\w+Uv\s*\)/g,
        "textureGrad( $1, cdSurfaceUv, cdSurfaceDx, cdSurfaceDy )");
    },
  };
}
