// Small changes to three.js materials for a calm picture: soft shadows made of many
// fixed looks into the shadow map instead of five noisy ones (the noise crawled over the
// moving bodies), and shiny bumpy surfaces turned a touch rougher where the bumps are
// finer than a pixel, so highlights stop sparkling.

import * as THREE from "three";
import type { Quality } from "./quality.ts";

const PCF = /float phi = interleavedGradientNoise\( gl_FragCoord\.xy \) \* PI2;\s*shadow = \([\s\S]*?\) \* 0\.2;/;

/** The shadow chunk with `taps` looks into the map in a fixed spiral (a noisy turned one when there are few). */
function shadowChunk(taps: number): string {
  const phi = taps >= 12 ? "0.0" : "interleavedGradientNoise( gl_FragCoord.xy ) * PI2";
  return THREE.ShaderChunk.shadowmap_pars_fragment.replace(PCF, `float phi = ${phi};
				shadow = 0.0;
				for ( int i = 0; i < ${taps}; i ++ ) shadow += texture( shadowMap, vec3( shadowCoord.xy + vogelDiskSample( i, ${taps}, phi ) * radius, shadowCoord.z ) );
				shadow /= ${taps}.0;`);
}

const SPECULAR_AA = /* glsl */ `#include <lights_physical_fragment>
{
  vec3 nDx = dFdx( normal ), nDy = dFdy( normal );
  float kernel = min( 0.5 * ( dot( nDx, nDx ) + dot( nDy, nDy ) ), 0.18 );
  material.roughness = sqrt( clamp( material.roughness * material.roughness + kernel, 0.0, 1.0 ) );
}`;

/** Fits a material to the graphics level; `extra` adds its own shader changes, named by `extraKey` (materials
 *  with different changes must not share a compiled program). */
export function tune(m: THREE.Material, q: Quality, extra?: (shader: THREE.WebGLProgramParametersWithUniforms) => void, extraKey = "x"): void {
  const taps = Math.max(5, q.shadowSamples);
  const standard = (m as THREE.MeshStandardMaterial).isMeshStandardMaterial === true;
  m.onBeforeCompile = (shader) => {
    shader.fragmentShader = shader.fragmentShader.replace("#include <shadowmap_pars_fragment>", shadowChunk(taps));
    if (standard) shader.fragmentShader = shader.fragmentShader.replace("#include <lights_physical_fragment>", SPECULAR_AA);
    extra?.(shader);
  };
  const key = `cd-${taps}-${extra ? extraKey : ""}`;
  m.customProgramCacheKey = () => key;
  if (standard) (m as THREE.MeshStandardMaterial).envMapIntensity = q.env ? 0.35 : 0;
  m.needsUpdate = true;
}
