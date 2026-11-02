// A tiny vector kit for flat silhouettes. A shape is a filled body plus "cuts":
// lines and holes taken out of the body with a mask, so the icon stays one
// colour (currentColor) and sits on any background.

export type Prim =
  | { k: "rect"; x: number; y: number; w: number; h: number; r?: number }
  | { k: "poly"; pts: number[] }
  | { k: "circle"; cx: number; cy: number; r: number }
  | { k: "ellipse"; cx: number; cy: number; rx: number; ry: number }
  /** A cut line (only in cuts). */
  | { k: "line"; pts: number[]; w?: number };

export interface Shape {
  body: Prim[];
  cuts?: Prim[];
}

export const rect = (x: number, y: number, w: number, h: number, r = 0): Prim => ({ k: "rect", x, y, w, h, r });
export const poly = (...pts: number[]): Prim => ({ k: "poly", pts });
export const circle = (cx: number, cy: number, r: number): Prim => ({ k: "circle", cx, cy, r });
export const ellipse = (cx: number, cy: number, rx: number, ry: number): Prim => ({ k: "ellipse", cx, cy, rx, ry });
export const line = (pts: number[], w?: number): Prim => ({ k: "line", pts, w });

/** Points along an elliptic arc, angles in degrees (0 = right, 90 = down). */
export function arc(cx: number, cy: number, rx: number, ry: number, from: number, to: number, steps = 16): number[] {
  const out: number[] = [];
  for (let i = 0; i <= steps; i++) {
    const a = ((from + ((to - from) * i) / steps) * Math.PI) / 180;
    out.push(round(cx + rx * Math.cos(a)), round(cy + ry * Math.sin(a)));
  }
  return out;
}

const round = (v: number) => Math.round(v * 100) / 100;

/** Moves and scales a shape: (x, y) -> (dx + s*x, dy + s*y). */
export function place(shape: Shape, dx: number, dy: number, s = 1): Shape {
  const t = (p: Prim): Prim => {
    switch (p.k) {
      case "rect":
        return { ...p, x: round(dx + s * p.x), y: round(dy + s * p.y), w: round(s * p.w), h: round(s * p.h), r: round(s * (p.r ?? 0)) };
      case "circle":
        return { ...p, cx: round(dx + s * p.cx), cy: round(dy + s * p.cy), r: round(s * p.r) };
      case "ellipse":
        return { ...p, cx: round(dx + s * p.cx), cy: round(dy + s * p.cy), rx: round(s * p.rx), ry: round(s * p.ry) };
      case "poly":
        return { ...p, pts: p.pts.map((v, i) => round((i % 2 === 0 ? dx : dy) + s * v)) };
      case "line":
        return { ...p, pts: p.pts.map((v, i) => round((i % 2 === 0 ? dx : dy) + s * v)), w: round(s * (p.w ?? CUT)) };
    }
  };
  return { body: shape.body.map(t), cuts: (shape.cuts ?? []).map(t) };
}

export function merge(...shapes: Shape[]): Shape {
  return { body: shapes.flatMap((s) => s.body), cuts: shapes.flatMap((s) => s.cuts ?? []) };
}

/** Default width of a cut line. */
export const CUT = 1.6;

export function bbox(shape: Shape): [number, number, number, number] {
  let x0 = Infinity, y0 = Infinity, x1 = -Infinity, y1 = -Infinity;
  const add = (x: number, y: number) => {
    x0 = Math.min(x0, x); y0 = Math.min(y0, y); x1 = Math.max(x1, x); y1 = Math.max(y1, y);
  };
  for (const p of shape.body) {
    switch (p.k) {
      case "rect": add(p.x, p.y); add(p.x + p.w, p.y + p.h); break;
      case "circle": add(p.cx - p.r, p.cy - p.r); add(p.cx + p.r, p.cy + p.r); break;
      case "ellipse": add(p.cx - p.rx, p.cy - p.ry); add(p.cx + p.rx, p.cy + p.ry); break;
      case "poly": case "line": for (let i = 0; i < p.pts.length; i += 2) add(p.pts[i]!, p.pts[i + 1]!); break;
    }
  }
  return x0 === Infinity ? [0, 0, 1, 1] : [x0, y0, x1 - x0, y1 - y0];
}

function prim(p: Prim, fill: string): string {
  switch (p.k) {
    case "rect":
      return `<rect x="${p.x}" y="${p.y}" width="${p.w}" height="${p.h}"${p.r ? ` rx="${p.r}"` : ""} fill="${fill}"/>`;
    case "circle":
      return `<circle cx="${p.cx}" cy="${p.cy}" r="${p.r}" fill="${fill}"/>`;
    case "ellipse":
      return `<ellipse cx="${p.cx}" cy="${p.cy}" rx="${p.rx}" ry="${p.ry}" fill="${fill}"/>`;
    case "poly":
      return `<polygon points="${p.pts.join(" ")}" fill="${fill}"/>`;
    case "line":
      return `<polyline points="${p.pts.join(" ")}" fill="none" stroke="${fill}" stroke-width="${p.w ?? CUT}" stroke-linecap="round" stroke-linejoin="round"/>`;
  }
}

/** The body (currentColor) and the cuts (black, for a mask) as SVG markup, for pages that compose parts live. */
export function layers(shape: Shape): { body: string; cuts: string } {
  return {
    body: shape.body.map((p) => prim(p, "currentColor")).join(""),
    cuts: (shape.cuts ?? []).map((p) => prim(p, "#000")).join(""),
  };
}

/** Inner SVG markup: a mask with the cuts and the body filled with currentColor. */
export function markup(shape: Shape, id: string): string {
  const [x, y, w, h] = bbox(shape);
  const { body, cuts } = layers(shape);
  if (!cuts) return `<g>${body}</g>`;
  return `<mask id="${id}" maskUnits="userSpaceOnUse" x="${x - 2}" y="${y - 2}" width="${w + 4}" height="${h + 4}">` +
    `<rect x="${x - 2}" y="${y - 2}" width="${w + 4}" height="${h + 4}" fill="#fff"/>${cuts}</mask>` +
    `<g mask="url(#${id})">${body}</g>`;
}

/** A standalone SVG file, cropped to the shape with some padding. */
export function svg(shape: Shape, id: string, pad = 4): string {
  const [x, y, w, h] = bbox(shape);
  const vb = [x - pad, y - pad, w + 2 * pad, h + 2 * pad].map(round).join(" ");
  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="${vb}" fill="currentColor">${markup(shape, `cut-${id}`)}</svg>\n`;
}
