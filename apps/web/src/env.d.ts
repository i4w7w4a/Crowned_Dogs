/// <reference types="vite/client" />

interface ImportMetaEnv {
  /** The figure model (.glb, or .json with the .glb inside as base64). Default: models/mannequin.json next to the page. */
  readonly VITE_MODEL?: string;
}
