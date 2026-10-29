// Figure models come as .glb, or as .json with the .glb inside as base64 (artifact pages do not serve .glb).
// Kept apart from arena.ts so the base screen can load a Persona without pulling in the whole arena.

import { MeshoptDecoder } from "three/addons/libs/meshopt_decoder.module.js";
import { GLTFLoader, type GLTF } from "three/addons/loaders/GLTFLoader.js";
import { installSurfaceRelief } from "./surface-relief.ts";

/** The Persona both screens show when no other model is set for the build. */
export const PERSONA_MODEL: string = import.meta.env?.VITE_MODEL ?? "models/persona-male.json";

/** Fetches and parses a figure model: a .glb, or a .json with the .glb inside as base64. */
export async function loadModel(url: string): Promise<GLTF> {
  const res = await fetch(url);
  if (!res.ok) throw new Error(`model: ${res.status}`);
  let bytes: ArrayBuffer;
  if (url.endsWith(".json")) {
    const bin = atob(((await res.json()) as { data: string }).data);
    const arr = new Uint8Array(bin.length);
    for (let i = 0; i < bin.length; i++) arr[i] = bin.charCodeAt(i);
    bytes = arr.buffer;
  } else {
    bytes = await res.arrayBuffer();
  }
  const gltf = await new GLTFLoader().setMeshoptDecoder(MeshoptDecoder).parseAsync(bytes, "");
  await installSurfaceRelief(gltf.scene, url).catch(() => { /* Original PBR remains available if optional relief metadata is unavailable. */ });
  return gltf;
}
