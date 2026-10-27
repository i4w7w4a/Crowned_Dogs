// Liquid glass, Apple style: the picture behind a button bends at its rounded edge, like the rim of a thick lens.
// Only Chromium can push the backdrop through an SVG filter; Safari and Firefox keep the plain glass of style.css
// (a light blur and the lit rim). Each button size gets its own displacement map, drawn once on a canvas.

const NS = "http://www.w3.org/2000/svg";
const RIM = 12;     // px: how deep the bending edge reaches into a button
const PULL = 0.75;  // how far the very edge looks inward, as a share of the rim

const chromium = !!(navigator as Navigator & { userAgentData?: { brands: { brand: string }[] } })
  .userAgentData?.brands.some((b) => b.brand === "Chromium");
let defs: SVGDefsElement | undefined;
const filters = new Map<string, string>();
const watched = new WeakSet<Element>();
const sizes = chromium ? new ResizeObserver((list) => { for (const e of list) lens(e.target as HTMLElement); }) : undefined;

/** Bends the backdrop of every .glass element under root; call again after adding new buttons. */
export function liquidGlass(root: ParentNode = document): void {
  if (!sizes) return;
  for (const el of root.querySelectorAll<HTMLElement>(".glass")) {
    if (!watched.has(el)) { watched.add(el); sizes.observe(el); }
  }
}

function lens(el: HTMLElement): void {
  if (!el.isConnected) { sizes!.unobserve(el); return; }
  const w = el.offsetWidth, h = el.offsetHeight;
  if (!w || !h) return;
  const r = Math.min(parseFloat(getComputedStyle(el).borderTopLeftRadius) || 0, w / 2, h / 2);
  const key = `${w}x${h}r${Math.round(r)}`;
  let id = filters.get(key);
  if (!id) { id = "lg" + filters.size; filters.set(key, id); makeFilter(id, w, h, r); }
  el.style.setProperty("--lens", `url(#${id})`);
}

function makeFilter(id: string, w: number, h: number, r: number): void {
  if (!defs) {
    const svg = document.createElementNS(NS, "svg");
    attrs(svg, { width: 0, height: 0, "aria-hidden": "true" }); svg.style.position = "absolute";
    defs = document.createElementNS(NS, "defs"); svg.appendChild(defs); document.body.appendChild(svg);
  }
  const rim = Math.min(RIM, h / 3), f = document.createElementNS(NS, "filter");
  attrs(f, { id, x: 0, y: 0, width: w, height: h, filterUnits: "userSpaceOnUse", primitiveUnits: "userSpaceOnUse", "color-interpolation-filters": "sRGB" });
  const map = document.createElementNS(NS, "feImage");
  attrs(map, { href: drawMap(w, h, r, rim), x: 0, y: 0, width: w, height: h, preserveAspectRatio: "none", result: "map" });
  const bend = document.createElementNS(NS, "feDisplacementMap");
  attrs(bend, { in: "SourceGraphic", in2: "map", scale: 2 * PULL * rim, xChannelSelector: "R", yChannelSelector: "G" });
  f.append(map, bend); defs.appendChild(f);
}

// Red and green carry where each pixel looks (128 = straight through). Inside the rim it looks inward,
// hardest at the edge, so the picture there is stretched outward the way a lens rim does it.
function drawMap(w: number, h: number, r: number, rim: number): string {
  const c = document.createElement("canvas"); c.width = w; c.height = h;
  const g = c.getContext("2d")!, img = g.createImageData(w, h), px = img.data;
  const hw = w / 2, hh = h / 2;
  for (let y = 0; y < h; y++) for (let x = 0; x < w; x++) {
    const dx = x + 0.5 - hw, dy = y + 0.5 - hh, qx = Math.abs(dx) - (hw - r), qy = Math.abs(dy) - (hh - r);
    let nx = 0, ny = 0, depth: number;
    if (qx > 0 && qy > 0) { const l = Math.hypot(qx, qy); depth = r - l; nx = qx / l; ny = qy / l; }
    else if (qx > qy) { depth = r - qx; nx = 1; }
    else { depth = r - qy; ny = 1; }
    const t = Math.min(1, Math.max(0, 1 - depth / rim)), k = t * t, i = (y * w + x) * 4;
    px[i] = 128 - Math.sign(dx) * nx * k * 127;
    px[i + 1] = 128 - Math.sign(dy) * ny * k * 127;
    px[i + 2] = 128; px[i + 3] = 255;
  }
  g.putImageData(img, 0, 0);
  return c.toDataURL();
}

function attrs(el: Element, a: Record<string, string | number>): void {
  for (const [k, v] of Object.entries(a)) el.setAttribute(k, String(v));
}
