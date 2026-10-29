import * as THREE from "three";
import { MeshoptDecoder } from "three/addons/libs/meshopt_decoder.module.js";
import { GLTFLoader, type GLTF } from "three/addons/loaders/GLTFLoader.js";
import { callRust, sceneLoadModel } from "../../../../packages/runtime/src/index.ts";
export const PERSONA_MODEL: string = import.meta.env?.VITE_MODEL ?? "models/persona-male.json";
const host = { THREE, GLTFLoader, MeshoptDecoder, callRust, baseUrl: import.meta.env?.BASE_URL ?? "/" };
export function loadModel(url: string): Promise<GLTF> { return sceneLoadModel(host, url); }
