// Small changes to three.js materials for a calm picture: soft shadows made of many
// fixed looks into the shadow map instead of five noisy ones (the noise crawled over the
// moving bodies), and shiny bumpy surfaces turned a touch rougher where the bumps are
// finer than a pixel, so highlights stop sparkling.

import * as THREE from "three";
import { callRust } from "../../../../packages/runtime/src/index.ts";
import type { Quality } from "./quality.ts";
import { surfaceReliefHook } from "./surface-relief.ts";

const PCF = /float phi = interleavedGradientNoise\( gl_FragCoord\.xy \) \* PI2;\s*shadow = \([\s\S]*?\) \* 0\.2;/;

/** Fits a material to the graphics level; `extra` adds its own shader changes, named by `extraKey` (materials
 *  with different changes must not share a compiled program). */
export function tune(m: THREE.Material, q: Quality, extra?: (shader: THREE.WebGLProgramParametersWithUniforms) => void, extraKey = "x"): void {
  const taps = Math.max(5, q.shadowSamples);
  const snippets = callRust<{shadow: string; specular: string}>("motion.materialShaders", [taps]);
  const standard = (m as THREE.MeshStandardMaterial).isMeshStandardMaterial === true;
  const relief = standard ? surfaceReliefHook(m, q) : undefined;
  m.onBeforeCompile = (shader) => {
    shader.fragmentShader = shader.fragmentShader.replace("#include <shadowmap_pars_fragment>", THREE.ShaderChunk.shadowmap_pars_fragment.replace(PCF, snippets.shadow));
    if (standard) shader.fragmentShader = shader.fragmentShader.replace("#include <lights_physical_fragment>", snippets.specular);
    extra?.(shader);
    relief?.compile(shader);
  };
  const key = `cd-${taps}-${extra ? extraKey : ""}-${relief?.key ?? ""}`;
  m.customProgramCacheKey = () => key;
  if (standard) (m as THREE.MeshStandardMaterial).envMapIntensity = q.env ? 0.35 : 0;
  m.needsUpdate = true;
}
