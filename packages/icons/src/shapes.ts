// Silhouettes for every item in packages/core/src/items.ts. Weapons are drawn
// side-on, muzzle to the right, without their five swappable parts; each part
// is drawn around its own anchor and placed on the weapon by composeWeapon.
// The same five anchors are the attachment sockets for the 3D models.
//
// Units: rifles, the shotgun and the sniper use 1 unit = 2.5 mm; the pistol
// uses 1 unit = 1 mm. A part drawn in rifle units is scaled up 2.5x on the pistol.

import { arc, circle, ellipse, line, merge, place, poly, rect, type Shape } from "./draw.ts";

export type WeaponArtId = "pistol" | "shotgun" | "rifle" | "sniper";
export type Slot = "optic" | "muzzle" | "rail" | "mag" | "stock";
type Units = "rifle" | "pistol";

export interface WeaponArt {
  units: Units;
  body: Shape;
  /** Where each part sits: optic on top of the rail, muzzle at the bore, rail under the handguard,
   *  mag at the bottom of the magazine well, stock at the back of the receiver. */
  anchors: Record<Slot, [number, number]>;
}

export interface PartArt {
  units: Units;
  /** Drawn around the anchor (0, 0). Empty for factory parts that are part of the base model. */
  mount: Shape;
  /** Inventory icon when the mounted shape says little (ammunition). Defaults to the mount. */
  icon?: Shape;
}

const S = (body: Shape["body"], cuts: Shape["cuts"] = []): Shape => ({ body, cuts });
const EMPTY: Shape = { body: [], cuts: [] };

/** Teeth of a Picatinny rail as cut lines. */
function railTeeth(x0: number, x1: number, y: number, step = 5): Shape["body"] {
  const out = [];
  for (let x = x0; x <= x1; x += step) out.push(line([x, y, x, y + 2.6], 1.2));
  return out;
}

// ---------- weapons ----------

export const WEAPON_ART: Record<WeaponArtId, WeaponArt> = {
  rifle: {
    units: "rifle",
    body: S(
      [
        rect(90, 47, 112, 27, 2),
        rect(118, 43, 64, 5, 1),
        poly(147, 73, 163, 73, 153, 113, 138, 110),
        rect(158, 72, 28, 15, 5),
        rect(200, 49, 88, 24, 3),
        rect(200, 43, 80, 7, 3),
        rect(286, 56, 52, 8),
        rect(310, 54, 16, 11, 1),
        poly(313, 54, 316, 43, 321, 43, 324, 54),
      ],
      [
        ...railTeeth(121, 179, 43),
        rect(162, 73, 20, 10, 4),
        line([110, 53, 176, 53]),
        line([100, 62, 124, 66]),
        line([206, 57, 282, 57]),
        line([206, 66, 282, 66]),
        line([200, 50, 200, 72]),
      ],
    ),
    anchors: { optic: [150, 43], muzzle: [338, 60], rail: [252, 73], mag: [214, 74], stock: [90, 60] },
  },
  shotgun: {
    units: "rifle",
    body: S(
      [
        rect(100, 46, 106, 30, 3),
        rect(126, 42, 50, 5, 1),
        poly(150, 75, 166, 75, 156, 114, 141, 111),
        rect(160, 74, 28, 14, 5),
        rect(206, 53, 170, 9, 2),
        rect(206, 62, 84, 16, 4),
        circle(370, 52, 2.4),
      ],
      [
        ...railTeeth(129, 173, 42),
        rect(164, 75, 20, 9, 4),
        line([128, 55, 190, 55]),
        line([110, 66, 140, 66]),
        line([214, 70, 282, 70]),
        line([206, 63, 206, 77]),
      ],
    ),
    anchors: { optic: [151, 42], muzzle: [376, 57.5], rail: [250, 78], mag: [206, 76], stock: [100, 58] },
  },
  sniper: {
    units: "rifle",
    body: S(
      [
        rect(120, 46, 112, 24, 3),
        rect(126, 42, 108, 5, 1),
        rect(232, 48, 100, 22, 3),
        rect(332, 54, 110, 8, 2),
        rect(196, 50, 10, 6, 2),
        poly(150, 69, 166, 69, 158, 108, 143, 105),
        rect(160, 68, 28, 15, 5),
      ],
      [
        ...railTeeth(129, 231, 42),
        rect(164, 69, 20, 10, 4),
        line([132, 53, 190, 53]),
        line([246, 59, 262, 59], 3),
        line([270, 59, 286, 59], 3),
        line([294, 59, 310, 59], 3),
        line([232, 49, 232, 69]),
      ],
    ),
    anchors: { optic: [180, 42], muzzle: [442, 58], rail: [300, 70], mag: [209, 70], stock: [120, 58] },
  },
  pistol: {
    units: "pistol",
    body: S(
      [
        rect(30, 30, 170, 26, 3),
        rect(40, 55, 156, 13, 2),
        poly(48, 62, 92, 62, 80, 150, 34, 146),
        rect(88, 60, 52, 34, 10),
        rect(186, 25, 6, 6, 1),
      ],
      [
        ...[40, 45, 50, 55, 60].map((x) => line([x, 34, x, 52], 1.4)),
        line([112, 37, 148, 37], 4),
        rect(94, 66, 40, 22, 8),
        line([40, 56, 196, 56], 1.2),
        ...railTeeth(150, 190, 65, 6),
      ],
    ),
    anchors: { optic: [105, 30], muzzle: [200, 44], rail: [165, 68], mag: [57, 148], stock: [41, 104] },
  },
};

// ---------- parts ----------

/** A curved 30-round magazine around the well's bottom centre. */
const curvedMag = (dx = 0, dy = 0) => poly(-11 + dx, dy, 11 + dx, dy, 18 + dx, 27 + dy, 30 + dx, 62 + dy, 10 + dx, 68 + dy, -1 + dx, 34 + dy);

/** A red dot sight: housing, front hood and a mount. Rifle units. */
const redDot = (lift = 0): Shape => S(
  [rect(-12, -4 - lift, 24, 4 + lift, 1), rect(-15, -19 - lift, 30, 15, 4), rect(10, -21 - lift, 7, 19, 2)],
  [line([10, -19 - lift, 10, -6 - lift]), circle(-2, -11.5 - lift, 2.2)],
);

/** Shotgun tube magazine showing past the forend. */
const tube: Shape = S([rect(84, -14, 64, 10, 4), rect(146, -15, 6, 12, 2)], [line([146, -14, 146, -4])]);

/** Shells on a side saddle, as cut lines over the receiver. */
const saddle = (shells: number): Shape => S([], [
  rect(-84, -27, 6 + shells * 9, 2, 1),
  ...Array.from({ length: shells }, (_, i) => line([-80 + i * 9, -24, -80 + i * 9, -12], 5)),
]);

/** Cartridges for ammunition icons. */
function cartridges(n: number, len: number, tip: "round" | "hollow" | "ap" | "long"): Shape {
  const body = [], cuts = [];
  for (let i = 0; i < n; i++) {
    const x = i * 14;
    body.push(rect(x, 0, 10, len, 1.5));
    body.push(tip === "round" || tip === "hollow" ? ellipse(x + 5, 0, 5, 7) : poly(x, 0, x + 3, -14, x + 5, -18, x + 7, -14, x + 10, 0));
    cuts.push(line([x, len - 4, x + 10, len - 4], 1.4));
    if (tip === "hollow") cuts.push(circle(x + 5, -5.5, 1.8));
    if (tip === "ap") cuts.push(line([x + 1.5, -9, x + 8.5, -9], 2.2));
  }
  return S(body, cuts);
}

/** Shotgun shells for ammunition icons. */
function shells(kind: "slug" | "buck"): Shape {
  const body = [], cuts = [];
  for (let i = 0; i < 3; i++) {
    const x = i * 16;
    body.push(rect(x, 0, 12, 34, 2));
    cuts.push(line([x, 24, x + 12, 24], 1.4));
    if (kind === "slug") body.push(ellipse(x + 6, 0, 6, 7));
    else for (const [dx, dy] of [[3.5, 6], [8.5, 6], [6, 11]] as const) cuts.push(circle(x + dx, dy, 1.6));
  }
  return S(body, cuts);
}

export const PART_ART: Record<string, PartArt> = {
  // Rifle factory parts.
  "rifle-optic-stock": { units: "rifle", mount: S([rect(-28, -5, 10, 5, 1)], [line([-23, -5, -23, -2], 1.2)]) },
  "rifle-muzzle-stock": { units: "rifle", mount: S([rect(0, -4.5, 16, 9, 1.5)], [line([5, -4.5, 12, 2])]) },
  "rifle-rail-stock": { units: "rifle", mount: S([rect(-16, 0, 32, 3, 1)]) },
  "rifle-mag-stock": { units: "rifle", mount: S([curvedMag()], [line([2, 6, 9, 32, 20, 60])]) },
  "rifle-stock-stock": {
    units: "rifle",
    mount: S([poly(0, -12, -68, -15, -78, -12, -80, 22, -74, 26, 0, 12)], [poly(-8, -6, -66, -9, -72, -6, -72, 17, -8, 6)]),
  },
  // Shotgun factory parts.
  "shotgun-optic-stock": { units: "rifle", mount: S([rect(-20, -6, 8, 6, 1)], [circle(-16, -3.5, 1.5)]) },
  "shotgun-muzzle-stock": { units: "rifle", mount: S([rect(0, -4.5, 4, 9, 1)]) },
  "shotgun-rail-stock": { units: "rifle", mount: S([rect(-14, 0, 28, 3, 1)]) },
  "shotgun-mag-stock": { units: "rifle", mount: tube },
  "shotgun-stock-stock": {
    units: "rifle",
    mount: S([poly(0, -12, -40, -12, -88, -6, -96, -8, -98, 28, -90, 30, -40, 18, 0, 14)], [line([-92, -6, -93, 28]), line([-40, -12, -40, 18])]),
  },
  // Sniper factory parts.
  "sniper-optic-stock": {
    units: "rifle",
    mount: S(
      [rect(-14, -6, 28, 6, 1), rect(-48, -18, 96, 9, 4.5), poly(30, -18, 54, -24, 54, -3, 30, -9), poly(-48, -19, -64, -21, -64, -6, -48, -8), rect(-6, -26, 12, 9, 2)],
      [line([30, -18, 30, -9]), line([-48, -19, -48, -8]), line([-6, -17, 6, -17])],
    ),
  },
  "sniper-muzzle-stock": { units: "rifle", mount: S([rect(0, -5, 18, 10, 1.5)], [rect(5, -6, 3, 5), rect(11, -6, 3, 5)]) },
  "sniper-rail-stock": { units: "rifle", mount: S([rect(-3, 0, 6, 4), circle(0, 8, 5)], [circle(0, 8, 2.5)]) },
  "sniper-mag-stock": { units: "rifle", mount: S([rect(-13, 0, 26, 18, 2)], [line([-13, 14, 13, 14])]) },
  "sniper-stock-stock": {
    units: "rifle",
    mount: S(
      [poly(0, -12, -20, -14, -112, -6, -118, -8, -120, 30, -112, 32, -60, 22, -30, 26, -20, 14, 0, 12)],
      [poly(-30, -2, -54, -4, -56, 12, -36, 16), line([-114, -6, -116, 30])],
    ),
  },
  // Pistol factory parts (pistol units).
  "pistol-optic-stock": { units: "pistol", mount: S([rect(-76, -5, 9, 5, 1)]) },
  "pistol-muzzle-stock": { units: "pistol", mount: EMPTY },
  "pistol-rail-stock": { units: "pistol", mount: EMPTY },
  "pistol-mag-stock": { units: "pistol", mount: S([rect(-24, -3, 48, 6, 2)]) },
  "pistol-stock-stock": { units: "pistol", mount: S([], [line([10, -36, -2, 40], 1.2)]) },

  // Optics.
  "optic-tritium": { units: "pistol", mount: S([rect(-76, -6, 10, 6, 1)], [circle(-73.5, -3, 1.2), circle(-68.5, -3, 1.2)]), icon: S([rect(0, 0, 30, 14, 2), rect(60, 4, 10, 10, 2)], [circle(9, 6, 2.5), circle(21, 6, 2.5), circle(65, 8, 2.5)]) },
  "optic-kp1": { units: "pistol", mount: S([rect(-15, -16, 30, 16, 3)], [rect(-10, -13, 18, 8, 2)]) },
  "optic-blik": { units: "rifle", mount: redDot() },
  "optic-gran": {
    units: "rifle",
    mount: S(
      [rect(-16, -4, 32, 4, 1), rect(-18, -21, 30, 17, 3), rect(10, -23, 10, 21, 3), rect(-24, -19, 7, 13, 2)],
      [line([-14, -12.5, 8, -12.5]), line([10, -21, 10, -6]), circle(-4, -17, 1.8)],
    ),
  },
  "optic-sokol": {
    units: "rifle",
    mount: S(
      [rect(-22, -10, 7, 10, 1), rect(12, -10, 7, 10, 1), rect(-30, -18, 62, 9, 4), poly(28, -18, 46, -23, 46, -4, 28, -9), poly(-30, -19, -44, -22, -44, -5, -30, -8), rect(-5, -25, 10, 8, 2)],
      [line([28, -18, 28, -9]), line([-30, -19, -30, -8]), line([-5, -19, 5, -19])],
    ),
  },
  "optic-znoy": {
    units: "rifle",
    mount: S(
      [rect(-18, -4, 36, 4, 1), rect(-28, -26, 52, 22, 4), rect(24, -25, 12, 20, 3), rect(-36, -22, 8, 14, 3)],
      [line([-22, -15, 18, -15]), circle(-4, -21, 2), circle(4, -21, 2), line([24, -24, 24, -6])],
    ),
  },
  "optic-oryol": {
    units: "rifle",
    mount: S(
      [rect(-26, -9, 8, 9, 1), rect(18, -9, 8, 9, 1), rect(-56, -21, 112, 11, 5.5), poly(36, -21, 70, -30, 70, -1, 36, -10), poly(-56, -22, -74, -25, -74, -6, -56, -9), rect(-8, -31, 16, 11, 2), rect(12, -27, 8, 7, 2)],
      [line([36, -21, 36, -10]), line([-56, -22, -56, -9]), line([-8, -24, 8, -24]), line([66, -28, 66, -3])],
    ),
  },
  "optic-blik-b": { units: "rifle", mount: merge(redDot(8), S([rect(-14, -8, 28, 8, 1)], [rect(-8, -6, 16, 4, 1)])) },

  // Muzzle devices.
  "muzzle-pb9": { units: "pistol", mount: S([rect(0, -12, 96, 24, 10)], [line([10, -12, 10, 12]), line([86, -12, 86, 12])]) },
  "muzzle-klyk": { units: "rifle", mount: S([rect(0, -5, 20, 10, 1.5)], [line([6, -5, 6, -1.5], 2.2), line([10.5, -5, 10.5, -1.5], 2.2), line([15, -5, 15, -1.5], 2.2)]) },
  "muzzle-choke": { units: "rifle", mount: S([rect(0, -5, 12, 10, 1.5)], [line([3, -5, 3, 5], 1), line([6, -5, 6, 5], 1), line([9, -5, 9, 5], 1)]) },
  "muzzle-vata": { units: "rifle", mount: S([rect(0, -9, 72, 18, 8)], [line([10, -9, 10, 9]), line([62, -9, 62, 9])]) },
  "muzzle-tishina": { units: "rifle", mount: S([rect(0, -7, 66, 14, 6)], [line([8, -7, 8, 7]), line([58, -7, 58, 7])]) },
  "muzzle-tishina-s": { units: "rifle", mount: S([rect(0, -8, 82, 16, 7)], [line([8, -8, 8, 8]), line([74, -8, 74, 8])]) },
  "muzzle-molot": { units: "rifle", mount: S([rect(0, -7, 26, 14, 2)], [rect(5, -8, 5, 7, 1), rect(13, -8, 5, 7, 1), line([22, -7, 22, 7])]) },

  // Handguard modules.
  "rail-luch": { units: "rifle", mount: S([rect(-8, 0, 16, 4, 1), rect(-20, 4, 40, 10, 3), rect(18, 2, 12, 14, 2)], [line([18, 3, 18, 15]), line([-14, 9, 8, 9], 1.2)]) },
  "rail-lcu": { units: "pistol", mount: S([rect(-10, 0, 20, 3, 1), rect(-22, 3, 40, 16, 3)], [circle(12, 11, 3), line([-16, 11, 2, 11], 1.2)]) },
  "rail-kulak": { units: "rifle", mount: S([rect(-8, 0, 16, 4, 1), poly(-6, 4, 6, 4, 5, 36, -6, 36)], [line([-6, 12, -2, 12], 1.4), line([-6, 19, -2, 19], 1.4), line([-6, 26, -2, 26], 1.4)]) },
  "rail-koster": {
    units: "rifle",
    mount: S(
      [rect(-30, 0, 60, 4, 1), rect(-24, 4, 68, 13, 3), rect(-34, 2, 14, 18, 2), rect(-40, 17, 16, 12, 4), poly(-38, 18, -30, 18, -33, 40, -41, 38)],
      [line([44, 5, 44, 16]), rect(-37, 19, 9, 7, 2), line([-20, 6, -20, 17])],
    ),
  },
  "rail-upor": {
    units: "rifle",
    mount: S([rect(-8, 0, 16, 7, 2), poly(-2, 6, 3, 6, 22, 58, 17, 60), poly(-3, 6, 2, 6, -14, 58, -19, 57), rect(15, 57, 9, 4, 1), rect(-22, 55, 8, 4, 1)], [circle(0, 3.5, 1.6)]),
  },

  // Magazines and ammunition.
  "mag-pistol-long": { units: "pistol", mount: S([rect(-22, -3, 44, 34, 3)], [line([-22, 25, 22, 25])]) },
  "mag-pistol-hp": { units: "pistol", mount: S([rect(-24, -3, 48, 6, 2)], [circle(0, 0, 1.5)]), icon: cartridges(3, 18, "hollow") },
  "mag-pistol-ap": { units: "pistol", mount: S([rect(-24, -3, 48, 6, 2)], [line([-6, 0, 6, 0], 1.4)]), icon: cartridges(3, 18, "ap") },
  "mag-shotgun-box": { units: "rifle", mount: S([poly(-38, 0, -8, 0, -6, 46, -36, 48)], [line([-36, 12, -7, 12], 1.2), line([-36, 24, -7, 24], 1.2), line([-36, 38, -7, 38], 1.2)]) },
  "mag-shotgun-slug": { units: "rifle", mount: merge(tube, saddle(4)), icon: shells("slug") },
  "mag-shotgun-buck": { units: "rifle", mount: merge(tube, saddle(5)), icon: shells("buck") },
  "mag-rifle-twin": {
    units: "rifle",
    mount: S([curvedMag(), curvedMag(9, 2)], [line([-2, 2, 8, 36, 19, 70]), line([3, 40, 22, 36], 3)]),
  },
  "mag-rifle-ap": { units: "rifle", mount: S([curvedMag()], [line([2, 6, 9, 32, 20, 60]), line([10, 54, 26, 50], 2)]), icon: cartridges(3, 30, "ap") },
  "mag-rifle-drum": { units: "rifle", mount: S([rect(-10, 0, 20, 12, 2), circle(8, 42, 30)], [line(arc(8, 42, 22, 22, 0, 360, 36)), circle(8, 42, 5)]) },
  "mag-sniper-10": { units: "rifle", mount: S([rect(-13, 0, 26, 32, 2)], [line([-13, 28, 13, 28]), line([-8, 6, -8, 24], 1.2)]) },
  "mag-sniper-ap": { units: "rifle", mount: S([rect(-13, 0, 26, 18, 2)], [line([-13, 14, 13, 14]), circle(0, 7, 2)]), icon: cartridges(2, 40, "ap") },

  // Stocks and grips.
  "stock-pistol-hvat": {
    units: "pistol",
    mount: S([], Array.from({ length: 24 }, (_, i) => circle(16 - Math.floor(i / 4) * 2 + (i % 4) * 7, -26 + Math.floor(i / 4) * 12, 1.3))),
    icon: S([poly(14, 0, 44, 0, 36, 70, 4, 66)], Array.from({ length: 20 }, (_, i) => circle(14 + (i % 4) * 7 - Math.floor(i / 4) * 1.5, 10 + Math.floor(i / 4) * 12, 1.6))),
  },
  "stock-pistol-upor": {
    units: "pistol",
    mount: S([poly(4, -34, -6, -36, -118, -26, -126, -30, -128, 18, -118, 22, -14, 30, -4, 36)], [poly(-20, -24, -104, -18, -106, 8, -24, 18), line([-120, -25, -122, 18])]),
  },
  "stock-shotgun-fold": {
    units: "rifle",
    mount: S([poly(0, -12, -76, -16, -84, -12, -86, 24, -80, 28, 0, 14)], [poly(-8, -6, -74, -10, -78, -6, -78, 19, -8, 8), circle(-4, 0, 1.8)]),
  },
  "stock-shotgun-otdacha": {
    units: "rifle",
    mount: S(
      [poly(0, -12, -40, -13, -84, -8, -84, 28, -40, 20, 0, 14), rect(-98, -11, 15, 42, 4)],
      [line([-86, -7, -90, -2, -86, 4, -90, 10, -86, 16, -90, 22, -86, 27], 1.2), line([-84, -8, -84, 28])],
    ),
  },
  "stock-rifle-opora": {
    units: "rifle",
    mount: S(
      [rect(-44, -6, 44, 12, 3), poly(-40, -13, -82, -15, -84, 24, -64, 27, -40, 11), rect(-76, -21, 30, 8, 3)],
      [line([-79, -14, -80, 24]), circle(-61, -17, 1.6), line([-40, -13, -40, 11])],
    ),
  },
  "stock-rifle-tesno": { units: "rifle", mount: S([rect(-30, -6, 30, 12, 3), poly(-30, -13, -42, -13, -44, 18, -30, 15)], [line([-40, -12, -41, 17])]) },
  "stock-sniper-izgotovka": {
    units: "rifle",
    mount: S(
      [rect(-40, -6, 40, 12, 3), poly(-36, -10, -110, -12, -112, 30, -96, 32, -36, 10), rect(-96, -22, 44, 9, 3), rect(-84, -14, 4, 6), rect(-64, -14, 4, 6), rect(-118, -14, 8, 48, 3)],
      [rect(-90, -4, 40, 14, 6), line([-110, -12, -110, 32])],
    ),
  },
  "stock-sniper-opora": {
    units: "rifle",
    mount: S(
      [rect(-40, -6, 40, 12, 3), poly(-36, -10, -110, -12, -112, 30, -96, 32, -36, 10), rect(-118, -14, 8, 48, 3), rect(-88, 28, 5, 20), rect(-92, 46, 13, 4, 1)],
      [rect(-90, -4, 40, 14, 6), line([-110, -12, -110, 32]), line([-88, 38, -83, 38], 1.2)],
    ),
  },
};

/** Draws a weapon with the given part in each slot. */
export function composeWeapon(weapon: WeaponArtId, parts: Record<Slot, string>): Shape {
  const art = WEAPON_ART[weapon];
  const placed = (Object.keys(parts) as Slot[]).map((slot) => {
    const part = PART_ART[parts[slot]];
    if (!part) throw new Error(`No art for ${parts[slot]}`);
    const [x, y] = art.anchors[slot];
    const s = part.units === "rifle" && art.units === "pistol" ? 2.5 : 1;
    return place(part.mount, x, y, s);
  });
  // Parts go under the body so the receiver hides their mount ends.
  return merge(...placed, art.body);
}

// ---------- what the Persona wears (100 x 100 boxes) ----------

/** A helmet dome from ear to ear, seen from the side. */
const dome = (top: number, bottom = 66, left = 14, right = 86) => poly(...arc((left + right) / 2, bottom, (right - left) / 2, bottom - top, 180, 360, 24));

/** Uneven strand lengths, so the ghillie outline looks torn rather than toothed. */
const JAG = [1.4, -0.3, 0.6, 2.1, -0.5, 1.0, 0.2, 1.7, -0.2, 0.8];

/** A deterministic ragged outline for the ghillie suit. */
function ragged(pts: number[], depth: number): number[] {
  const out: number[] = [];
  for (let i = 0; i < pts.length; i += 2) {
    const [x0, y0, x1, y1] = [pts[i]!, pts[i + 1]!, pts[(i + 2) % pts.length]!, pts[(i + 3) % pts.length]!];
    const n = Math.max(2, Math.round(Math.hypot(x1 - x0, y1 - y0) / 4));
    for (let k = 0; k < n; k++) {
      const t = k / n, j = JAG[(i / 2 + k) % JAG.length]! * depth;
      const nx = -(y1 - y0), ny = x1 - x0, len = Math.hypot(nx, ny) || 1;
      out.push(Math.round((x0 + (x1 - x0) * t + (nx / len) * j) * 10) / 10, Math.round((y0 + (y1 - y0) * t + (ny / len) * j) * 10) / 10);
    }
  }
  return out;
}

const JACKET = [30, 16, 42, 12, 58, 12, 70, 16, 88, 30, 92, 62, 80, 64, 76, 40, 76, 88, 24, 88, 24, 40, 20, 64, 8, 62, 12, 30];
const BOOT_HIGH = [30, 8, 56, 8, 58, 54, 84, 62, 92, 70, 92, 80, 26, 80, 26, 54];

export const WEAR_ART: Record<string, Shape> = {
  "head-cap": S([dome(30, 62, 18, 78), poly(70, 56, 97, 62, 96, 68, 70, 64), rect(18, 58, 54, 7, 2)], [rect(32, 40, 18, 12, 2), line([48, 31, 48, 58], 1.2)]),
  "head-k6": S([dome(22), poly(8, 64, 92, 64, 88, 72, 12, 72)], [line([12, 66, 88, 66]), line([30, 50, 70, 50], 1.2)]),
  "head-sfera": S([dome(20), poly(60, 30, 90, 46, 92, 80, 64, 76), rect(10, 62, 52, 16, 4)], [line([60, 30, 64, 76]), line([66, 52, 90, 58]), circle(36, 70, 3)]),
  "head-sych": S([dome(26, 62, 16, 82), rect(70, 34, 10, 14, 2), poly(74, 44, 96, 50, 94, 66, 72, 60)], [line([24, 58, 74, 58]), rect(32, 36, 16, 10, 2), line([76, 48, 92, 52], 1.2), circle(92, 58, 3)]),
  "head-marevo": S([dome(26, 62, 16, 82), rect(70, 30, 24, 18, 3)], [line([24, 58, 74, 58]), circle(88, 39, 5), line([74, 34, 80, 34], 1.2)]),
  "head-filin": S([dome(22), rect(66, 26, 12, 12, 2), poly(70, 34, 98, 38, 98, 58, 68, 54), poly(8, 64, 90, 64, 86, 72, 12, 72)], [line([12, 66, 86, 66]), line([72, 46, 96, 48]), circle(94, 42, 2.6), circle(94, 54, 2.6)]),

  "body-light": S(
    [poly(28, 8, 36, 8, 50, 40, 64, 8, 72, 8, 58, 46, 42, 46), rect(16, 44, 68, 34, 5)],
    [line([33, 46, 33, 78]), line([50, 46, 50, 78]), line([67, 46, 67, 78]), line([16, 56, 84, 56])],
  ),
  "body-plita2": S(
    [poly(26, 10, 38, 10, 40, 24, 60, 24, 62, 10, 74, 10, 78, 30, 78, 80, 22, 80, 22, 30)],
    [poly(30, 32, 70, 32, 70, 72, 30, 72), line([22, 76, 78, 76])],
  ),
  "body-plita4": S(
    [poly(26, 6, 38, 6, 40, 20, 60, 20, 62, 6, 74, 6, 78, 26, 78, 74, 22, 74, 22, 26), poly(38, 74, 62, 74, 58, 96, 42, 96), circle(84, 56, 9)],
    [line([30, 30, 70, 30]), line([30, 40, 70, 40]), line([30, 50, 70, 50]), line([30, 60, 70, 60]), line([22, 70, 78, 70]), circle(84, 56, 3)],
  ),
  "body-bastion": S(
    [poly(28, 12, 72, 12, 80, 30, 80, 76, 20, 76, 20, 30), poly(30, 4, 70, 4, 76, 14, 24, 14), ellipse(12, 30, 11, 15), ellipse(88, 30, 11, 15), poly(36, 76, 64, 76, 60, 98, 40, 98)],
    [line([24, 14, 76, 14]), poly(30, 30, 70, 30, 70, 66, 30, 66), line([20, 72, 80, 72]), line([3, 30, 21, 30]), line([79, 30, 97, 30])],
  ),

  "suit-field": S([poly(...JACKET)], [line([50, 16, 50, 88]), poly(42, 12, 50, 26, 58, 12), rect(30, 50, 12, 10, 2), rect(58, 50, 12, 10, 2)]),
  "suit-solonchak": S([poly(...JACKET)], [line([50, 16, 50, 88]), poly(42, 12, 50, 26, 58, 12), line([30, 40, 30, 80], 1.2), line([34, 40, 34, 80], 1.2), line([66, 40, 66, 80], 1.2), line([70, 40, 70, 80], 1.2)]),
  "suit-leshy": S([poly(...ragged([50, 4, 64, 9, 72, 22, 86, 40, 90, 92, 10, 92, 14, 40, 28, 22, 36, 9], 3))], [ellipse(50, 22, 9, 9), line([36, 46, 40, 60], 1.2), line([60, 44, 64, 58], 1.2), line([30, 66, 34, 84], 1.2), line([48, 56, 50, 76], 1.2), line([66, 64, 70, 84], 1.2)]),
  "suit-ryvok": S(
    [poly(32, 8, 68, 8, 84, 22, 86, 52, 76, 52, 72, 34, 72, 56, 66, 96, 54, 96, 50, 62, 46, 96, 34, 96, 28, 56, 28, 34, 24, 52, 14, 52, 16, 22)],
    [line([50, 8, 50, 60]), rect(34, 74, 10, 8, 2), rect(56, 74, 10, 8, 2), line([28, 44, 72, 44])],
  ),
  "suit-ten": S(
    [poly(50, 2, 66, 8, 72, 22, 86, 40, 90, 96, 10, 96, 14, 40, 28, 22, 34, 8)],
    [ellipse(50, 20, 10, 10), line([34, 44, 28, 94], 1.2), line([66, 44, 72, 94], 1.2), line([50, 36, 50, 94], 1.2)],
  ),

  "boots-bertsy": S([poly(...BOOT_HIGH), rect(24, 80, 70, 8, 2)], [line([34, 16, 52, 16], 1.2), line([34, 24, 52, 24], 1.2), line([34, 32, 52, 32], 1.2), line([36, 40, 54, 40], 1.2), line([26, 80, 92, 80])]),
  "boots-sprint": S([poly(26, 50, 50, 44, 62, 56, 86, 62, 94, 72, 94, 80, 24, 80), rect(22, 80, 74, 7, 3)], [line([38, 66, 62, 58, 80, 70]), line([24, 80, 94, 80])]),
  "boots-hodok": S(
    [poly(30, 24, 56, 24, 58, 54, 84, 62, 92, 70, 92, 80, 26, 80, 26, 54), poly(24, 80, 94, 80, 94, 86, 90, 90, 86, 86, 80, 90, 76, 86, 70, 90, 66, 86, 60, 90, 56, 86, 50, 90, 46, 86, 40, 90, 36, 86, 30, 90, 24, 86)],
    [line([34, 32, 52, 32], 1.2), line([34, 40, 52, 40], 1.2), line([36, 48, 54, 48], 1.2), line([26, 80, 92, 80])],
  ),
  "boots-saper": S([poly(...BOOT_HIGH), rect(22, 80, 74, 11, 3), rect(56, 12, 12, 46, 4)], [poly(58, 16, 66, 16, 66, 54, 58, 54), line([24, 82, 94, 82]), line([34, 24, 50, 24], 1.2), line([34, 34, 50, 34], 1.2)]),

  "knife-std": S([rect(4, 44, 34, 12, 4), rect(36, 38, 5, 24, 2), poly(41, 44, 84, 44, 96, 48, 84, 56, 41, 56)], [line([44, 50, 80, 50], 1.2), line([12, 44, 12, 56], 1.2), line([20, 44, 20, 56], 1.2), line([28, 44, 28, 56], 1.2)]),
  "knife-shtyk": S([rect(4, 44, 32, 12, 4), circle(40, 40, 8), rect(36, 46, 6, 12, 2), poly(42, 45, 82, 45, 96, 50, 82, 56, 42, 56)], [circle(40, 40, 4.5), ...[48, 54, 60, 66, 72].map((x) => line([x, 45, x + 3, 47.5], 1.2)), line([10, 47, 30, 47], 1.2)]),
  "knife-zhalo": S([rect(2, 44, 32, 14, 5), rect(32, 36, 6, 30, 2), poly(38, 42, 84, 42, 96, 52, 90, 62, 38, 62)], [line([42, 52, 88, 52], 1.2), line([84, 42, 90, 62], 1.2), line([10, 51, 26, 51], 2)]),
  "knife-kogot": S(
    [poly(...arc(56, 70, 34, 30, 200, 300, 14), ...arc(56, 70, 24, 22, 300, 200, 14)), rect(14, 58, 26, 12, 4), circle(10, 66, 8)],
    [circle(10, 66, 4.5), line([20, 64, 34, 64], 1.2)],
  ),
  "knife-iskra": S([rect(4, 42, 34, 16, 4), rect(36, 38, 5, 24, 2), poly(41, 44, 84, 44, 96, 48, 84, 56, 41, 56)], [line([44, 54, 88, 51], 1.2), rect(10, 46, 14, 8, 2), line([28, 46, 28, 54], 1.2)]),

  "grenade-rg1": S([ellipse(50, 60, 22, 26), rect(44, 26, 12, 10, 2), poly(56, 28, 66, 30, 64, 70, 60, 70, 60, 34), circle(36, 26, 7)], [line([28, 60, 72, 60]), circle(36, 26, 4)]),
  "grenade-grad": S([ellipse(50, 60, 24, 30), rect(43, 22, 14, 10, 2), poly(57, 24, 68, 26, 66, 70, 62, 70, 62, 30), circle(34, 22, 7)], [line([26, 50, 74, 50]), line([26, 64, 74, 64]), line([26, 78, 74, 78]), line([40, 32, 40, 88]), line([60, 32, 60, 88]), circle(34, 22, 4)]),
  "grenade-emp": S([rect(32, 30, 36, 62, 8), rect(42, 20, 16, 12, 2), poly(58, 22, 68, 24, 66, 60, 62, 60, 62, 28)], [line([32, 42, 68, 42]), line([32, 82, 68, 82]), poly(52, 48, 42, 64, 50, 64, 46, 76, 58, 58, 50, 58, 54, 48)]),
  "grenade-smoke": S([rect(32, 30, 36, 62, 6), rect(42, 20, 16, 12, 2), poly(58, 22, 68, 24, 66, 60, 62, 60, 62, 28), circle(30, 16, 6)], [circle(40, 38, 2.4), circle(50, 38, 2.4), circle(60, 38, 2.4), line([32, 50, 68, 50]), line([32, 80, 68, 80]), circle(30, 16, 3)]),
};

/** The inventory icon of any item id. Weapons are drawn with their factory parts. */
export function itemArt(id: string, stockParts?: Record<WeaponArtId, Record<Slot, string>>): Shape | null {
  if (WEAR_ART[id]) return WEAR_ART[id];
  const part = PART_ART[id];
  if (part) return part.icon ?? (part.mount.body.length ? part.mount : null);
  const weapon = (Object.keys(WEAPON_ART) as WeaponArtId[]).find((w) => id.startsWith(`${w}-`));
  if (weapon && stockParts) return composeWeapon(weapon, stockParts[weapon]);
  return null;
}
