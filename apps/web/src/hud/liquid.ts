import { liquid_glass } from "../../../../packages/runtime/src/index.ts";
/** Rust owns the resize observer, filter cache and displacement pixels. */
export function liquidGlass(root: ParentNode = document): void { liquid_glass(root); }
