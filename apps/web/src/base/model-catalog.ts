import type { Item } from "@crowned-dogs/core";

export const ITEM_MODELS: Readonly<Record<string, string>> = {
  "head-k6": "tripo-helmet", "body-plita2": "tripo-vest", "suit-field": "tripo-suit-male", "rifle-varyag": "varyag",
};

/** The fitting catalogue offers actual models, plus the three explicitly allowed gear types. */
export function availableInFitting(item: Item): boolean {
  if (["knife", "grenade", "boots"].includes(item.kind)) return true;
  if (item.kind === "mod") return (!!item.isStock && item.fits.includes("rifle")) || ["optic-sokol", "muzzle-tishina"].includes(item.id);
  return !!ITEM_MODELS[item.id];
}
