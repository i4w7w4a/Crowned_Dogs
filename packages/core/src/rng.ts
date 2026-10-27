// Seeded random numbers. The state is a plain array, so a match can be saved,
// sent over the network and replayed with exactly the same rolls.

export type RngState = [number, number, number, number];

/** Turns any string or number into a 128-bit seed (cyrb128). */
export function seedRng(seed: string | number): RngState {
  const str = String(seed);
  let h1 = 1779033703, h2 = 3144134277, h3 = 1013904242, h4 = 2773480762;
  for (let i = 0; i < str.length; i++) {
    const k = str.charCodeAt(i);
    h1 = h2 ^ Math.imul(h1 ^ k, 597399067);
    h2 = h3 ^ Math.imul(h2 ^ k, 2869860233);
    h3 = h4 ^ Math.imul(h3 ^ k, 951274213);
    h4 = h1 ^ Math.imul(h4 ^ k, 2716044179);
  }
  h1 = Math.imul(h3 ^ (h1 >>> 18), 597399067);
  h2 = Math.imul(h4 ^ (h2 >>> 22), 2869860233);
  h3 = Math.imul(h1 ^ (h3 >>> 17), 951274213);
  h4 = Math.imul(h2 ^ (h4 >>> 19), 2716044179);
  return [(h1 ^ h2 ^ h3 ^ h4) >>> 0, (h2 ^ h1) >>> 0, (h3 ^ h1) >>> 0, (h4 ^ h1) >>> 0];
}

/** Next number in [0, 1). Advances the state in place (sfc32). */
export function nextFloat(s: RngState): number {
  let [a, b, c, d] = s;
  let t = (a + b) | 0;
  a = b ^ (b >>> 9);
  b = (c + (c << 3)) | 0;
  c = (c << 21) | (c >>> 11);
  d = (d + 1) | 0;
  t = (t + d) | 0;
  c = (c + t) | 0;
  s[0] = a >>> 0;
  s[1] = b >>> 0;
  s[2] = c >>> 0;
  s[3] = d >>> 0;
  return (t >>> 0) / 4294967296;
}

/** True with probability p. */
export function roll(s: RngState, p: number): boolean {
  return nextFloat(s) < p;
}

/** Whole number in [0, n). */
export function nextInt(s: RngState, n: number): number {
  return Math.floor(nextFloat(s) * n);
}

/** Picks one item, each with its own weight. */
export function pickWeighted<T>(s: RngState, items: ReadonlyArray<readonly [T, number]>): T {
  let total = 0;
  for (const [, w] of items) total += w;
  let r = nextFloat(s) * total;
  for (const [item, w] of items) {
    r -= w;
    if (r < 0) return item;
  }
  const last = items[items.length - 1];
  if (!last) throw new Error("pickWeighted: empty list");
  return last[0];
}
