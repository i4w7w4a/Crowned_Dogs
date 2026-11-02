import { callRust } from "../../runtime/src/index.ts";

export type Prim =
  | { k: "rect"; x: number; y: number; w: number; h: number; r?: number }
  | { k: "poly"; pts: number[] }
  | { k: "circle"; cx: number; cy: number; r: number }
  | { k: "ellipse"; cx: number; cy: number; rx: number; ry: number }
  | { k: "line"; pts: number[]; w?: number };
export interface Shape { body: Prim[]; cuts?: Prim[] }

// These factories describe data; all geometric and drawing work is in Rust.
export const rect = (x: number, y: number, w: number, h: number, r = 0): Prim => ({ k: "rect", x, y, w, h, r });
export const poly = (...pts: number[]): Prim => ({ k: "poly", pts });
export const circle = (cx: number, cy: number, r: number): Prim => ({ k: "circle", cx, cy, r });
export const ellipse = (cx: number, cy: number, rx: number, ry: number): Prim => ({ k: "ellipse", cx, cy, rx, ry });
export const line = (pts: number[], w?: number): Prim => ({ k: "line", pts, w });
export const CUT = 1.6;

export const arc = (cx: number, cy: number, rx: number, ry: number, from: number, to: number, steps = 16): number[] =>
  callRust("icons.arc", [cx, cy, rx, ry, from, to, steps]);
export const place = (shape: Shape, dx: number, dy: number, scale = 1): Shape => callRust("icons.place", [shape, dx, dy, scale]);
export const merge = (...shapes: Shape[]): Shape => callRust("icons.merge", shapes);
export const bbox = (shape: Shape): [number, number, number, number] => callRust("icons.bbox", [shape]);
export const layers = (shape: Shape): { body: string; cuts: string } => callRust("icons.layers", [shape]);
export const markup = (shape: Shape, id: string): string => callRust("icons.markup", [shape, id]);
export const svg = (shape: Shape, id: string, pad = 4): string => callRust("icons.svg", [shape, id, pad]);
