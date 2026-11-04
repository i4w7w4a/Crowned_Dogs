// The arena on WebGPU (?gpu=1): the same Rust render graph, with three's WebGPU renderer.
// Rust makes its materials as ShaderMaterials with GLSL; WebGPU draws only node materials, so
// `ShaderMaterial` here builds a TSL twin of each named one. On «Минимум» and «Низкое» the picture
// matches WebGL: the weather at each quality's richness, depth of field behind the enemy, the
// lens smear and glow, the shield's dome and sparks. Not ported yet, and hidden for now (none of
// them is on those two levels): the haze, the light shafts, dust specks, the heat shimmer and the
// light shafts in the lens. Figure details made with onBeforeCompile are skipped by WebGPU, all
// but the first-person squeeze toward the sight line (see `tune`); at battle distance the eyes'
// glow and the painted scalp do not show on these levels (checked close up, 4 Nov 2026).
import * as WEBGPU from "three/webgpu";
import type { Node } from "three/webgpu";
import {
  abs, acos, asin, atan, attribute, cameraProjectionMatrix, clamp, cos, Discard, dot, exp, float, floor, Fn, fract, frontFacing,
  fwidth, If, int, length, log2, max, min, mix, modelViewMatrix, modelViewProjection, normalize, normalView, positionGeometry,
  positionView, pow, screenCoordinate, select, sin, smoothstep, sqrt, step, texture, uniform, uv, varying,
  vec2, vec3, vec4,
} from "three/tsl";
import { Dust as WebGLDust } from "./dust.ts";
import { tune as webglTune } from "./materials.ts";
import type { Quality } from "./quality.ts";

type Fields = Record<string, unknown> & { name?: string; uniforms?: Record<string, { value: unknown }> };
type Shader = WEBGPU.NodeMaterial & { uniforms: Record<string, { value: unknown }>; setFeatures?: (bits: number) => void };
type V2 = Node<"vec2">;
type V3 = Node<"vec3">;
type V4 = Node<"vec4">;
type F = Node<"float">;
type Sampler = ReturnType<typeof texture>;

// what stands in for a texture Rust has not given yet (WGSL has no empty sampler)
const blank = new WEBGPU.DataTexture(new Uint8Array([0, 0, 0, 0]), 1, 1);
blank.needsUpdate = true;
const TEXTURE = /^(t[A-Z]|photo$|originalPhoto$|repairMask$|surfaceTile$|bakedTexture$)/;

/** Rust's `uniforms` with TSL nodes behind them: `.value` reads and writes the node's value. */
function wrap(source: Record<string, { value: unknown }> = {}): Shader["uniforms"] {
  const out: Shader["uniforms"] = {};
  for (const [key, { value }] of Object.entries(source)) {
    if (TEXTURE.test(key)) {
      out[key] = textureSlot(texture((value as WEBGPU.Texture | null) ?? blank));
    } else {
      out[key] = uniform(value as number) as never;
    }
  }
  return out;
}
function textureSlot(node: Sampler): { value: unknown; node: Sampler } {
  return {
    node,
    get value() { return node.value === blank ? null : node.value; },
    set value(v: unknown) { node.value = (v as WEBGPU.Texture | null) ?? blank; },
  };
}
function read(t: Sampler, at: V2): V4 {
  return t.sample(at) as unknown as V4;
}
function readLevel(t: Sampler, at: V2, level: F | number): V4 {
  return (t.sample(at) as unknown as { level(l: F | number): V4 }).level(level);
}
function sampler(u: Shader["uniforms"], key: string): Sampler {
  return (u[key] as unknown as { node: Sampler }).node;
}
function num(u: Shader["uniforms"], key: string): F {
  return u[key] as unknown as F;
}
function v2(u: Shader["uniforms"], key: string): V2 {
  return u[key] as unknown as V2;
}
function v3(u: Shader["uniforms"], key: string): V3 {
  return u[key] as unknown as V3;
}
function v4(u: Shader["uniforms"], key: string): V4 {
  return u[key] as unknown as V4;
}

// three.js ACES filmic, as the compose shader has it
function aces(color: V3, exposure: F): V3 {
  const c = color.mul(exposure.div(0.6));
  const x = vec3(0.59719, 0.07600, 0.02840).mul(c.x)
    .add(vec3(0.35458, 0.90834, 0.13383).mul(c.y)).add(vec3(0.04823, 0.01566, 0.83777).mul(c.z));
  const a = x.mul(x.add(0.0245786)).sub(0.000090537);
  const b = x.mul(x.mul(0.983729).add(0.4329510)).add(0.238081);
  const y = a.div(b);
  const o = vec3(1.60475, -0.10208, -0.00327).mul(y.x)
    .add(vec3(-0.53108, 1.10813, -0.07276).mul(y.y)).add(vec3(-0.07367, -0.00605, 1.07602).mul(y.z));
  return clamp(o, 0, 1) as V3;
}
function toSRGB(c: V3): V3 {
  return mix(c.mul(12.92), pow(c, vec3(1 / 2.4)).mul(1.055).sub(0.055), step(vec3(0.0031308), c)) as V3;
}
const LUMA = vec3(0.2126, 0.7152, 0.0722);

/**
 * The hall behind the fighters, focused on the enemy, as the compose shader's focused(): the
 * painting's depth map gives each pixel its distance, and the blur grows with how far it lies behind
 * him. `taps` is the quality's dofTaps: 0 none, 1 one read of a blurred copy, more a disc of reads.
 */
function focused(u: Shader["uniforms"], at: V2, taps: number): V3 {
  const bg = sampler(u, "tBg");
  const plain = read(bg, at).rgb;
  if (taps < 1) return plain;
  const depth = sampler(u, "tDepth"), bgSize = v2(u, "bgSize");
  const rows = vec2(depth.size(int(0)) as unknown as Node<"ivec2">).y;
  const level = log2(max(rows.mul(0.012), 1) as F);
  const coc = (p: V2): F => {
    const d = num(u, "focusInv").sub(readLevel(depth, p, level).r.mul(1.25)); // the map holds 1/z
    const behind = select(d.greaterThan(0), d, d.negate().mul(num(u, "dofNear"))) as F;
    return num(u, "dof").mul(min(behind, 0.13)).mul(bgSize.y);
  };
  const r = coc(at);
  let blurred: V3;
  if (taps < 2) {
    blurred = readLevel(bg, at, log2(max(r.mul(1.6), 0.001) as F)).rgb;
  } else {
    // a disc of reads, each from a copy blurred just enough to close the gaps between them
    const lod = log2(max(r.mul(2.2 / Math.sqrt(taps)), 1) as F);
    let sum = readLevel(bg, at, lod).rgb as V3;
    let wsum: F = float(1);
    for (let i = 0; i < taps; i++) {
      const rr = r.mul(Math.sqrt((i + 0.5) / taps)), a = i * 2.39996;
      const t = at.add(vec2(Math.cos(a), Math.sin(a)).mul(rr).div(bgSize));
      // something sharper than this pixel (nearer the focus) does not spread over it
      const w = clamp(coc(t).sub(rr).add(1), 0, 1) as F;
      sum = sum.add(readLevel(bg, t, lod).rgb.mul(w));
      wsum = wsum.add(w);
    }
    blurred = sum.div(wsum);
  }
  return select(num(u, "hasDepth").greaterThan(0.5).and(r.greaterThanEqual(0.6)), blurred, plain) as V3;
}

/** The compose pass: fighters over the hall, toned like the painting, the gun and hands on top. */
function compose(u: Shader["uniforms"], features: number, dofTaps: number): V4 {
  const minimal = (features & 32) !== 0, nvg = (features & 1) !== 0, glare = (features & 4) !== 0;
  const weather = (features & 8) !== 0;
  // minimal draws one flat layer of weather, low (lite) two, the rest three
  const wx: Wx = { minimal, lite: (features & 16) !== 0 || minimal, layers: minimal ? 1 : (features & 16) !== 0 ? 2 : 3 };
  const events = (features & 64) !== 0;
  const at = uv();
  const exposure = num(u, "exposure"), saturation = num(u, "saturation"), tint = v3(u, "tint");
  const hasBg = num(u, "hasBg");
  const tone = (f: V4): V3 => {
    const lifted = f.a.greaterThan(0.001);
    const lin = select(lifted, f.rgb.div(max(f.a, 0.001) as F), f.rgb) as V3;
    let c = toSRGB(nvg ? max(lin, vec3(0)) as V3 : aces(lin, exposure));
    c = (mix(vec3(dot(c, LUMA)), c, saturation) as V3).mul(tint);
    return select(lifted, c.mul(f.a), c) as V3;
  };
  const fig = sampler(u, "tFig");
  let f = read(fig, at);
  let hand: F = float(0);
  let rim: V4 = vec4(0);
  if (minimal) {
    // the gun and the hands are in the fighters' picture, marked by an alpha of one half
    const marked = (t: V4): F => float(1).sub(step(0.01, abs(t.a.sub(0.5))));
    hand = marked(f);
    f = vec4(f.rgb, mix(f.a, 1, hand) as F);
    // beside the gun a soft fringe of its colour, as the separate picture's smooth read gave
    const texel = vec2(1).div(vec2(fig.size(int(0)) as unknown as Node<"ivec2">));
    for (const o of [[-1, 0], [1, 0], [0, -1], [0, 1]]) {
      const t = read(fig, at.add(vec2(o[0], o[1]).mul(texel)));
      rim = rim.add(vec4(t.rgb, 1).mul(marked(t)));
    }
    rim = rim.mul(float(1).sub(hand));
  }
  const bg = select(hasBg.greaterThan(0.5), focused(u, at, minimal ? 0 : dofTaps), vec3(0)) as V3;
  let c = tone(f).add(bg.mul(float(1).sub(f.a)));
  const a = select(hasBg.greaterThan(0.5), float(1), f.a) as F;
  const c0 = c;
  if (weather) c = air(u, c, f, at, wx);
  let n: V4;
  if (minimal) {
    // the air's weather stays behind the gun and the hands
    c = mix(c, c0, hand) as V3;
    const fringe = min(rim.a.mul(0.2), 0.6) as F;
    const edge = mix(c, tone(vec4(rim.rgb.div(max(rim.a, 0.001) as F), 1)), fringe) as V3;
    c = select(rim.a.greaterThan(0), edge, c) as V3;
    n = vec4(0, 0, 0, max(hand, fringe) as F);
  } else {
    n = smoothRead(sampler(u, "tNear"), at, v2(u, "nearSize"));
    c = tone(n).add(c.mul(float(1).sub(n.a)));
  }
  if (weather) c = onLens(u, c, n, at, wx);
  if (events) {
    // a strike close by flashes the picture, a blackout darkens it
    const pulse = v4(u, "cdPulse");
    c = c.mul(pulse.w).add(pulse.rgb.mul(c.mul(0.7).add(0.3))) as V3;
  }
  const alpha = max(a, n.a) as F;
  if (glare) {
    const bgSize = v2(u, "bgSize");
    const delta = at.sub(v2(u, "cdGlareSun")).mul(vec2(bgSize.x.div(max(bgSize.y, 1) as F), 1));
    c = c.add(vec3(1.0, 0.83, 0.62).mul(exp(length(delta).mul(-5.5)).mul(0.14)));
  }
  if (nvg) {
    // white-phosphor intensifier, as nightVision.glsl
    const signal = pow(float(1).sub(exp((max(dot(c, LUMA), 0) as F).mul(-14))), 0.52) as F;
    return vec4(vec3(0.79, 0.91, 0.87).mul(signal), alpha);
  }
  return vec4(c, alpha);
}

function wxHash(at: V2): F {
  let p = fract(at.mul(vec2(123.34, 456.21)));
  p = p.add(dot(p, p.add(45.32)));
  return fract(p.x.mul(p.y));
}
function wxNoise(p: V2): F {
  const i = floor(p) as V2, f0 = fract(p);
  const f = f0.mul(f0).mul(vec2(3).sub(f0.mul(2)));
  return mix(
    mix(wxHash(i), wxHash(i.add(vec2(1, 0))), f.x),
    mix(wxHash(i.add(vec2(0, 1))), wxHash(i.add(vec2(1, 1))), f.x),
    f.y,
  ) as F;
}
// low quality drops the finest octave: it hardly shows through the lens's softness
function wxFbm(p: V2, lite: boolean): F {
  const a = wxNoise(p), b = wxNoise(p.mul(2.03).add(1.7));
  if (lite) return a.mul(0.62).add(b.mul(0.38));
  return a.mul(0.55).add(b.mul(0.3)).add(wxNoise(p.mul(4.1).add(5.2)).mul(0.15));
}

/** How rich the weather is: minimal (flat veils, one rain layer), low (lite: two layers) or full (three). */
interface Wx { minimal: boolean; lite: boolean; layers: number }

/** Weather in the air between the lens and the painting, as weather.glsl. */
function air(u: Shader["uniforms"], c: V3, f: V4, at: V2, o: Wx): V3 {
  return Fn(() => {
    const hasDepth = num(u, "hasDepth"), bgSize = v2(u, "bgSize");
    const w = v4(u, "cdWeather"), w2 = v4(u, "cdWeather2"), t = num(u, "cdWeatherTime");
    const dim = num(u, "cdWeatherDim");
    // the map holds 1/z; read soft, so a projected stage's proxy boxes leave no outlines in the air
    const depth = sampler(u, "tDepth");
    const rows = vec2(depth.size(int(0)) as unknown as Node<"ivec2">).y;
    const soft = readLevel(depth, at, log2(max(rows.mul(0.05), 1) as F)).r.mul(1.25);
    let inv = select(hasDepth.greaterThan(0.5), soft, float(0.06)) as F;
    inv = mix(inv, num(u, "focusInv"), step(0.5, f.a)) as F; // the enemy stands at his own depth
    const z = min(float(1).div(max(inv, 0.016) as F), 60) as F; // the open sky counts as far away
    const far = float(1).sub(exp(z.mul(-0.075)));
    const p = vec2(at.x.mul(bgSize.x).div(max(bgSize.y, 1) as F), at.y);
    const k = c.mul(dim).toVar();
    // fog: slow banks rolling across (one even layer on minimal), thicker low down
    If(w.x.greaterThan(0), () => {
      const bank = o.minimal ? float(0.5) : wxFbm(vec2(p.x.mul(2.2).add(t.mul(0.018)), p.y.mul(3).sub(t.mul(0.005))), o.lite);
      const fog = w.x.mul(float(1.15).sub(at.y.mul(0.35))).mul(float(1).sub(exp(z.negate().mul(bank.mul(0.06).add(0.045)))));
      k.assign(mix(k, v3(u, "cdFogColor"), clamp(fog, 0, 0.93)));
    });
    // salt dust: gusts of grains streaming sideways in layers (an even veil on minimal)
    If(w.y.greaterThan(0), () => {
      const gust = float(0.7).add(sin(t.mul(0.6).add(sin(t.mul(0.21)).mul(3))).mul(0.3));
      let veil: F = float(0.3), billow: F = float(0.5);
      if (!o.minimal) {
        const wave = sin(p.x.mul(3).add(t.mul(0.4))).mul(0.25);
        const s1 = wxFbm(vec2(p.x.mul(1.1).sub(t.mul(0.5)), p.y.mul(9).add(wave)), o.lite);
        const s2 = wxFbm(vec2(p.x.mul(2.6).sub(t.mul(1.3)), p.y.mul(24).sub(wave)).add(7.3), o.lite);
        veil = (smoothstep(0.42, 0.72, s1) as F).mul(0.7).add((smoothstep(0.48, 0.78, s2) as F).mul(0.5));
        billow = wxFbm(vec2(p.x.mul(1.3).sub(t.mul(0.8)), p.y.mul(1.8).add(t.mul(0.06))), o.lite);
      }
      const surge = w2.x;
      const dust = w.y.mul(gust).mul(float(1.2).sub(at.y.mul(0.4))).mul(far.mul(0.8).add(0.2)).mul(veil.mul(1.1).add(0.12))
        .add(surge.mul(billow.mul(0.25).add(0.8).add(veil.mul(0.1))));
      k.assign(mix(k, v3(u, "cdDustColor").mul(veil.mul(0.18).add(0.82)), clamp(dust, 0, surge.mul(0.09).add(0.88))));
      if (!o.minimal) {
        // single grains flicking past the lens: fine sideways specks, never round drops
        const g = vec2(p.x.mul(150).sub(t.mul(70)), at.y.mul(170).add(sin(p.x.mul(9).add(t)).mul(0.6)));
        const d = fract(g).sub(0.5).mul(vec2(0.45, 2.2));
        const grain = step(float(0.988).sub(surge.mul(0.025)), wxHash(floor(g) as V2)).mul(smoothstep(0.32, 0, length(d)));
        k.assign(mix(k, v3(u, "cdDustColor"), grain.mul(w.y).mul(surge.mul(0.4).add(0.3))));
      }
    });
    // rain: thin slanted streaks in depth layers, hidden behind what stands nearer
    If(w.z.greaterThan(0), () => {
      let rain: F = float(0);
      for (let i = 0; i < o.layers; i++) {
        const cols = 70 + i * 60;
        const q = vec2(p.x.add(at.y.mul(0.07)).mul(cols), at.y.mul(cols * 0.09).add(t.mul(2.6 - i * 0.55)));
        const h = wxHash((floor(q) as V2).add(i * 17));
        const x = fract(q.x).sub(0.5).add(h.sub(0.5).mul(0.5));
        const y = fract(q.y.add(h.mul(7)));
        const streak = (smoothstep(0.09, 0, abs(x)) as F).mul(smoothstep(0, 0.3, y)).mul(smoothstep(1, 0.6, y)).mul(step(0.62, h));
        rain = rain.add(streak.mul(step(2.5 + i * 4.5, z)).mul(0.5 - i * 0.12));
      }
      k.assign(mix(k, v3(u, "cdRainColor"), clamp(rain.mul(w.z), 0, 0.55)));
    });
    // wet ground: rings where the drops hit the floor, a darker soaked surface and a dull sheen
    const below = w2.w.sub(at.y);
    If(w2.z.greaterThan(0).and(below.greaterThan(0.004)).and(f.a.lessThan(0.5)), () => {
      const ground = vec2(at.x.sub(0.5).mul(bgSize.x).div(max(bgSize.y, 1) as F), 1).mul(float(1.6).div(below)); // metres, roughly
      // only the floor itself: anything standing up is nearer than the floor behind it
      const onFloor = select(hasDepth.greaterThan(0.5), smoothstep(0.55, 0.8, z.div(ground.y)), float(1)) as F;
      const wet = w2.z.mul(onFloor).mul(smoothstep(40, 6, ground.y));
      if (o.minimal) {
        k.assign(k.mul(float(1).sub(wet.mul(0.12)))); // minimal: the soaked floor only darkens
        return;
      }
      const puddle = smoothstep(0.5, 0.75, wxFbm(ground.mul(0.35), o.lite)) as F;
      const sky = readLevel(sampler(u, "tBg"), vec2(at.x, min(w2.w.add(below.mul(0.6)), 0.999)), 4).rgb.mul(dim);
      k.assign(mix(k.mul(float(1).sub(wet.mul(0.12))), sky.mul(0.55), wet.mul(puddle).mul(0.22)));
      let rings: F = float(0);
      for (let i = 0; i < o.layers - 1; i++) {
        const cell = ground.div(0.45).add(i * 0.5), id = floor(cell) as V2;
        const clock = t.mul(1.6).add(wxHash(id.add(i * 9)).mul(7));
        const life = fract(clock);
        const key = id.add(floor(clock).mul(1.37));
        const at0 = vec2(wxHash(key.add(1.1)), wxHash(key.add(2.3))).mul(0.6).add(0.2);
        const dist = length(fract(cell).sub(at0)).mul(0.45);
        const width = max(ground.y.mul(ground.y).mul(0.0014), 0.025) as F; // never thinner than a pixel
        const ring = (smoothstep(width, 0, abs(dist.sub(life.mul(0.2)))) as F).mul(float(1).sub(life)).mul(step(0.2, wxHash(key.add(3.7))));
        const hit = (smoothstep(0.025, 0, dist) as F).mul(smoothstep(0.12, 0, life));
        rings = rings.add(ring).add(hit.mul(1.5));
      }
      if (o.layers > 1) {
        // far off the rings flatten into lines along the horizon, so only the near floor has them
        const near = smoothstep(14, 5, ground.y) as F;
        const lum = dot(k, vec3(0.3, 0.5, 0.2));
        k.addAssign(v3(u, "cdRainColor").mul(clamp(rings.mul(wet).mul(near), 0, 1)).mul(lum.mul(0.6).add(0.22).add(w.w.mul(0.6))));
      }
    });
    // lightning: the far, open parts flare first, the floor near the lens least
    If(w.w.greaterThan(0), () => {
      const lit = w.w.mul(far.mul(0.7).add(0.3));
      k.assign(k.mul(lit.mul(1.5).add(1)).add(vec3(0.78, 0.84, 0.92).mul(lit.mul(0.1))));
    });
    return select(num(u, "hasBg").greaterThan(0.5), k, c);
  })() as unknown as V3;
}

// drops sitting on the glass, one or none per cell: the slope in xy, the cover in z (weatherCommon.glsl)
function drops(q: V2, cell: number, t: F, seed: number, amount: F): V3 {
  const g = q.mul(cell), id = floor(g) as V2;
  const rate = wxHash(id.add(seed + 1.3)).mul(0.05).add(0.03);
  const age = t.mul(rate).add(wxHash(id.add(seed)).mul(5));
  const k = id.add(seed).add(floor(age).mul(1.618));
  const life = fract(age);
  const live = (smoothstep(0, 0.02, life) as F).mul(smoothstep(1, 0.55, life));
  const o = vec2(wxHash(k.add(2.1)), wxHash(k.add(4.7))).mul(0.44).add(0.28);
  const r = wxHash(k.add(6.1)).mul(0.12).add(0.09).mul(mix(1, 0.7, life));
  const d0 = fract(g).sub(o).div(r);
  const d = vec2(d0.x, d0.y.mul(wxHash(k.add(8.3)).mul(0.2).add(0.9))); // never quite round
  const cover = (smoothstep(1, 0.5, length(d)) as F).mul(live); // the glass is out of focus: soft edges
  return select(wxHash(k.add(9.7)).lessThan(float(1).sub(amount)), vec3(0), vec3(d.mul(cover), cover)) as V3;
}

// a drop running down the glass in its own column, leaving a broken trail of beads
function slide(q: V2, cols: number, t: F, seed: number): V3 {
  const col = floor(q.x.mul(cols)) as F;
  const h = wxHash(vec2(col, seed));
  const speed = wxHash(vec2(col, seed + 3.3)).mul(0.16).add(0.09);
  const head = float(1.2).sub(fract(t.mul(speed).add(h.mul(7))).mul(1.6));
  const x = fract(q.x.mul(cols)).sub(0.5).add(sin(q.y.mul(21).add(h.mul(6))).mul(0.18)).div(cols); // the track wanders
  const d = vec2(x, q.y.sub(head).mul(0.85)).div(0.012);
  const cover = smoothstep(1, 0.5, length(d)) as F;
  const drop = vec3(d.mul(cover), cover);
  const above = q.y.sub(head);
  const row = floor(q.y.mul(110)) as F;
  const b = vec2(x, fract(q.y.mul(110)).sub(0.5).div(110)).div(0.0032);
  const bead = (smoothstep(1, 0.7, length(b)) as F).mul(step(0.55, wxHash(vec2(col, row).add(seed)))).mul(float(1).sub(above.div(0.3)));
  const beaded = above.greaterThan(0).and(above.lessThan(0.3)).and(bead.greaterThan(drop.z));
  return select(h.lessThan(0.6), vec3(0), select(beaded, vec3(b.mul(bead), bead), drop)) as V3;
}

/** What sits on the lens itself, over the gun and the hands too, as weatherLens.glsl. */
function onLens(u: Shader["uniforms"], c: V3, n: V4, at: V2, o: Wx): V3 {
  return Fn(() => {
    const w = v4(u, "cdWeather"), w2 = v4(u, "cdWeather2"), t = num(u, "cdWeatherTime");
    const nearSize = v2(u, "nearSize");
    const aspect = nearSize.x.div(max(nearSize.y, 1) as F);
    const q = vec2(at.x.mul(aspect), at.y);
    const dustColor = v3(u, "cdDustColor");
    const k = c.toVar();
    // dust: a gust thickens until it swallows even the gun, and grains catch on the glass
    If(w2.y.greaterThan(0), () => {
      const surge = w2.x;
      const roll = o.minimal ? float(0.5) : wxFbm(vec2(q.x.mul(1.6).sub(t.mul(0.9)), q.y.mul(2.4).add(t.mul(0.12))), o.lite);
      k.assign(mix(k, dustColor.mul(roll.mul(0.12).add(0.88)), clamp(surge.mul(roll.mul(0.25).add(0.3)).mul(n.a), 0, 0.6)));
      // out-of-focus specks: a few stuck on the glass, more blown past in a gust
      let specks: F = float(0);
      for (let i = 0; i < o.layers - 1; i++) {
        const g0 = q.mul(7 + i * 6);
        const gx = g0.x.sub(t.mul(i * 2.6));
        const g = vec2(gx, g0.y.add(sin(gx.mul(0.7).add(i)).mul(0.15 * i)));
        const id = floor(g) as V2;
        const many = i === 0 ? w2.y.mul(0.12).add(surge.mul(0.25)) : w2.y.mul(0.05).add(surge.mul(0.5));
        const show = clamp(wxHash(id.add(i * 31)).sub(float(1).sub(many)).div(0.06), 0, 1) as F;
        const at0 = vec2(wxHash(id.add(3.1)), wxHash(id.add(7.7))).mul(0.5).add(0.25);
        const r = wxHash(id.add(11.3)).mul(0.15).add(0.07);
        const d = length(fract(g).sub(at0));
        // a lit disc with a slightly brighter rim, the way out-of-focus grit looks
        specks = specks.add((smoothstep(r, r.mul(0.6), d) as F).mul((smoothstep(r.mul(0.3), r.mul(0.85), d) as F).mul(0.3).add(0.7))
          .mul(show).mul(0.75 - i * 0.25));
      }
      if (o.layers > 1) k.assign(mix(k, dustColor.mul(1.18), clamp(specks.mul(0.32), 0, 0.55)));
      if (!o.minimal) {
        // grains streaking past right in front of the lens
        const sg = vec2(q.x.mul(38).sub(t.mul(60)), q.y.mul(210));
        const sd = fract(sg).sub(0.5).mul(vec2(0.28, 2));
        const streak = step(float(0.994).sub(surge.mul(0.012)), wxHash(floor(sg) as V2)).mul(smoothstep(0.35, 0, length(sd)));
        k.assign(mix(k, dustColor.mul(1.1), streak.mul(surge.mul(0.45).add(0.15)).mul(w2.y)));
      }
    });
    // rain: drops on the glass bend the view behind them; the gun and the hands are wet
    If(w2.z.greaterThan(0), () => {
      const wet = w2.z;
      let drop = drops(q, 4.5, t, 1, wet.mul(0.4));
      if (!o.minimal) {
        // and smaller drops, one running down the glass (minimal: only the big drops)
        const d2 = drops(q, 10, t, 7, wet.mul(0.5));
        drop = select(d2.z.greaterThan(drop.z), d2, drop) as V3;
        if (!o.lite) {
          const d3 = drops(q, 23, t, 13, wet.mul(0.55));
          drop = select(d3.z.greaterThan(drop.z), d3, drop) as V3;
        }
        const sl = slide(q, 7, t, 21);
        drop = select(sl.z.greaterThan(drop.z), sl, drop) as V3;
      }
      // wet metal and gloves: a little darker, with beads that catch the light
      const lit = dot(k, vec3(0.3, 0.5, 0.2));
      k.assign(k.mul(float(1).sub(wet.mul(0.14).mul(n.a))));
      if (!o.minimal) {
        // beads sit only on the gun and the hands, and in one cell of eight
        const bq = q.mul(70), cell = floor(bq) as V2;
        const bd = fract(bq).sub(0.25).sub(vec2(wxHash(cell.add(2)), wxHash(cell.add(3))).mul(0.5))
          .div(wxHash(cell.add(4)).mul(0.16).add(0.12));
        const bead = smoothstep(1, 0.3, length(bd)) as F;
        const on = n.a.greaterThan(0).and(wxHash(cell.add(5)).greaterThanEqual(0.88));
        k.addAssign(select(on, vec3(0.9, 0.94, 1).mul(bead.mul(n.a).mul(wet).mul(lit.mul(0.5).add(0.05).add(w.w.mul(0.8)))), vec3(0)));
      }
      If(drop.z.greaterThan(0.001), () => {
        const s = drop.xy.div(drop.z);
        const l2 = min(dot(s, s), 1) as F;
        // a drop out of focus on the glass: a soft smudge of the view round it, a little bent
        const ruv = clamp(at.sub(s.mul(vec2(float(0.03).div(aspect), 0.03))), vec2(0.001), vec2(0.999)) as V2;
        const behind = readLevel(sampler(u, "tBg"), ruv, 3.2).rgb.mul(num(u, "cdWeatherDim"));
        const seen = mix(select(num(u, "hasBg").greaterThan(0.5), behind, k) as V3, k.mul(0.95), n.a) as V3;
        const normal = vec3(s, sqrt(float(1).sub(l2)));
        const glint = pow(max(dot(normal, normalize(vec3(-0.45, 0.6, 0.66))), 0) as F, 28);
        const inside = seen.mul(float(1).sub((smoothstep(0.4, 1, l2) as F).mul(0.3))) // the edge bends some light away
          .add(vec3(0.9, 0.94, 1).mul(glint.mul(dot(seen, vec3(0.33)).mul(0.6).add(0.1).add(w.w))));
        k.assign(mix(k, inside, drop.z.mul(wet.mul(0.3).add(0.6))));
      });
    });
    return k;
  })() as unknown as V3;
}

// a smooth B-spline read of a small picture, as the compose shader's smoothRead
function smoothRead(t: Sampler, at: V2, size: V2): V4 {
  const st = at.mul(size).sub(0.5), i = st.floor() as V2, f = st.sub(i);
  const f2 = f.mul(f), f3 = f2.mul(f);
  const w0 = float(1).sub(f.mul(3)).add(f2.mul(3)).sub(f3).div(6);
  const w1 = float(4).sub(f2.mul(6)).add(f3.mul(3)).div(6);
  const w2 = float(1).add(f.mul(3)).add(f2.mul(3)).sub(f3.mul(3)).div(6);
  const w3 = f3.div(6);
  const g0 = w0.add(w1), g1 = w2.add(w3);
  const h0 = i.sub(0.5).add(w1.div(g0)).div(size), h1 = i.add(1.5).add(w3.div(g1)).div(size);
  const at2 = (x: F, y: F) => read(t, vec2(x, y));
  return at2(h0.x, h0.y).mul(g0.x).add(at2(h1.x, h0.y).mul(g1.x)).mul(g0.y)
    .add(at2(h0.x, h1.y).mul(g0.x).add(at2(h1.x, h1.y).mul(g1.x)).mul(g1.y));
}

// a different value in every pixel, as the lens shader's ign
function ign(p: V2): F {
  return fract(fract(dot(p, vec2(0.06711056, 0.00583715))).mul(52.9829189));
}

// red through green to blue as t goes 0..1, half white: the colour of each tap of the lens smear
function tapColor(t: number): V3 {
  const w = Math.min(Math.max(1 - Math.abs(2 * Math.min(Math.max((t - 1 / 6) / (4 / 6), 0), 1) - 1), 0), 1);
  const lo = t <= 0.5 ? 1 : 0;
  const [r, g, b] = [lo * (1 - w), w, (1 - lo) * (1 - w)].map(x => 0.55 + 0.45 * x);
  return vec3(r, g, b);
}

// FSR1 EASU (after AMD's ffx_fsr1.h, MIT), as the lens shader's easu(): 12 texels around the point,
// a Lanczos-like kernel stretched along the local edge, clamped to the 4 nearest texels
function easu(t: Sampler, at: V2, size: V2): V3 {
  const pp0 = at.mul(size).sub(0.5), fp = floor(pp0) as V2, pp = pp0.sub(fp);
  const texel = (x: number, y: number) => readLevel(t, fp.add(vec2(x + 0.5, y + 0.5)).div(size), 0).rgb;
  const luma = (c: V3) => c.g.add(c.r.add(c.b).mul(0.5));
  const b = texel(0, -1), c = texel(1, -1);
  const e = texel(-1, 0), f = texel(0, 0), g = texel(1, 0), h = texel(2, 0);
  const i = texel(-1, 1), j = texel(0, 1), k = texel(1, 1), l = texel(2, 1);
  const n = texel(0, 2), o = texel(1, 2);
  const [bL, cL, eL, fL, gL, hL, iL, jL, kL, lL, nL, oL] = [b, c, e, f, g, h, i, j, k, l, n, o].map(luma) as [F, F, F, F, F, F, F, F, F, F, F, F];
  let dir: V2 = vec2(0), len: F = float(0);
  const set = (w: F, lA: F, lB: F, lC: F, lD: F, lE: F) => {
    const dirX = lD.sub(lB), dirY = lE.sub(lA);
    const lenX = clamp(abs(dirX).div(max(max(abs(lD.sub(lC)), abs(lC.sub(lB))), 1e-5)), 0, 1) as F;
    const lenY = clamp(abs(dirY).div(max(max(abs(lE.sub(lC)), abs(lC.sub(lA))), 1e-5)), 0, 1) as F;
    dir = dir.add(vec2(dirX, dirY).mul(w));
    len = len.add(lenX.mul(lenX).add(lenY.mul(lenY)).mul(w));
  };
  const ix = float(1).sub(pp.x), iy = float(1).sub(pp.y);
  set(ix.mul(iy), bL, eL, fL, gL, jL);
  set(pp.x.mul(iy), cL, fL, gL, hL, kL);
  set(ix.mul(pp.y), fL, iL, jL, kL, nL);
  set(pp.x.mul(pp.y), gL, jL, kL, lL, oL);
  const dirR = dot(dir, dir);
  const d = select(dirR.lessThan(1 / 32768), vec2(1, 0), dir.div(sqrt(max(dirR, 1e-12)))) as V2;
  const half = len.mul(0.5), lenQ = half.mul(half);
  const stretch = dot(d, d).div(max(abs(d.x), abs(d.y)));
  const len2 = vec2(float(1).add(stretch.sub(1).mul(lenQ)), float(1).sub(lenQ.mul(0.5)));
  const lob = float(0.5).add(lenQ.mul(0.21 - 0.5)), clp = float(1).div(lob);
  let aC: V3 = vec3(0), aW: F = float(0);
  const tap = (x: number, y: number, col: V3) => {
    const off = vec2(x, y).sub(pp);
    const v = vec2(dot(off, d), dot(off, vec2(d.y.negate(), d.x))).mul(len2);
    const d2 = min(dot(v, v), clp);
    const wB0 = d2.mul(0.4).sub(1), wA0 = lob.mul(d2).sub(1);
    const w = wB0.mul(wB0).mul(1.5625).sub(0.5625).mul(wA0.mul(wA0));
    aW = aW.add(w); aC = aC.add(col.mul(w));
  };
  tap(0, -1, b); tap(1, -1, c); tap(-1, 1, i); tap(0, 1, j); tap(0, 0, f); tap(-1, 0, e);
  tap(1, 1, k); tap(2, 1, l); tap(2, 0, h); tap(1, 0, g); tap(1, 2, o); tap(0, 2, n);
  return clamp(aC.div(aW), min(min(f, g), min(j, k)), max(max(f, g), max(j, k))) as V3;
}

/** The lens pass to the screen: the bend, the smear and glow at the edges (`taps` > 0), contrast, vignette and dither (no rays yet). */
function lens(u: Shader["uniforms"], taps: number): V4 {
  const at = uv(), center = v2(u, "center");
  const d = at.sub(center).div(v2(u, "halfView"));
  const aspect = num(u, "aspect");
  const e = d.mul(vec2(aspect, 1)).div(length(vec2(aspect, 1)));
  const src = center.add(at.sub(center).mul(float(1).add(num(u, "barrel").mul(dot(e, e)))));
  const q = dot(d, d).mul(0.5);
  const comp = sampler(u, "tComp");
  const o = vec2(0.25).div(v2(u, "outSize"));
  const near = (x: F, y: F) => read(comp, src.add(vec2(x, y)));
  const four = near(o.x.negate(), o.y.mul(-0.5)).add(near(o.x.mul(0.5), o.y.negate()))
    .add(near(o.x, o.y.mul(0.5))).add(near(o.x.mul(-0.5), o.y)).mul(0.25);
  const plain = select(num(u, "ssaa").lessThanEqual(1), read(comp, src), four) as V4;
  // a screen larger than the frame (FSR): the frame is enlarged along its edges, not blurred
  let c = Fn(() => {
    const picked = plain.toVar();
    If(num(u, "fsr").greaterThan(0.5), () => { picked.assign(vec4(easu(comp, src, v2(u, "compSize")), plain.a)); });
    return picked;
  })() as unknown as V4;
  let rgb: V3 = c.rgb;
  if (taps > 0) {
    // the taps read a blurred copy of the frame, as blurred as they are far apart, so they flow into one smear
    const outSize = v2(u, "outSize");
    const amount = num(u, "smear").mul(smoothstep(0.08, 1, q));
    const r = num(u, "soft").mul(smoothstep(0.15, 1, q));
    const span = amount.mul(2).mul(length(src.sub(center).mul(outSize))).div(taps);
    const lod = log2((max(max(span.mul(2), r.mul(outSize.y).mul(1.5)), 1) as F).mul(num(u, "ssaa")));
    // each pixel starts its taps at its own offset, so a busy pattern near the edge does not break into rows of copies
    const jitter = ign(screenCoordinate.xy.add(17));
    let sum: V3 = vec3(0), wsum: V3 = vec3(0), alpha: F = float(0);
    for (let i = 0; i < taps; i++) {
      const w = tapColor((i + 0.5) / taps);
      const t = jitter.add(i).div(taps);
      // red spreads outwards
      const s = readLevel(comp, center.add(src.sub(center).mul(amount.mul(t.sub(0.5)).mul(2).add(1))), lod);
      sum = sum.add(s.rgb.mul(w)); wsum = wsum.add(w); alpha = alpha.add(s.a);
    }
    const clean = amount.lessThan(0.0004).and(r.lessThan(0.0004));
    c = select(clean, c, vec4(sum.div(wsum), alpha.div(taps))) as V4;
    // the bright parts bleed light over everything near them: two blurred copies, only what is near white
    const glowLod = num(u, "glowLod");
    const halo = max(readLevel(comp, src, glowLod.add(4.5)).rgb.sub(0.55), 0)
      .add(max(readLevel(comp, src, glowLod.add(6)).rgb.sub(0.5), 0));
    rgb = c.rgb.add(halo.mul(num(u, "glow")).mul(num(u, "hasBg")));
  }
  rgb = clamp(rgb, 0, 1) as V3;
  rgb = mix(rgb, rgb.mul(rgb).mul(vec3(3).sub(rgb.mul(2))), num(u, "contrast").mul(num(u, "hasBg"))) as V3;
  const v = smoothstep(0.45, 1, q) as F;
  const dark = float(1).sub(num(u, "vignette").mul(v).mul(sqrt(v)));
  const dither = ign(screenCoordinate.xy).sub(0.5).div(255);
  return vec4(clamp(rgb.mul(dark).add(dither), 0, 1) as V3, c.a);
}

/** The projected hall: the photograph glued to the proxy geometry, as projectedStage*.glsl. */
function stage(u: Shader["uniforms"]): V4 {
  const sourceClip = varying(
    (u.sourceViewProjection as unknown as Node<"mat4">).mul(vec4(positionGeometry, 1)),
  ) as unknown as V4;
  const position = varying(positionGeometry) as unknown as V3;
  const w = sourceClip.w;
  const sourceW = select(w.lessThan(0), min(w, -0.00001), max(w, 0.00001)) as F;
  const at = sourceClip.xy.div(sourceW).mul(0.5).add(0.5);
  const inside = clamp(at, vec2(0.001), vec2(0.999)) as V2;
  const side = v3(u, "sideColor").mul(v3(u, "sideTint")).mul(num(u, "sideExposure"));
  let color = select(num(u, "isSide").greaterThan(0.5), side, read(sampler(u, "photo"), inside).rgb) as V3;
  const repaired = mix(
    read(sampler(u, "originalPhoto"), inside).rgb, color, read(sampler(u, "repairMask"), inside).r,
  ) as V3;
  color = select(num(u, "useRepair").greaterThan(0.5), repaired, color) as V3;
  const kind = num(u, "surfaceKind");
  const tileAt = (select(kind.greaterThan(1.5).and(kind.lessThan(2.5)), position.zy, position.xz) as V2).mul(0.25);
  const extension = read(sampler(u, "surfaceTile"), tileAt).rgb
    .mul(v3(u, "extensionTint")).mul(num(u, "extensionExposure"));
  const edge = (x: F) => smoothstep(0, 0.08, x) as F;
  const photograph = edge(at.y).mul(edge(float(1).sub(at.y))).mul(edge(at.x)).mul(edge(float(1).sub(at.x)))
    .mul(step(0, sourceClip.w));
  color = select(kind.greaterThan(0.5), mix(extension, color, photograph), color) as V3;
  const baked = read(sampler(u, "bakedTexture"), uv()).rgb;
  return vec4(select(num(u, "useBakedUv").greaterThan(0.5), baked, color) as V3, 1);
}

// GLSL's mod, which floors (WGSL's % truncates)
function fmod<T extends V2 | F>(x: T, y: T | V2 | number): T {
  return x.sub((y as V2).mul(floor(x.div(y as V2)))) as T;
}
function shieldHash(p: V2): F {
  return fract(sin(dot(p, vec2(127.1, 311.7))).mul(43758.5453));
}
// the hit's sphere point from a longitude and latitude
function onSphere(lon: F, lat: F): V3 {
  return vec3(cos(lat).mul(cos(lon)), sin(lat), cos(lat).mul(sin(lon)));
}
function angleTo(a: V3, b: V3): F {
  return acos(clamp(dot(a, b), -1, 1) as F);
}

/** Pulls the picture in toward the sight line, as my first-person body is (the «wide» squeeze). */
function squeezed(clip: V4, u: Shader["uniforms"]): V4 {
  const k = uniform(1).onRenderUpdate(() => Number(u.wideK?.value ?? 1));
  const y = uniform(0).onRenderUpdate(() => Number(u.wideY?.value ?? 0));
  return vec4(mix(vec2(0, y).mul(clip.w), clip.xy, k) as V2, clip.z, clip.w);
}

/**
 * The «Зонт» dome, as shieldDome*.glsl: only the section over the hit zone lights up, a hot
 * point, a wave over its hex cells, a glassy rim, then a fade; in heat and rain a faint patch
 * here and there. Light is added (alpha 0).
 */
function shieldDome(u: Shader["uniforms"]): { vertex: V4; fragment: V4 } {
  const vLocal = varying(positionGeometry) as unknown as V3;
  const vNormal = varying(normalView) as unknown as V3;
  const vView = varying(positionView) as unknown as V3;
  const hit = v3(u, "uHit"), age = num(u, "uAge"), life = num(u, "uLife"), reach0 = num(u, "uReach");
  const cells = num(u, "uCells"), detail = num(u, "uDetail"), heatK = num(u, "uHeat"), wetK = num(u, "uWet");
  const time = num(u, "uTime"), tint = v3(u, "uColor");
  const fragment = Fn(() => {
    const k = clamp(age.div(max(life, 0.001) as F), 0, 1) as F;
    const p = normalize(vLocal) as V3;
    const d = angleTo(p, hit);
    const resting = heatK.add(wetK);
    If(d.greaterThan(reach0.mul(1.3)).and(resting.lessThanEqual(0)), () => { Discard(); });
    // hex cells: the longitude holds a whole number of cells, so the seam does not show
    const toAngle = float(6.2831853).div(cells);
    const lon = atan(p.z, p.x) as F;
    const at = vec2(lon, asin(p.y)).div(toAngle);
    const R = vec2(1, 1.7320508);
    const a = fmod(at, R).sub(R.mul(0.5));
    const b = fmod(at.sub(R.mul(0.5)), R).sub(R.mul(0.5));
    const g = select(dot(a, a).lessThan(dot(b, b)), a, b) as V2;
    const q = abs(g) as V2;
    const edge = float(0.5).sub(max(dot(q, vec2(0.5, 0.8660254)), q.x));
    const line = float(1).sub(smoothstep(0, fwidth(edge).mul(1.2).add(0.015), edge));
    const centre = at.sub(g).mul(toAngle);
    const cell = shieldHash(floor(at.sub(g).mul(2).add(0.5)) as V2);
    const cp = onSphere(centre.x, centre.y);
    const dc = angleTo(cp, hit);
    // the lit section: whole cells near the hit, its rim a little ragged, fading off softly
    const reach = reach0.mul((smoothstep(0, 0.12, age) as F).mul(0.45).add(0.55));
    const inside = smoothstep(reach, reach.sub(0.06), dc.add(cell.sub(0.5).mul(0.1))) as F;
    const soft = smoothstep(reach0.mul(1.25), reach0.mul(0.3), d) as F;
    const mask = inside.mul(soft);
    // the wave runs out from the hit to the section's edge; a weaker echo follows
    const front = reach0.mul(1.1).mul(float(1).sub(exp(age.mul(-7))));
    const band = k.mul(0.06).add(0.035);
    const wave = exp(pow(d.sub(front).div(band), 2).negate());
    const echo = detail.mul(0.5).mul(exp(pow(d.sub(front.mul(0.55)).div(band.mul(0.8)), 2).negate()));
    const fade = pow(float(1).sub(k), 1.4);
    const rise = smoothstep(0, 0.03, age) as F;
    // seen edge-on the section reads like thick curved glass
    const edgeOn = pow(float(1).sub(abs(dot(normalize(vNormal), normalize(vView.negate())))), 2);
    const core = exp(d.mul(d).mul(-400)).mul(exp(age.mul(-14))).mul(4);
    const glow = exp(d.mul(d).mul(-40)).mul(exp(age.mul(-9)));
    const fill = (smoothstep(0.7, 1, cell) as F).mul(glow).mul(detail.mul(0.3).add(0.2));
    const field = wave.add(echo).mul(fade);
    const plate = edgeOn.mul(0.3).add(0.04).mul(rise).mul(fade);
    const hex = line.mul(field.mul(1.4).add(glow.mul(0.9)).add(plate.mul(2)));
    const light = plate.add(field.mul(0.3)).add(glow.mul(0.35)).add(hex).add(fill).mul(mask).add(core).toVar();
    // a broken guard: cells round the hit drop out and the section stutters
    If(num(u, "uBroken").greaterThan(0.5), () => {
      const hole = (smoothstep(reach0.mul(0.8), reach0.mul(0.2), d) as F).mul(step(cell, k.mul(1.1).add(0.2)));
      light.mulAssign(float(1).sub(hole).mul(step(0.4, shieldHash(vec2(floor(age.mul(32)), 7))).mul(0.5).add(0.5)));
    });
    const facing = select(frontFacing, float(1), float(0.35)) as F;
    light.mulAssign(select(age.lessThan(life), float(1), float(0)).mul(facing));
    const color = mix(tint, vec3(1), clamp(core.mul(0.4).add(glow.mul(0.25)), 0, 1)) as V3;
    const out = color.mul(light).mul(num(u, "uPower")).toVar();
    // the field at rest: a small patch of cells surfaces here and there, hot air rippling through it or rain running down it
    If(resting.greaterThan(0), () => {
      let spots: F = float(0);
      for (let i = 0; i < 3; i++) {
        const clock = time.mul(0.45).add(i * 0.37);
        const slot = floor(clock) as F;
        const y = mix(-0.35, 0.85, shieldHash(vec2(float(i + 4.1), slot))) as F;
        const around = shieldHash(vec2(slot, i)).mul(6.2831853);
        const ring = sqrt(float(1).sub(y.mul(y)));
        const spot = vec3(ring.mul(cos(around)), y, ring.mul(sin(around)));
        const size = shieldHash(vec2(slot.add(2.7), i)).mul(0.16).add(0.2);
        // measured from cell centres, so a patch is made of whole cells with a ragged edge
        const ds = angleTo(cp, spot).add(cell.sub(0.5).mul(0.12));
        spots = max(spots, (smoothstep(size, size.mul(0.45), ds) as F).mul(sin(fract(clock).mul(3.1415927)))) as F;
      }
      const ripple = sin(p.y.mul(9).sub(time.mul(1.7)).add(sin(lon.mul(5).add(time.mul(0.6))).mul(1.5))).mul(0.4).add(0.6);
      const glass = line.mul(0.1).add(0.02).add(edgeOn.mul(0.05)).add((smoothstep(0.8, 1, cell) as F).mul(0.025));
      const heat = heatK.mul(ripple).mul(glass);
      const around = lon.div(6.2831853).mul(cells).mul(1.5);
      const hc = shieldHash(vec2(floor(around), 3));
      const phase = fract(p.y.mul(2.5).add(time.mul(hc.mul(0.4).add(0.35))).add(hc.mul(9)));
      const across = abs(fract(around).sub(0.5));
      const run = step(0.5, hc).mul(smoothstep(0.2, 0, across))
        .mul((smoothstep(0.05, 0, phase) as F).add(exp(phase.mul(-10)).mul(0.25)));
      const wet = wetK.mul(glass.add(run.mul(0.25)));
      out.addAssign(mix(tint, vec3(1), 0.35).mul(heat.add(wet).mul(spots).mul(facing)));
    });
    return vec4(out, 0);
  })() as unknown as V4;
  return { vertex: squeezed(modelViewProjection as unknown as V4, u), fragment };
}

/** Sparks thrown off the dome, as shieldSpark*.glsl: each a short streak, its flight worked out from the hit. */
function shieldSparks(u: Shader["uniforms"]): { vertex: V4; fragment: V4 } {
  const dir = attribute("aDir", "vec3") as unknown as V3;
  const spark = attribute("aSpark", "vec4") as unknown as V4; // speed m/s, life s, width, warm (1 = the bullet going off)
  const corner = attribute("aCorner", "vec2") as unknown as V2; // along the streak (0 tail, 1 head), across it (-1, 1)
  const origin = v3(u, "uOrigin"), age = num(u, "uAge"), aspect = num(u, "uAspect");
  // drag slows a spark fast, then it falls; the bullet keeps its speed
  const drag = mix(4.5, 0.4, spark.w) as F;
  const flight = (t: F): V3 => origin.add(dir.mul(spark.x).mul(float(1).sub(exp(drag.negate().mul(t)))).div(drag))
    .add(vec3(0, float(-4.9).mul(float(1).sub(spark.w)), 0).mul(t.mul(t)));
  const project = (p: V3): V4 => (cameraProjectionMatrix as unknown as Node<"mat4">).mul(modelViewMatrix as unknown as Node<"mat4">).mul(vec4(p, 1)) as unknown as V4;
  const life = spark.y;
  const t = min(age, life) as F;
  const head = project(flight(t));
  const tail = project(flight(max(t.sub(0.035).sub(spark.w.mul(0.035)), 0) as F));
  const c = mix(tail, head, corner.x) as V4;
  const s0 = head.xy.div(max(head.w, 1e-3) as F).sub(tail.xy.div(max(tail.w, 1e-3) as F));
  const s = vec2(s0.x.mul(aspect), s0.y);
  const side = select(dot(s, s).greaterThan(1e-10), normalize(vec2(s.y.negate(), s.x)), vec2(0, 1)) as V2;
  const across = vec2(side.x.div(aspect), side.y);
  const moved = vec4(c.xy.add(across.mul(corner.y).mul(num(u, "uWidth")).mul(spark.z).mul(c.w)), c.z, c.w);
  const gone = age.greaterThanEqual(life).or(head.w.lessThan(0.05)).or(tail.w.lessThan(0.05));
  const vertex = squeezed(select(gone, vec4(2, 2, 2, 1), moved) as V4, u);
  const vCorner = varying(corner) as unknown as V2;
  const vHeat = varying(float(1).sub(t.div(life))) as unknown as F;
  const vWarm = varying(spark.w) as unknown as F;
  // field sparks are cold cyan, the bullet that goes off is hot amber; both add light
  const along = float(1).sub(abs(vCorner.y));
  const body = along.mul(along).mul(mix(0.1, 1, vCorner.x.mul(vCorner.x))).mul(vHeat);
  const tint = mix(v3(u, "uCool"), v3(u, "uWarm"), vWarm) as V3;
  const color = (mix(tint, vec3(1), vHeat.mul(0.35).mul(vCorner.x)) as V3).mul(body).mul(vWarm.mul(2).add(2.4));
  return { vertex, fragment: vec4(color, 0) };
}

// Rust switches a pass's quality by its GLSL defines (TAPS, DOF_TAPS); here a new set rebuilds the node twin
const DEFINES = new WeakMap<object, Record<string, unknown>>();
function defines(material: object): Record<string, unknown> {
  return DEFINES.get(material) ?? {};
}
function onDefines(material: object, build: () => void): void {
  Object.defineProperty(material, "defines", {
    get: () => defines(material),
    set: (value: Record<string, unknown> | undefined) => { DEFINES.set(material, { ...value }); build(); },
    configurable: true,
  });
}

/** Every ShaderMaterial Rust asks for, by name: a node twin, or hidden where there is none yet. */
class ShaderMaterial {
  constructor(fields: Fields = {}) {
    const { vertexShader, fragmentShader, uniforms, defines: initial, ...rest } = fields;
    void vertexShader; void fragmentShader;
    const material = new WEBGPU.NodeMaterial() as Shader;
    Object.assign(material, rest);
    material.lights = false;
    material.fog = false;
    material.uniforms = wrap(uniforms);
    DEFINES.set(material, { ...(initial as Record<string, unknown> | undefined) });
    const quad = vec4(positionGeometry.xy, 0, 1);
    switch (fields.name) {
      case "compose": {
        // the effects Rust switches on and off arrive as feature bits (see refresh_battle_shader)
        for (const [key, value] of Object.entries({
          cdBattleTime: 0, cdHeatDepth: new WEBGPU.Vector4(), cdGlareSun: new WEBGPU.Vector2(0.7, 0.75),
          cdWeather: new WEBGPU.Vector4(), cdWeather2: new WEBGPU.Vector4(), cdWeatherTime: 0, cdWeatherDim: 1,
          cdFogColor: new WEBGPU.Vector3(), cdDustColor: new WEBGPU.Vector3(), cdRainColor: new WEBGPU.Vector3(),
          cdPulse: new WEBGPU.Vector4(0, 0, 0, 1),
        })) material.uniforms[key] = uniform(value as number) as never;
        material.vertexNode = quad;
        let bits = 0;
        const build = () => {
          material.fragmentNode = compose(material.uniforms, bits, Number(defines(material).DOF_TAPS ?? 0));
          material.needsUpdate = true;
        };
        material.setFeatures = (next) => { bits = next; build(); };
        onDefines(material, build);
        build();
        break;
      }
      case "lens": {
        material.vertexNode = quad;
        const build = () => {
          material.fragmentNode = lens(material.uniforms, Number(defines(material).TAPS ?? 0));
          material.needsUpdate = true;
        };
        onDefines(material, build);
        build();
        break;
      }
      case "handMark":
        material.vertexNode = vec4(positionGeometry.xy, 0.9999, 1);
        material.fragmentNode = vec4(0, 0, 0, 0.5);
        break;
      case "stage":
        material.fragmentNode = stage(material.uniforms);
        break;
      case "stageDepth":
        material.fragmentNode = vec4(vec3(clamp(float(0.8).div(max(positionView.z.negate(), 0.8) as F), 0, 1) as F), 1);
        break;
      default: {
        // the shield's dome and sparks have no name; their uniforms tell them apart
        const twin = "uHit" in material.uniforms ? shieldDome(material.uniforms)
          : "uOrigin" in material.uniforms && "uCool" in material.uniforms ? shieldSparks(material.uniforms) : null;
        if (twin) {
          material.vertexNode = twin.vertex;
          material.fragmentNode = twin.fragment;
          break;
        }
        // haze, light shafts and dust (none on minimal and low): not on WebGPU yet
        material.visible = false;
      }
    }
    return material;
  }
}

/** three's WebGPU renderer with the WebGL renderer's few extras the Rust side asks for. */
class WebGLRenderer extends WEBGPU.WebGPURenderer {
  readonly extensions = { has: () => true };
  readonly capabilities = { maxTextureSize: 8192, getMaxAnisotropy: () => 16 };
  private probe: WebGL2RenderingContext | null | undefined;
  readonly three = THREE;
  constructor(parameters: ConstructorParameters<typeof WEBGPU.WebGPURenderer>[0]) {
    // ?perf=1: GPU time per frame from timestamp queries, where the adapter has them (fps_meter.rs reads it)
    super({ ...parameters, trackTimestamp: new URLSearchParams(location.search).get("perf") === "1" });
    // Rust's lens writes finished, display-ready colour; the renderer adds nothing on the way out
    for (const [key, value] of [["outputColorSpace", WEBGPU.LinearSRGBColorSpace], ["toneMapping", WEBGPU.NoToneMapping]] as const) {
      Object.defineProperty(this, key, { get: () => value, set: () => {}, configurable: true });
    }
    void this.init();
  }
  // the quality guess asks a WebGL context for the card's name
  override getContext(): never {
    this.probe ??= document.createElement("canvas").getContext("webgl2");
    return this.probe as never;
  }
  // frames asked for before the device is up are skipped
  override render(scene: WEBGPU.Object3D, camera: WEBGPU.Camera): void {
    if (this.initialized) super.render(scene, camera);
  }
  override clear(color?: boolean, depth?: boolean, stencil?: boolean): void {
    if (this.initialized) super.clear(color, depth, stencil);
  }
}

/** WebGPU prefilters an environment map itself, the first time a material reads it. */
class PMREMGenerator {
  // a copy, since the caller disposes the map it passed in
  fromEquirectangular(map: WEBGPU.Texture): { texture: WEBGPU.Texture } { return { texture: map.clone() }; }
  dispose(): void {}
}

export const THREE = { ...WEBGPU, WebGLRenderer, ShaderMaterial, PMREMGenerator };

/**
 * Materials are tuned as on WebGL; the "wide" extra, which pulls my first-person body and gun in
 * toward the sight line by `wideK` around the height `wideY`, becomes a vertex node, since WebGPU
 * never calls onBeforeCompile.
 */
export function tune(material: WEBGPU.Material, quality: Quality, extra?: (shader: never) => void, extraKey = "x"): void {
  webglTune(material as never, quality, extra as never, extraKey);
  if (extraKey !== "wide" || !extra) return;
  // the extra hands over Rust's uniform objects; run it on a stand-in shader to get them
  const shader = { uniforms: {} as Record<string, { value: number }>, vertexShader: "", fragmentShader: "" };
  extra(shader as never);
  const { wideK, wideY } = shader.uniforms;
  if (!wideK || !wideY) return;
  const k = uniform(wideK.value).onRenderUpdate(() => wideK.value);
  const y = uniform(wideY.value).onRenderUpdate(() => wideY.value);
  const clip = modelViewProjection as unknown as V4;
  const xy = mix(vec2(0, y).mul(clip.w), clip.xy, k) as V2;
  (material as WEBGPU.Material & { vertexNode: V4 }).vertexNode = vec4(xy, clip.z, clip.w);
}

/** Dust is made by the arena itself, with no renderer to bring the THREE above. */
export class Dust extends WebGLDust {
  constructor() { super(THREE); }
}
