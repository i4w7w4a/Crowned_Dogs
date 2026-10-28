import type { GearSlot } from "@crowned-dogs/core";

/**
 * Equipment pictograms for the fitting room, independent of the combat silhouettes.
 * The 48-unit grid has a 4-unit safe area. Outer contours use 1.7 units; seams and
 * machining use 1.2 units. Every mark inherits currentColor, including the small
 * material wash, so selection can switch grey to white without replacing art.
 */
const edge = (paths: string, details = "", extras = "") =>
  `<g fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round">${paths}${extras}${details ? `<g stroke-width="1.2" stroke-opacity=".65">${details}</g>` : ""}</g>`;
const p = (d: string) => `<path d="${d}"/>`;
const wash = (d: string) => `<path d="${d}" fill="currentColor" fill-opacity=".075"/>`;
const circle = (x: number, y: number, r: number) => `<circle cx="${x}" cy="${y}" r="${r}"/>`;

const cap = () => edge(
  wash("M10 26v-4C10 12 16 7 24 7s14 5 14 15v4Z") +
  p("M10 26v-4C10 12 16 7 24 7s14 5 14 15v4M10 26c7-3 21-3 28 0l6 6c-9 6-24 6-33 0l-3-3Z"),
  p("M24 8v13M14 15l3 1m14-1-3 1M13 29c7-2 15-2 22 0M17 11l-1 2m16-2 1 2") + circle(24, 6.5, 1.5),
);

function helmet(kind: "standard" | "visor" | "night" | "thermal" | "heavy" = "standard"): string {
  const shell = wash("M8 27v-5C8 12 15 6 24 6s16 6 16 16v5l-5 2-3 8H16l-3-8Z") +
    p("M8 27v-5C8 12 15 6 24 6s16 6 16 16v5l-5 2-3 8H16l-3-8Z M8 26c8-3 24-3 32 0 M16 29l2 10 6 3 6-3 2-10");
  const seams = p("M14 17c2-5 6-7 10-7M36 19v3M11 30l1 4m25-4-1 4M20 39h8");
  if (kind === "visor") return edge(shell + p("M13 24h22l-2 10H15Z"), seams + p("M17 27h14M19 30h10"));
  if (kind === "night" || kind === "heavy") return edge(shell +
    p("M22 16h4v7h-4ZM16 24h16v7H16Z M24 24v7"), seams + p("M19 26v3m10-3v3M22 13h4"));
  if (kind === "thermal") return edge(shell + p("M23 16h4v8h-4ZM20 24h12v7H20Z"), seams + circle(26, 27.5, 2));
  return edge(shell, seams + p("M21 15h6v6h-6Z"));
}

function vest(kind: "light" | "plate" | "heavy" | "assault" = "plate"): string {
  const outline = kind === "light"
    ? "M12 7h7l2 8h6l2-8h7l-2 11 5 4v18H9V22l5-4Z"
    : "M12 7h7l2 5h6l2-5h7l-1 13 4 4v15l-5 3H14l-5-3V24l4-4Z";
  const body = wash(outline) + p(outline) + p("M17 15h14l3 6v10H14V21Z M14 33h20M19 7v5m10-5v5");
  const pouches = p("M14 27h6v10h-6ZM21 27h6v10h-6ZM28 27h6v10h-6Z");
  const seams = p("M18 19h12M18 22h12M11 25h2m22 0h2M16 30h2m5 0h2m5 0h2M17 39h14");
  if (kind === "heavy" || kind === "assault") return edge(body + pouches +
    p("M16 40v3h16v-3M9 23l-4 2v10l4 2m30-14 4 2v10l-4 2"), seams + (kind === "assault" ? p("M13 8l-5 4v7l5-1m22-10 5 4v7l-5-1") : ""));
  return edge(body + pouches, seams);
}

function suit(kind: "field" | "thermal" | "assault" | "cloak" = "field"): string {
  if (kind === "cloak") return edge(wash("M19 7h10l3 6 7 4 4 17-8 3-3-9v15H16V28l-3 9-8-3 4-17 7-4Z") +
    p("M19 7h10l3 6 7 4 4 17-8 3-3-9v15H16V28l-3 9-8-3 4-17 7-4ZM19 7l-2 7 7 5 7-5-2-7"),
    p("M24 20v18M10 24l3 2m23-2-3 2M18 32l-2 6m14-6 2 6M12 18l-1 4m25-4 1 4"));
  const outline = "M19 6h10l2 6 8 4 4 16-7 3-4-11v18H16V24l-4 11-7-3 4-16 8-4Z";
  return edge(wash(outline) + p(outline) + p("M19 6l5 7 5-7M24 14v28M16 23v-6l5 2v5h-5m16-1v-6l-5 2v5h5"),
    p("M10 28l4 2m20 0 4-2M17 34h4m6 0h4M18 39h3m6 0h3") +
    (kind === "thermal" ? p("M12 18l2 6M36 18l-2 6M19 27v4m10-4v4") : "") +
    (kind === "assault" ? p("M9 23l5 2m20 0 5-2M18 27h3v4h-3m9-4h3v4h-3") : ""));
}

function boot(kind: "standard" | "light" | "armored" = "standard"): string {
  if (kind === "light") return edge(wash("M8 20l7 4 6-4 5 8 14 5 2 6-3 3H7l-2-4Z") +
    p("M8 20l7 4 6-4 5 8 14 5 2 6-3 3H7l-2-4ZM6 37h35"),
    p("M17 25l6 2m-5 2 7 2M9 30l7 3M29 34h8M9 39v2m7-2v2m7-2v2m7-2v2m7-2v2"));
  const outline = "M12 6h15l-1 16 3 7 10 4 3 5-1 4H8l-1-7 4-8Z";
  return edge(wash(outline) + p(outline) + p("M7 36h18l4-4M8 39h33M11 10h15M13 27l7 4h6"),
    p("M16 13l7 3m-7 1 7 3m-7 1 7 3M19 12v13M29 35h8M12 40v2m6-2v2m6-2v2m6-2v2m6-2v2") +
    (kind === "armored" ? p("M12 12h4v12h-5M11 30l3-2 7 4M28 29l-3 7") : ""));
}

function knife(kind: "standard" | "bayonet" | "heavy" | "curved" | "thermal" = "standard"): string {
  if (kind === "curved") return edge(p("M15 9c7-6 17-3 20 4 3 7-2 15-10 17l-4-6c6-1 9-6 7-10-1-3-5-3-7-1") +
    p("M17 16l7 8-10 15-6-5Z M13 21l7 5"), p("M11 30l5 4m-2-8 5 4") + circle(17, 11, 5));
  const blade = kind === "heavy" ? "M25 8l15-4-4 16-11 10-7-7Z" : "M24 10L40 5l-5 15-11 10-6-6Z";
  return edge(wash(blade) + p(blade) + p("M15 22l12 12M17 27L7 37l5 5 10-10Z M6 36l7 7"),
    p("M12 34l5 5m-2-8 5 5M24 24l10-13") +
    (kind === "bayonet" ? circle(25, 31, 2) + p("M27 13l3 3m0-6 3 3") : "") +
    (kind === "thermal" ? p("M27 23l6-6M11 37l1 1") : ""));
}

function grenade(kind: "frag" | "heavy" | "emp" | "smoke" = "frag"): string {
  const upper = p("M20 13V8h8v5M20 8h13l4 17M27 8V5h8v6");
  if (kind === "smoke" || kind === "emp") {
    const can = "M17 14h15l3 4v21l-3 3H17l-3-3V18Z";
    return edge(wash(can) + p(can) + upper, p("M15 20h19M15 36h19M19 17h11M19 39h11") +
      (kind === "emp" ? p("M26 23l-6 7h5l-2 5 6-7h-5Z") : p("M20 24h10M20 28h10M20 32h10")));
  }
  const body = "M19 14h10c7 4 10 11 9 17-1 7-6 11-14 11s-13-4-14-11c-1-6 2-13 9-17Z";
  return edge(wash(body) + p(body) + upper,
    p("M14 21h20M11 28h26M12 35h24M20 17l-3 21m11-21 3 21M24 17v24") +
    (kind === "heavy" ? p("M10 31h4m20 0h4") : ""));
}

function gun(kind: "pistol" | "rifle" | "shotgun" | "sniper"): string {
  let art: string;
  if (kind === "pistol") art = edge(
    wash("M8 13h29l4 3v7H24l-5 16H9l5-17H8Z") +
    p("M8 13h29l4 3v7H24l-5 16H9l5-17H8ZM24 23h7l-2 9h-8M9 11h5m20 0h4"),
    p("M10 18h28M12 15v4m3-4v4m3-4v4M27 23l-1 5M14 28l5 2m-6 2 5 2M10 37h9"));
  else if (kind === "shotgun") art = edge(
    p("M5 20h9l6-3h12v3h11v4H22l-3 4-2 10h-6l2-10-8 4ZM29 24v4h12v-4M20 24h7v7h-8"),
    p("M22 20h18M31 25v2m3-2v2m3-2v2M7 23h6M15 29l-1 5M24 25v3M39 17h2"));
  else if (kind === "sniper") art = edge(
    p("M5 22l9-1 7-3h12v3h11v3H25l-4 4-3 10h-5l2-11-10 5ZM25 24h6v8h-7M17 12h16v5H17ZM15 11h3v7h-3m17-7h3v7h-3M21 17v2m9-2v2M37 24l-3 13m3-13 4 13"),
    p("M7 25h7M20 14h10M25 27h4M16 29l-1 6M40 21h2"));
  else art = edge(
    p("M5 20h9l3-3h13l3 3h9v4H30l-3 3H17l-4 3-8 3ZM18 27l-3 11h6l3-11M26 27l2 11 6-2-3-12M18 27v5h5M31 17h6v3M18 14h9v3"),
    p("M19 21h9M34 22h7M28 30l3-1m-2 4 3-1M8 23h5M17 34h3M21 28v2M20 15h4"));
  // Long guns use the same diagonal as the knife, giving their receivers room at 32px.
  return kind === "pistol" ? art : `<g transform="rotate(-24 24 24) scale(.9) translate(2.6667 2.6667)">${art}</g>`;
}

const frame = (art: string) => `<svg class="cd-equipment-icon" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 48 48" fill="none" aria-hidden="true" focusable="false">${art}</svg>`;

/** All eight Persona slots; the item ID selects the actual equipment variant. */
export function equipmentIcon(slot: GearSlot, itemId?: string | null): string {
  const id = itemId ?? "";
  switch (slot) {
    case "weapon1": case "weapon2": {
      const kind = id.startsWith("pistol-") ? "pistol" : id.startsWith("shotgun-") ? "shotgun" : id.startsWith("sniper-") ? "sniper" : id.startsWith("rifle-") ? "rifle" : slot === "weapon2" ? "pistol" : "rifle";
      return frame(gun(kind));
    }
    case "head": return frame(id === "head-cap" ? cap() : helmet(id === "head-sfera" ? "visor" : id === "head-marevo" ? "thermal" : id === "head-filin" ? "heavy" : id === "head-sych" ? "night" : "standard"));
    case "body": return frame(vest(id === "body-light" ? "light" : id === "body-plita4" ? "heavy" : id === "body-bastion" ? "assault" : "plate"));
    case "suit": return frame(suit(id === "suit-leshy" ? "cloak" : id === "suit-ryvok" ? "assault" : id === "suit-ten" || id === "suit-solonchak" ? "thermal" : "field"));
    case "boots": return frame(boot(id === "boots-sprint" ? "light" : id === "boots-saper" || id === "boots-hodok" ? "armored" : "standard"));
    case "knife": return frame(knife(id === "knife-kogot" ? "curved" : id === "knife-shtyk" ? "bayonet" : id === "knife-zhalo" ? "heavy" : id === "knife-iskra" ? "thermal" : "standard"));
    case "grenade": return frame(grenade(id === "grenade-emp" ? "emp" : id === "grenade-smoke" ? "smoke" : id === "grenade-grad" ? "heavy" : "frag"));
  }
}

/** Catalogue helper; unknown IDs return no art rather than the wrong item's pictogram. */
export function equipmentItemIcon(itemId: string): string {
  if (/^(pistol|rifle|shotgun|sniper)-/.test(itemId)) return equipmentIcon("weapon1", itemId);
  const prefix = itemId.split("-")[0];
  if (prefix === "head" || prefix === "body" || prefix === "suit" || prefix === "boots" || prefix === "knife" || prefix === "grenade") return equipmentIcon(prefix, itemId);
  return "";
}

/** Menu entry uses the same carrier art as the equipment slots. */
export function fittingRoomIcon(): string { return frame(vest("plate")); }
