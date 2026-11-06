/// <reference types="vite/client" />

interface ImportMetaEnv {
  readonly VITE_ONLINE_API?: string;
  /** One figure model for both fighters (.glb, or .json with the .glb inside as base64). Default: the male Persona
   *  for us and the female one for the enemy, built from models/persona-male.glb and models/persona-female.glb. */
  readonly VITE_MODEL?: string;
}
