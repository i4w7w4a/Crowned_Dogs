import assert from "node:assert/strict";
import { test } from "node:test";
import * as THREE from "three";
import { setSurfaceReliefPreview, surfaceReliefHook } from "../src/arena/surface-relief.ts";
import { tune } from "../src/arena/materials.ts";
import { QUALITY } from "../src/arena/quality.ts";

function material(): THREE.MeshPhysicalMaterial {
  const texture = () => { const value = new THREE.Texture(); value.flipY = false; return value; };
  const result = new THREE.MeshPhysicalMaterial({ map: texture(), normalMap: texture(), roughnessMap: texture(), metalnessMap: texture(), aoMap: texture(),
    specularColorMap: texture(), specularIntensityMap: texture(), bumpMap: texture(), bumpScale: 0 });
  result.userData.cdSurfaceRelief = { depthUnits: .00035, midpoint: 128 / 255, source: "test" };
  return result;
}

test("surface relief is opt-in, clones keep maps, low and incompatible UVs retain original PBR", () => {
  const source = material(), clone = source.clone();
  setSurfaceReliefPreview(false);
  assert.equal(surfaceReliefHook(clone, QUALITY.ultra), undefined);
  setSurfaceReliefPreview(true);
  assert.equal(clone.bumpMap, source.bumpMap);
  assert.ok(surfaceReliefHook(clone, QUALITY.ultra));
  assert.equal(surfaceReliefHook(clone, QUALITY.low), undefined);
  clone.normalMap!.channel = 1;
  assert.equal(surfaceReliefHook(clone, QUALITY.ultra), undefined);
  clone.normalMap!.channel = 0;
  clone.roughnessMap!.offset.x = .125;
  assert.equal(surfaceReliefHook(clone, QUALITY.ultra), undefined);
  setSurfaceReliefPreview(false);
});

test("surface relief composes battle tuning and shifts every PBR channel with stable gradients", () => {
  setSurfaceReliefPreview(true);
  const source = material();
  tune(source, QUALITY.ultra, shader => { shader.fragmentShader += "\n// existing-extra-hook"; }, "existing");
  const shader = { vertexShader: THREE.ShaderLib.physical.vertexShader, fragmentShader: THREE.ShaderLib.physical.fragmentShader, uniforms: {} } as THREE.WebGLProgramParametersWithUniforms;
  source.onBeforeCompile(shader, {} as THREE.WebGLRenderer);
  assert.match(shader.fragmentShader, /existing-extra-hook/);
  assert.match(source.customProgramCacheKey(), /existing.*cd-surface-relief/);
  for (const name of ["map", "normalMap", "roughnessMap", "metalnessMap", "aoMap", "specularColorMap", "specularIntensityMap"]) {
    assert.ok(shader.fragmentShader.includes(`textureGrad( ${name}, cdSurfaceUv, cdSurfaceDx, cdSurfaceDy )`), name);
  }
  assert.ok(shader.fragmentShader.includes("#include <normal_fragment_begin>"), "original tangent-frame calculation remains unchanged");
  assert.equal(shader.fragmentShader.includes("gl_FragDepth"), false, "surface effect must not claim geometric or silhouette displacement");
  assert.equal(shader.vertexShader.includes("displacementScale ="), false);
  assert.equal(shader.uniforms.cdSurfaceHeight?.value, source.bumpMap);
  // Quality retuning replaces the composed hook rather than accumulating it.
  tune(source, QUALITY.high);
  const next = { vertexShader: THREE.ShaderLib.physical.vertexShader, fragmentShader: THREE.ShaderLib.physical.fragmentShader, uniforms: {} } as THREE.WebGLProgramParametersWithUniforms;
  source.onBeforeCompile(next, {} as THREE.WebGLRenderer);
  assert.equal(next.fragmentShader.split("vec2 cdSurfaceReliefUv(").length - 1, 1);
  assert.equal(next.fragmentShader.includes("existing-extra-hook"), false);
  assert.match(source.customProgramCacheKey(), /cd-surface-relief-v1-high/);
  setSurfaceReliefPreview(false);
  tune(source, QUALITY.high);
  const original = { vertexShader: THREE.ShaderLib.physical.vertexShader, fragmentShader: THREE.ShaderLib.physical.fragmentShader, uniforms: {} } as THREE.WebGLProgramParametersWithUniforms;
  source.onBeforeCompile(original, {} as THREE.WebGLRenderer);
  assert.equal(original.fragmentShader.includes("cdSurfaceReliefUv"), false);
});
