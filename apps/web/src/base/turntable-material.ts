import * as THREE from "three";

/** Painted workshop steel: broad oxidation, rubbed edges and fine machining marks.
 * Kept in material space, so neither the wear nor the highlights slide when it turns. */
export function turntableMaterial(): THREE.MeshStandardMaterial {
  const size = 1024;
  const color = document.createElement("canvas"), rough = document.createElement("canvas"), relief = document.createElement("canvas");
  for (const canvas of [color, rough, relief]) canvas.width = canvas.height = size;
  const ctx = color.getContext("2d")!, rctx = rough.getContext("2d")!, hctx = relief.getContext("2d")!;
  const c = ctx.createImageData(size, size), r = rctx.createImageData(size, size), h = hctx.createImageData(size, size);
  const hash = (x: number, y: number) => {
    let n = Math.imul(x, 374761393) + Math.imul(y, 668265263);
    n = Math.imul(n ^ (n >>> 13), 1274126177);
    return ((n ^ (n >>> 16)) >>> 0) / 4294967295;
  };
  const noise = (x: number, y: number) => {
    const ix = Math.floor(x), iy = Math.floor(y);
    const fx = x - ix, fy = y - iy, u = fx * fx * (3 - 2 * fx), v = fy * fy * (3 - 2 * fy);
    return THREE.MathUtils.lerp(THREE.MathUtils.lerp(hash(ix, iy), hash(ix + 1, iy), u), THREE.MathUtils.lerp(hash(ix, iy + 1), hash(ix + 1, iy + 1), u), v);
  };
  for (let y = 0; y < size; y++) for (let x = 0; x < size; x++) {
    const i = (y * size + x) * 4;
    const broad = noise(x / 150, y / 150), mottled = noise(x / 28, y / 28), pits = noise(x / 3, y / 3), grain = hash(x, y);
    const radius = Math.hypot(x - size / 2, y - size / 2) / (size / 2);
    const edge = radius > .965 - .022 * mottled && grain > .28 ? 24 : 0;
    const stain = THREE.MathUtils.smoothstep(broad, .35, .75);
    const value = 68 + broad * 25 + mottled * 12 + (grain - .5) * 6 + edge - stain * 18;
    c.data[i] = value + stain * 3; c.data[i + 1] = value + 1; c.data[i + 2] = value + 2 - stain * 3; c.data[i + 3] = 255;
    const roughness = 186 + broad * 45 + mottled * 14 - edge * 1.4;
    r.data[i] = r.data[i + 1] = r.data[i + 2] = roughness; r.data[i + 3] = 255;
    h.data[i] = h.data[i + 1] = h.data[i + 2] = 110 + pits * 24 + mottled * 12; h.data[i + 3] = 255;
  }
  ctx.putImageData(c, 0, 0); rctx.putImageData(r, 0, 0); hctx.putImageData(h, 0, 0);
  // Sparse, directional wear, with larger boot scuffs instead of uniform screen noise.
  for (let i = 0; i < 420; i++) {
    const x = hash(i, 2) * size, y = hash(i, 7) * size;
    const length = 3 + hash(i, 13) ** 3 * 95, angle = -.65 + hash(i, 19) * .34;
    for (const [context, shade] of [[ctx, "rgba(182,185,181,.19)"], [rctx, "rgba(110,110,110,.28)"], [hctx, "rgba(55,55,55,.12)"]] as const) {
      context.strokeStyle = shade; context.lineWidth = .5 + hash(i, 31);
      context.beginPath(); context.moveTo(x, y); context.lineTo(x + Math.cos(angle) * length, y + Math.sin(angle) * length); context.stroke();
    }
  }
  for (let i = 0; i < 24; i++) {
    const radius = size * (.30 + hash(i, 46) * .185), start = hash(i, 41) * Math.PI * 2;
    ctx.strokeStyle = "rgba(185,184,176,.13)"; ctx.lineWidth = .7 + hash(i, 49) * 2;
    ctx.beginPath(); ctx.arc(size / 2, size / 2, radius, start, start + .04 + hash(i, 53) * .3); ctx.stroke();
  }
  const map = new THREE.CanvasTexture(color), roughnessMap = new THREE.CanvasTexture(rough), bumpMap = new THREE.CanvasTexture(relief);
  map.colorSpace = THREE.SRGBColorSpace;
  for (const texture of [map, roughnessMap, bumpMap]) texture.anisotropy = 4;
  return new THREE.MeshStandardMaterial({ color: 0xb5b8ba, map, roughness: .98, roughnessMap, metalness: .42, bumpMap, bumpScale: .004 });
}
