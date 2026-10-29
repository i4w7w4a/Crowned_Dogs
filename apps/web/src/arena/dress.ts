import * as THREE from "three";
import { sceneDress } from "../../../../packages/runtime/src/index.ts";
export interface Wear { suit: boolean; body: boolean; head: boolean }
export const KIT_A: Wear = { suit: true, body: true, head: true };
export function dress(model: THREE.Object3D, wear: Wear): void { sceneDress({ THREE }, model, wear); }
