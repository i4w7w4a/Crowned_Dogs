/// <reference types="vite/client" />

interface ImportMetaEnv {
  /** Folder with music/, sfx/ and models/, ending in a slash. Default: media/ next to the page. */
  readonly VITE_ASSETS?: string;
  /** The figure model (.glb, or .json with the .glb inside as base64). */
  readonly VITE_MODEL?: string;
}
