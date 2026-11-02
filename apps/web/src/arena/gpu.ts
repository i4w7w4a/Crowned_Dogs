// The arena on WebGPU (?gpu=1): the same Rust render graph, with three's WebGPU renderer.
// Rust makes its materials as ShaderMaterials with GLSL; WebGPU draws only node materials, so
// `ShaderMaterial` here builds a TSL twin of each named one. The weather is the minimal quality's
// (flat veils, one rain layer, big drops on the glass) at every quality. Not ported yet, and
// hidden for now: the haze, the light shafts, dust, the shield, depth of field, the lens smear
// and glow and the heat shimmer. Figure details made with onBeforeCompile are skipped by WebGPU,
// all but the first-person squeeze toward the sight line (see `tune`).
import * as WEBGPU from "three/webgpu";
import type { Node } from "three/webgpu";
import {
  abs, clamp, dot, exp, float, floor, fract, int, length, log2, max, min, mix, modelViewProjection, normalize, positionGeometry,
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

/** The compose pass: fighters over the hall, toned like the painting, the gun and hands on top. */
function compose(u: Shader["uniforms"], features: number): V4 {
  const minimal = (features & 32) !== 0, nvg = (features & 1) !== 0, glare = (features & 4) !== 0;
  const weather = (features & 8) !== 0;
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
  const bg = select(hasBg.greaterThan(0.5), read(sampler(u, "tBg"), at).rgb, vec3(0)) as V3;
  let c = tone(f).add(bg.mul(float(1).sub(f.a)));
  const a = select(hasBg.greaterThan(0.5), float(1), f.a) as F;
  const c0 = c;
  if (weather) c = air(u, c, f, at);
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
  if (weather) c = onLens(u, c, n, at);
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

/** Weather in the air between the lens and the painting, as weather.glsl at minimal quality. */
function air(u: Shader["uniforms"], c: V3, f: V4, at: V2): V3 {
  const hasDepth = num(u, "hasDepth"), bgSize = v2(u, "bgSize");
  const w = v4(u, "cdWeather"), w2 = v4(u, "cdWeather2"), t = num(u, "cdWeatherTime");
  // the map holds 1/z; read soft, so a projected stage's proxy boxes leave no outlines in the air
  const depth = sampler(u, "tDepth");
  const rows = vec2(depth.size(int(0)) as unknown as Node<"ivec2">).y;
  const soft = readLevel(depth, at, log2(max(rows.mul(0.05), 1) as F)).r.mul(1.25);
  let inv = select(hasDepth.greaterThan(0.5), soft, float(0.06)) as F;
  inv = mix(inv, num(u, "focusInv"), step(0.5, f.a)) as F; // the enemy stands at his own depth
  const z = min(float(1).div(max(inv, 0.016) as F), 60) as F; // the open sky counts as far away
  const far = float(1).sub(exp(z.mul(-0.075)));
  const p = vec2(at.x.mul(bgSize.x).div(max(bgSize.y, 1) as F), at.y);
  let k = c.mul(num(u, "cdWeatherDim"));
  // fog: one even layer, thicker low down
  const fog = w.x.mul(float(1.15).sub(at.y.mul(0.35))).mul(float(1).sub(exp(z.mul(-0.075))));
  k = mix(k, v3(u, "cdFogColor"), clamp(fog, 0, 0.93) as F) as V3;
  // salt dust: an even veil that breathes with the gusts, nearly closing the view in a surge
  const gust = float(0.7).add(sin(t.mul(0.6).add(sin(t.mul(0.21)).mul(3))).mul(0.3));
  const surge = w2.x;
  const dust = w.y.mul(gust).mul(float(1.2).sub(at.y.mul(0.4))).mul(far.mul(0.8).add(0.2)).mul(0.12 + 1.1 * 0.3)
    .add(surge.mul(0.8 + 0.25 * 0.5 + 0.1 * 0.3));
  const dusty = mix(k, v3(u, "cdDustColor").mul(0.82 + 0.18 * 0.3), clamp(dust, 0, surge.mul(0.09).add(0.88)) as F) as V3;
  k = select(w.y.greaterThan(0), dusty, k) as V3;
  // rain: thin slanted streaks, hidden behind what stands nearer
  const q = vec2(p.x.add(at.y.mul(0.07)).mul(70), at.y.mul(70 * 0.09).add(t.mul(2.6)));
  const h = wxHash(floor(q) as V2);
  const x = fract(q.x).sub(0.5).add(h.sub(0.5).mul(0.5));
  const y = fract(q.y.add(h.mul(7)));
  const streak = (smoothstep(0.09, 0, abs(x)) as F).mul(smoothstep(0, 0.3, y)).mul(smoothstep(1, 0.6, y)).mul(step(0.62, h));
  const rain = streak.mul(step(2.5, z)).mul(0.5);
  k = mix(k, v3(u, "cdRainColor"), clamp(rain.mul(w.z), 0, 0.55) as F) as V3;
  // wet ground: the soaked floor only darkens
  const below = w2.w.sub(at.y);
  const ground = float(1.6).div(max(below, 0.004) as F); // metres, roughly
  const onFloor = select(hasDepth.greaterThan(0.5), smoothstep(0.55, 0.8, z.div(ground)), float(1)) as F;
  const wet = w2.z.mul(onFloor).mul(smoothstep(40, 6, ground));
  const soaked = w2.z.greaterThan(0).and(below.greaterThan(0.004)).and(f.a.lessThan(0.5));
  k = select(soaked, k.mul(float(1).sub(wet.mul(0.12))), k) as V3;
  // lightning: the far, open parts flare first
  const lit = w.w.mul(far.mul(0.7).add(0.3));
  k = k.mul(lit.mul(1.5).add(1)).add(vec3(0.78, 0.84, 0.92).mul(lit.mul(0.1)));
  return select(num(u, "hasBg").greaterThan(0.5), k, c) as V3;
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

/** What sits on the lens itself, over the gun and the hands too, as weatherLens.glsl at minimal quality. */
function onLens(u: Shader["uniforms"], c: V3, n: V4, at: V2): V3 {
  const w = v4(u, "cdWeather"), w2 = v4(u, "cdWeather2"), t = num(u, "cdWeatherTime");
  const nearSize = v2(u, "nearSize");
  const aspect = nearSize.x.div(max(nearSize.y, 1) as F);
  const q = vec2(at.x.mul(aspect), at.y);
  // dust: a gust thickens until it swallows even the gun
  const dusty = mix(c, v3(u, "cdDustColor").mul(0.94), clamp(w2.x.mul(0.425).mul(n.a), 0, 0.6) as F) as V3;
  let k = select(w2.y.greaterThan(0), dusty, c) as V3;
  // rain: big drops on the glass bend the view behind them; the gun and the hands are wet
  const wet = w2.z;
  const drop = drops(q, 4.5, t, 1, wet.mul(0.4));
  const damp = k.mul(float(1).sub(wet.mul(0.14).mul(n.a)));
  const s = drop.xy.div(max(drop.z, 0.001) as F);
  const l2 = min(dot(s, s), 1) as F;
  const ruv = clamp(at.sub(s.mul(vec2(float(0.03).div(aspect), 0.03))), vec2(0.001), vec2(0.999)) as V2;
  const behind = readLevel(sampler(u, "tBg"), ruv, 3.2).rgb.mul(num(u, "cdWeatherDim"));
  const seen = mix(select(num(u, "hasBg").greaterThan(0.5), behind, damp) as V3, damp.mul(0.95), n.a) as V3;
  const normal = vec3(s, sqrt(float(1).sub(l2)));
  const glint = pow(max(dot(normal, normalize(vec3(-0.45, 0.6, 0.66))), 0) as F, 28);
  const inside = seen.mul(float(1).sub((smoothstep(0.4, 1, l2) as F).mul(0.3)))
    .add(vec3(0.9, 0.94, 1).mul(glint.mul(dot(seen, vec3(0.33)).mul(0.6).add(0.1).add(w.w))));
  const through = select(drop.z.greaterThan(0.001), mix(damp, inside, drop.z.mul(wet.mul(0.3).add(0.6))), damp) as V3;
  k = select(wet.greaterThan(0), through, k) as V3;
  return k;
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

/** The lens pass to the screen: the bend, contrast, vignette and dither (no smear, glow or rays yet). */
function lens(u: Shader["uniforms"]): V4 {
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
  const c = select(num(u, "ssaa").lessThanEqual(1), read(comp, src), four) as V4;
  let rgb = clamp(c.rgb, 0, 1) as V3;
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

/** Every ShaderMaterial Rust asks for, by name: a node twin, or hidden where there is none yet. */
class ShaderMaterial {
  constructor(fields: Fields = {}) {
    const { vertexShader, fragmentShader, uniforms, defines, ...rest } = fields;
    void vertexShader; void fragmentShader; void defines;
    const material = new WEBGPU.NodeMaterial() as Shader;
    Object.assign(material, rest);
    material.lights = false;
    material.fog = false;
    material.uniforms = wrap(uniforms);
    const quad = vec4(positionGeometry.xy, 0, 1);
    switch (fields.name) {
      case "compose": {
        // the effects Rust switches on and off arrive as feature bits (see refresh_battle_shader)
        for (const [key, value] of Object.entries({
          cdBattleTime: 0, cdHeatDepth: new WEBGPU.Vector4(), cdGlareSun: new WEBGPU.Vector2(0.7, 0.75),
          cdWeather: new WEBGPU.Vector4(), cdWeather2: new WEBGPU.Vector4(), cdWeatherTime: 0, cdWeatherDim: 1,
          cdFogColor: new WEBGPU.Vector3(), cdDustColor: new WEBGPU.Vector3(), cdRainColor: new WEBGPU.Vector3(),
        })) material.uniforms[key] = uniform(value as number) as never;
        material.vertexNode = quad;
        material.setFeatures = (bits) => {
          material.fragmentNode = compose(material.uniforms, bits);
          material.needsUpdate = true;
        };
        material.setFeatures(0);
        break;
      }
      case "lens":
        material.vertexNode = quad;
        material.fragmentNode = lens(material.uniforms);
        break;
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
      default:
        // haze, light shafts, dust, the shield: not on WebGPU yet
        material.visible = false;
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
    super(parameters);
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
