import * as THREE from "three";
import { sceneDress } from "../../../../packages/runtime/src/index.ts";
/** Per slot an item id, `true` for the plain kit item, or nothing. */
export type Wear = Partial<Record<"head" | "body" | "suit" | "boots", string | boolean | null>>;
export const KIT_A: Wear = { suit: true, body: true, head: true, boots: true };
export function dress(model: THREE.Object3D, wear: Wear): void { sceneDress({ THREE }, model, wear); }
