import { store_get, store_set } from "../../../packages/runtime/src/index.ts";
/** Storage failures are contained by the Rust platform boundary. */
export const store = { get: store_get as (key: string) => string | null, set: store_set };
