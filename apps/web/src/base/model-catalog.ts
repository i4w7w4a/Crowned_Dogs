import type { Item } from "@crowned-dogs/core";
import { callRust } from "../../../../packages/runtime/src/index.ts";
export const ITEM_MODELS = callRust<Readonly<Record<string, string>>>("client.models.catalog", []);
export const availableInFitting = (item: Item): boolean => callRust("client.models.available", [item]);
