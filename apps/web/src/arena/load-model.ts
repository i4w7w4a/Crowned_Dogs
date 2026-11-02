import * as THREE from "three";
import { MeshoptDecoder } from "three/addons/libs/meshopt_decoder.module.js";
import { GLTFLoader, type GLTF } from "three/addons/loaders/GLTFLoader.js";
import { KTX2Loader } from "three/addons/loaders/KTX2Loader.js";
import { callRust, sceneLoadModel, shipped } from "../../../../packages/runtime/src/index.ts";
export const PERSONA_MODEL: string = import.meta.env?.VITE_MODEL ?? "models/persona-male.json";
const baseUrl = import.meta.env?.BASE_URL ?? "/";

// The phone qualities load each model's copy with KTX2 maps (vite.config.ts). One transcoder
// serves every model; the GPU's formats are read once, on a throwaway context.
let transcoder: KTX2Loader | null | undefined;
function ktx2(): KTX2Loader | null {
  if (transcoder !== undefined) return transcoder;
  transcoder = null;
  const gl = document.createElement("canvas").getContext("webgl2");
  if (!gl) return null;
  try {
    const extensions = { has: (name: string) => gl.getExtension(name) !== null, get: (name: string) => gl.getExtension(name) };
    transcoder = new KTX2Loader().setTranscoderPath(`${baseUrl}basis/`).setWorkerLimit(2)
      .detectSupport({ extensions } as unknown as THREE.WebGLRenderer);
  } catch {
    transcoder = null;
  } finally {
    gl.getExtension("WEBGL_lose_context")?.loseContext();
  }
  return transcoder;
}

const host = {
  THREE, GLTFLoader, MeshoptDecoder, callRust, baseUrl, ktx2,
  hasFile: (url: string) => import.meta.env?.DEV === true || shipped(url),
};
export function loadModel(url: string): Promise<GLTF> { return sceneLoadModel(host, url); }
