import * as THREE from "three";
import { callRust, graphicsTexture } from "../rust/engine.ts";
import { tune } from "../arena/materials.ts";
import type { Quality, QualityId } from "../arena/quality.ts";

interface ReliefProgram { common: string; trace: string; normal: string; version: string }
interface ReliefQuality { steps: number; refine: number; shadowSteps: number; textureSize: number; enabled: boolean }
type Compile = (shader: THREE.WebGLProgramParametersWithUniforms) => void;
const reliefHooks = new WeakMap<THREE.Material, { compile: Compile; key: string; quality: (id: QualityId) => void }>();

/** The Rust generator authors the maps; this adapter only uploads pixel buffers. */
export function turntableMaterial(): THREE.MeshStandardMaterial {
  const size = 1024;
  const texture = (kind: string) => {
    const result = new THREE.DataTexture(graphicsTexture(kind, size), size, size, THREE.RGBAFormat);
    result.magFilter = THREE.LinearFilter;
    result.minFilter = THREE.LinearMipmapLinearFilter;
    result.generateMipmaps = true;
    result.anisotropy = 4;
    result.needsUpdate = true;
    return result;
  };
  const map = texture("steel-color"), roughnessMap = texture("steel-roughness"), bumpMap = texture("steel-relief");
  map.colorSpace = THREE.SRGBColorSpace;
  return new THREE.MeshStandardMaterial({ color: 0xb5b8ba, map, roughness: .98, roughnessMap, metalness: .42, bumpMap, bumpScale: .004 });
}

/** Keep the battle's material tuning and the Rust relief shader in the same compile hook. */
export function tuneFittingMaterial(material: THREE.Material, quality: Quality): void {
  const hook = reliefHooks.get(material);
  hook?.quality(quality.id);
  tune(material, quality, hook?.compile, hook?.key);
}

/** Box proxy + true finite ray-heightfield intersection, with a normal mesh on low. */
export function silhouetteDeck(source: THREE.MeshStandardMaterial, radius: number, height: number): {
  group: THREE.Group; setQuality: (quality: QualityId) => void;
} {
  const program = callRust<ReliefProgram>("graphics.pomShaders", {});
  const material = source.clone();
  material.name = "Rust_Silhouette_Relief_Steel";
  material.bumpMap = null;
  // Relief rays must enter the volume from the light-facing proxy surface as well.
  material.shadowSide = THREE.FrontSide;
  const mesh = new THREE.Mesh(new THREE.BoxGeometry(radius * 2, height, radius * 2), material);
  mesh.name = "Silhouette_POM_Finite_Volume";
  mesh.castShadow = mesh.receiveShadow = true;
  const fallback = new THREE.Mesh(new THREE.CylinderGeometry(radius, radius, height, 96), source);
  fallback.name = "Low_Quality_Steel_Deck";
  fallback.castShadow = fallback.receiveShadow = true;
  const group = new THREE.Group(); group.add(mesh, fallback);

  const uniforms = {
    cdPomRelief: { value: source.bumpMap }, cdPomSize: { value: new THREE.Vector2(radius, height / 2) },
    cdPomCameraLocal: { value: new THREE.Vector3() }, cdPomRayLocal: { value: new THREE.Vector3() },
    cdPomOrthographic: { value: false }, cdPomSteps: { value: 28 }, cdPomRefine: { value: 5 }, cdPomShadowSteps: { value: 10 }, cdPomTexel: { value: 1 / 1024 },
    cdPomModelView: { value: new THREE.Matrix4() }, cdPomModelWorld: { value: new THREE.Matrix4() },
    cdPomProjection: { value: new THREE.Matrix4() }, cdPomNormalMatrix: { value: new THREE.Matrix3() },
    cdPomViewToLocal: { value: new THREE.Matrix3() },
  };
  const inverseWorld = new THREE.Matrix4(), inverseView = new THREE.Matrix4();
  const updateCamera = (camera: THREE.Camera): void => {
    inverseWorld.copy(mesh.matrixWorld).invert();
    uniforms.cdPomCameraLocal.value.setFromMatrixPosition(camera.matrixWorld).applyMatrix4(inverseWorld);
    camera.getWorldDirection(uniforms.cdPomRayLocal.value).transformDirection(inverseWorld);
    uniforms.cdPomOrthographic.value = (camera as THREE.OrthographicCamera).isOrthographicCamera === true;
    uniforms.cdPomModelWorld.value.copy(mesh.matrixWorld);
    uniforms.cdPomModelView.value.multiplyMatrices(camera.matrixWorldInverse, mesh.matrixWorld);
    uniforms.cdPomProjection.value.copy(camera.projectionMatrix);
    uniforms.cdPomNormalMatrix.value.getNormalMatrix(uniforms.cdPomModelView.value);
    inverseView.copy(uniforms.cdPomModelView.value).invert();
    uniforms.cdPomViewToLocal.value.setFromMatrix4(inverseView);
  };
  mesh.onBeforeRender = (_renderer, _scene, camera) => updateCamera(camera);
  mesh.onBeforeShadow = (_renderer, _object, _camera, shadowCamera) => updateCamera(shadowCamera);

  const patchCommon: Compile = (shader) => {
    Object.assign(shader.uniforms, uniforms);
    shader.vertexShader = `varying vec3 cdPomLocal;\n${shader.vertexShader}`
      .replace("#include <begin_vertex>", "#include <begin_vertex>\ncdPomLocal = transformed;");
    shader.fragmentShader = shader.fragmentShader
      .replace("#include <common>", `#include <common>\n${program.common}`)
      .replace("void main() {", `void main() {\n${program.trace}`);
  };
  const compile: Compile = (shader) => {
    patchCommon(shader);
    const shadowMatrices = `
#if defined(USE_SHADOWMAP) && NUM_DIR_LIGHT_SHADOWS > 0
uniform mat4 directionalShadowMatrix[NUM_DIR_LIGHT_SHADOWS];
#endif
#if defined(USE_SHADOWMAP) && NUM_POINT_LIGHT_SHADOWS > 0
uniform mat4 pointShadowMatrix[NUM_POINT_LIGHT_SHADOWS];
#endif
#if NUM_SPOT_LIGHT_COORDS > 0
uniform mat4 spotLightMatrix[NUM_SPOT_LIGHT_COORDS];
#endif
`;
    let lighting = THREE.ShaderChunk.lights_fragment_begin.replaceAll("vViewPosition", "cdPomViewPosition")
      .replaceAll("vDirectionalShadowCoord[ i ]", "(vDirectionalShadowCoord[ i ] + directionalShadowMatrix[ i ] * vec4(cdPomWorldDelta, 0.0))")
      .replaceAll("vPointShadowCoord[ i ]", "(vPointShadowCoord[ i ] + pointShadowMatrix[ i ] * vec4(cdPomWorldDelta, 0.0))")
      .replaceAll("vSpotLightCoord[ i ]", "(vSpotLightCoord[ i ] + spotLightMatrix[ i ] * vec4(cdPomWorldDelta, 0.0))");
    lighting = lighting.replaceAll("RE_Direct( directLight,", "directLight.color *= cdPomLightVisibility(directLight.direction);\n\t\tRE_Direct( directLight,");
    shader.fragmentShader = shader.fragmentShader
      .replace("#include <map_fragment>", THREE.ShaderChunk.map_fragment.replaceAll("vMapUv", "cdPomUv"))
      .replace("#include <roughnessmap_fragment>", THREE.ShaderChunk.roughnessmap_fragment.replaceAll("vRoughnessMapUv", "cdPomUv"))
      .replace("#include <normal_fragment_begin>", program.normal)
      .replace("#include <lights_fragment_begin>", lighting)
      .replace("#include <common>", `#include <common>\n${shadowMatrices}`);
  };
  const depth = new THREE.MeshDepthMaterial({ depthPacking: THREE.RGBADepthPacking });
  depth.onBeforeCompile = (shader) => {
    patchCommon(shader);
    shader.fragmentShader = shader.fragmentShader.replace(/float fragCoordZ = [^;]+;/g, "float fragCoordZ = cdPomDepth;");
  };
  depth.customProgramCacheKey = () => `${program.version}-depth`;
  mesh.customDepthMaterial = depth;
  const distance = new THREE.MeshDistanceMaterial();
  distance.onBeforeCompile = (shader) => {
    patchCommon(shader);
    shader.fragmentShader = shader.fragmentShader.replace("length( vWorldPosition - referencePosition )", "length( cdPomWorldHit - referencePosition )");
  };
  distance.customProgramCacheKey = () => `${program.version}-distance`;
  mesh.customDistanceMaterial = distance;
  material.addEventListener("dispose", () => { depth.dispose(); distance.dispose(); });

  const setQuality = (quality: QualityId): void => {
    const selected = callRust<ReliefQuality>("graphics.pomQuality", { quality });
    mesh.visible = selected.enabled; fallback.visible = !selected.enabled;
    uniforms.cdPomSteps.value = selected.steps; uniforms.cdPomRefine.value = selected.refine; uniforms.cdPomShadowSteps.value = selected.shadowSteps;
    group.userData.reliefQuality = quality;
    group.userData.reliefEnabled = selected.enabled;
  };
  reliefHooks.set(material, { compile, key: program.version, quality: setQuality });
  material.onBeforeCompile = compile;
  material.customProgramCacheKey = () => program.version;
  setQuality("medium");
  return { group, setQuality };
}
