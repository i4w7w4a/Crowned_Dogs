/// <reference types="vite/client" />

interface ImportMetaEnv {
  /** The figure model (.glb, or .json with the .glb inside as base64). Default: models/persona-male.json, built from models/persona-male.glb. */
  readonly VITE_MODEL?: string;
}
