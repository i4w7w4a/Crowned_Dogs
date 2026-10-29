// The shaders of the bodycam lens (lens.ts), pass by pass: the frame put together, the
// haze in the air, the light shafts, and the glass itself. GLSL ES 3.0 (three.js adds the version line).

export const vertexShader = /* glsl */ `
out vec2 vUv;
void main() { vUv = uv; gl_Position = vec4(position.xy, 0.0, 1.0); }`;

/** A fixed per-pixel number in 0..1 that changes fast from pixel to pixel: no flicker, no visible pattern. */
const ign = /* glsl */ `
float ign(vec2 p) { return fract(52.9829189 * fract(dot(p, vec2(0.06711056, 0.00583715)))); }`;

// ---------- 1. the frame: painting, haze, enemy, our own figure ----------

export const composeShader = /* glsl */ `
uniform sampler2D tFig;
uniform sampler2D tNear;
uniform sampler2D tBg;
uniform sampler2D tHaze;
uniform sampler2D tDepth;
uniform float hasDepth;
uniform float focusInv;
uniform float dof;
uniform float dofNear;
uniform vec2 bgSize;
uniform vec2 nearSize;
uniform vec2 hazeSize;
uniform float hasBg;
uniform float exposure;
uniform float saturation;
uniform vec3 tint;
in vec2 vUv;

// three.js ACES filmic, so the figures keep the tone they had when drawn straight to the screen
vec3 RRTAndODTFit(vec3 v) {
  vec3 a = v * (v + 0.0245786) - 0.000090537;
  vec3 b = v * (0.983729 * v + 0.4329510) + 0.238081;
  return a / b;
}
vec3 aces(vec3 color) {
  const mat3 ACESInputMat = mat3(vec3(0.59719, 0.07600, 0.02840), vec3(0.35458, 0.90834, 0.13383), vec3(0.04823, 0.01566, 0.83777));
  const mat3 ACESOutputMat = mat3(vec3(1.60475, -0.10208, -0.00327), vec3(-0.53108, 1.10813, -0.07276), vec3(-0.07367, -0.00605, 1.07602));
  color *= exposure / 0.6;
  color = ACESInputMat * color;
  color = RRTAndODTFit(color);
  color = ACESOutputMat * color;
  return clamp(color, 0.0, 1.0);
}
vec3 toSRGB(vec3 c) {
  return mix(c * 12.92, 1.055 * pow(c, vec3(1.0 / 2.4)) - 0.055, step(vec3(0.0031308), c));
}

// a drawn layer (linear, premultiplied) toned and graded like the painting, still premultiplied
vec3 tone(vec4 f) {
  vec3 lin = f.a > 0.001 ? f.rgb / f.a : f.rgb;
  vec3 c = toSRGB(aces(lin));
  c = mix(vec3(dot(c, vec3(0.2126, 0.7152, 0.0722))), c, saturation) * tint;
  return f.a > 0.001 ? c * f.a : c; // sparks and dust add light without covering
}

// the painting: mipmapped where the screen is smaller than it, a sharp Catmull-Rom filter where it is larger
vec3 painting(vec2 uv) {
#ifdef SHARP_BG
  vec2 fw = max(abs(dFdx(uv * bgSize)), abs(dFdy(uv * bgSize)));
  if (max(fw.x, fw.y) < 0.9) {
    vec2 st = uv * bgSize - 0.5, i = floor(st) + 0.5, f = st - floor(st);
    vec2 w0 = f * (-0.5 + f * (1.0 - 0.5 * f));
    vec2 w1 = 1.0 + f * f * (-2.5 + 1.5 * f);
    vec2 w2 = f * (0.5 + f * (2.0 - 1.5 * f));
    vec2 w3 = f * f * (-0.5 + 0.5 * f);
    vec2 w12 = w1 + w2, o12 = w2 / w12;
    vec2 t0 = (i - 1.0) / bgSize, t3 = (i + 2.0) / bgSize, t12 = (i + o12) / bgSize;
    vec3 c = textureLod(tBg, vec2(t0.x, t0.y), 0.0).rgb * w0.x * w0.y + textureLod(tBg, vec2(t12.x, t0.y), 0.0).rgb * w12.x * w0.y + textureLod(tBg, vec2(t3.x, t0.y), 0.0).rgb * w3.x * w0.y
           + textureLod(tBg, vec2(t0.x, t12.y), 0.0).rgb * w0.x * w12.y + textureLod(tBg, vec2(t12.x, t12.y), 0.0).rgb * w12.x * w12.y + textureLod(tBg, vec2(t3.x, t12.y), 0.0).rgb * w3.x * w12.y
           + textureLod(tBg, vec2(t0.x, t3.y), 0.0).rgb * w0.x * w3.y + textureLod(tBg, vec2(t12.x, t3.y), 0.0).rgb * w12.x * w3.y + textureLod(tBg, vec2(t3.x, t3.y), 0.0).rgb * w3.x * w3.y;
    return clamp(c, 0.0, 1.0);
  }
#endif
  return texture(tBg, uv).rgb;
}

// Depth of field: the camera is focused on the enemy. The painting's depth map gives each pixel its
// distance, and the blur grows with how far it lies behind him (much less in front of him).
float coc(vec2 uv) {
  float d = focusInv - texture(tDepth, uv).r * 1.25; // the map holds 1/z, 1.0 = 1/0.8 m
  // radius in texels of the painting; held at 0.13 so a close enemy doesn't wash the hall out
  return dof * min(d > 0.0 ? d : -d * dofNear, 0.13) * bgSize.y;
}

vec3 focused(vec2 uv) {
  if (hasDepth < 0.5) return painting(uv);
  float r = coc(uv);
#if DOF_TAPS < 2
  return r < 0.6 ? painting(uv) : textureLod(tBg, uv, log2(r * 1.6)).rgb;
#else
  if (r < 0.6) return painting(uv);
  // a disc of reads, each from a copy blurred just enough to close the gaps between them
  float lod = log2(max(1.0, 2.2 * r / sqrt(float(DOF_TAPS))));
  vec3 sum = textureLod(tBg, uv, lod).rgb;
  float wsum = 1.0;
  for (int i = 0; i < DOF_TAPS; i++) {
    float rr = sqrt((float(i) + 0.5) / float(DOF_TAPS)) * r, a = float(i) * 2.39996;
    vec2 t = uv + vec2(cos(a), sin(a)) * rr / bgSize;
    // something sharper than this pixel (nearer the focus) does not spread over it
    float w = clamp(coc(t) - rr + 1.0, 0.0, 1.0);
    sum += textureLod(tBg, t, lod).rgb * w;
    wsum += w;
  }
  return sum / wsum;
#endif
}

// a smooth B-spline read of a small picture: stretched over the frame it comes back soft, with no steps or blocks
vec4 smoothRead(sampler2D t, vec2 uv, vec2 size) {
  vec2 st = uv * size - 0.5, i = floor(st), f = st - i;
  vec2 f2 = f * f, f3 = f2 * f;
  vec2 w0 = (1.0 - 3.0 * f + 3.0 * f2 - f3) / 6.0;
  vec2 w1 = (4.0 - 6.0 * f2 + 3.0 * f3) / 6.0;
  vec2 w2 = (1.0 + 3.0 * f + 3.0 * f2 - 3.0 * f3) / 6.0;
  vec2 w3 = f3 / 6.0;
  vec2 g0 = w0 + w1, g1 = w2 + w3;
  vec2 h0 = (i - 0.5 + w1 / g0) / size, h1 = (i + 1.5 + w3 / g1) / size;
  return g0.y * (g0.x * texture(t, vec2(h0.x, h0.y)) + g1.x * texture(t, vec2(h1.x, h0.y)))
       + g1.y * (g0.x * texture(t, vec2(h0.x, h1.y)) + g1.x * texture(t, vec2(h1.x, h1.y)));
}

void main() {
  vec4 f = texture(tFig, vUv);
  vec3 bg = hasBg > 0.5 ? focused(vUv) : vec3(0.0);
  vec3 c = tone(f) + bg * (1.0 - f.a);
  float a = hasBg > 0.5 ? 1.0 : f.a;
#ifdef HAZE
  // the haze is drawn small and noisy; four soft reads smooth it out
  vec2 hs = 0.75 / hazeSize;
  vec4 h = 0.25 * (texture(tHaze, vUv + vec2(-hs.x, -hs.y)) + texture(tHaze, vUv + vec2(hs.x, -hs.y)) + texture(tHaze, vUv + vec2(-hs.x, hs.y)) + texture(tHaze, vUv + hs));
  if (hasBg > 0.5) c = c * h.a + h.rgb;
#endif
  vec4 n = smoothRead(tNear, vUv, nearSize);
  c = tone(n) + c * (1.0 - n.a);
  gl_FragColor = vec4(c, max(a, n.a));
}`;

// ---------- 2. haze in the air (high and ultra) ----------

export const hazeShader = /* glsl */ `
precision highp sampler2DShadow;
uniform mat4 invProj;
uniform mat4 camWorld;
uniform vec3 camPos;
uniform sampler2D tFig;
uniform float enemyDist;
uniform sampler2D tDepth;
uniform float hasDepth;
uniform sampler2DShadow tShadow;
uniform mat4 shadowMatrix;
uniform float hasShadow;
uniform vec3 sunDir;
uniform vec3 sunColor;
uniform vec3 skyColor;
uniform float density;
uniform float time;
in vec2 vUv;
${ign}

float hash(vec3 p) { p = fract(p * 0.3183099 + 0.1); p *= 17.0; return fract(p.x * p.y * p.z * (p.x + p.y + p.z)); }
float noise(vec3 x) {
  vec3 i = floor(x), f = fract(x);
  f = f * f * (3.0 - 2.0 * f);
  return mix(mix(mix(hash(i), hash(i + vec3(1, 0, 0)), f.x), mix(hash(i + vec3(0, 1, 0)), hash(i + vec3(1, 1, 0)), f.x), f.y),
             mix(mix(hash(i + vec3(0, 0, 1)), hash(i + vec3(1, 0, 1)), f.x), mix(hash(i + vec3(0, 1, 1)), hash(i + vec3(1, 1, 1)), f.x), f.y), f.z);
}
float clouds(vec3 p) {
  float v = 0.0, a = 0.55;
  for (int o = 0; o < OCTAVES; o++) { v += a * noise(p); p = p * 2.03 + vec3(1.7, 9.2, 4.1); a *= 0.5; }
  return v;
}
// Henyey-Greenstein: haze glows when you look towards the light through it
float phase(float cosT, float g) { float g2 = g * g; return (1.0 - g2) / (12.566 * pow(1.0 + g2 - 2.0 * g * cosT, 1.5)); }

void main() {
  vec4 v = invProj * vec4(vUv * 2.0 - 1.0, 1.0, 1.0);
  vec3 dir = normalize((camWorld * vec4(v.xyz / v.w, 0.0)).xyz);
  float tMax = 24.0;
  if (dir.y < -0.001) tMax = min(tMax, camPos.y / -dir.y); // the painted floor
  if (hasDepth > 0.5) tMax = min(tMax, 1.0 / max(texture(tDepth, vUv).r * 1.25, 0.01) / max(-dir.z, 0.05)); // the painted walls and machines
  if (texture(tFig, vUv).a > 0.5) tMax = min(tMax, enemyDist / max(-dir.z, 0.2)); // the enemy's body
  float cosT = dot(dir, sunDir);
  float lobe = phase(cosT, 0.6) * 3.0 + phase(cosT, -0.2) * 1.5;
  vec3 drift = vec3(0.06, 0.012, 0.035) * time;
  float jit = ign(gl_FragCoord.xy);
  vec3 light = vec3(0.0);
  float T = 1.0;
  // steps bunch up near the lens, where the figures and their shadows are
  for (int i = 0; i < STEPS; i++) {
    float u0 = (float(i) + jit) / float(STEPS), u1 = (float(i) + 1.0 + jit) / float(STEPS);
    float t = tMax * u0 * u0, dt = tMax * (u1 * u1 - u0 * u0);
    vec3 p = camPos + dir * t;
    float d = density * exp(-max(p.y, 0.0) * 0.32) * smoothstep(0.15, 1.0, clouds(p * vec3(0.32, 0.5, 0.32) + drift));
    float vis = 1.0;
    if (hasShadow > 0.5) {
      vec4 sc = shadowMatrix * vec4(p, 1.0);
      sc.xyz /= sc.w;
      if (all(greaterThan(sc.xyz, vec3(0.0))) && all(lessThan(sc.xyz, vec3(1.0)))) vis = texture(tShadow, vec3(sc.xy, sc.z - 0.002));
    }
    light += T * d * dt * (skyColor + sunColor * lobe * vis);
    T *= exp(-d * dt);
  }
  gl_FragColor = vec4(min(light, vec3(0.6)), T);
}`;

// ---------- 3. light shafts from the bright openings of the painting ----------

/** What gives off light: the bright parts of the frame above the horizon, the figures block it. */
export const raysMaskShader = /* glsl */ `
uniform sampler2D tComp;
uniform sampler2D tFig;
uniform float lod;
uniform float horizon;
uniform float threshold;
in vec2 vUv;
void main() {
  vec3 c = textureLod(tComp, vUv, lod).rgb;
  float l = dot(c, vec3(0.2126, 0.7152, 0.0722));
  float sky = smoothstep(horizon - 0.1, horizon + 0.14, vUv.y); // vUv.y runs up from the bottom
  float m = smoothstep(threshold, 1.0, l) * sky * (1.0 - texture(tFig, vUv).a);
  gl_FragColor = vec4(c * m, 1.0);
}`;

/** Light streaming from the opening towards each pixel; run twice, the second time with fine steps to fill the gaps. */
export const raysBlurShader = /* glsl */ `
uniform sampler2D tSrc;
uniform vec2 light;
uniform float reach;
uniform float decay;
in vec2 vUv;
void main() {
  vec2 delta = (vUv - light) * reach / float(STEPS);
  vec2 p = vUv;
  vec3 sum = vec3(0.0);
  float w = 1.0, wsum = 0.0;
  for (int i = 0; i < STEPS; i++) {
    sum += texture(tSrc, p).rgb * w;
    wsum += w;
    w *= decay;
    p -= delta;
  }
  gl_FragColor = vec4(sum / wsum, 1.0);
}`;

// ---------- 4. the glass ----------

export const lensShader = /* glsl */ `
uniform sampler2D tComp;
uniform sampler2D tRays;
uniform vec2 compSize;
uniform vec2 outSize;
uniform float ssaa;
uniform float hasBg;
uniform float rays;
uniform float glowLod;
uniform vec2 center;
uniform vec2 halfView;
uniform float aspect;
uniform float smear;
uniform float soft;
uniform float barrel;
uniform float vignette;
uniform float glow;
uniform float contrast;
in vec2 vUv;
${ign}

// red through green to blue as t goes 0..1: each tap of the smear lands in its own colour
vec3 spectrum(float t) {
  float w = clamp(1.0 - abs(2.0 * clamp((t - 1.0 / 6.0) / (4.0 / 6.0), 0.0, 1.0) - 1.0), 0.0, 1.0);
  float lo = step(t, 0.5);
  return vec3(lo, 1.0, 1.0 - lo) * vec3(1.0 - w, w, 1.0 - w);
}

// the frame where the glass is clean: one read, or four when the frame is drawn larger than the screen
vec4 clean(vec2 uv) {
  if (ssaa <= 1.0) return texture(tComp, uv);
  vec2 o = 0.25 / outSize;
  return 0.25 * (texture(tComp, uv + vec2(-o.x, -o.y * 0.5)) + texture(tComp, uv + vec2(o.x * 0.5, -o.y))
               + texture(tComp, uv + vec2(o.x, o.y * 0.5)) + texture(tComp, uv + vec2(-o.x * 0.5, o.y)));
}

void main() {
  vec2 d = (vUv - center) / halfView; // -1..1 across the visible part
  vec2 e = d * vec2(aspect, 1.0) / length(vec2(aspect, 1.0));
  vec2 src = center + (vUv - center) * (1.0 + barrel * dot(e, e)); // the bend, round like the glass
  float q = 0.5 * dot(d, d); // the frame's own oval: 0.5 on the middle of each edge, 1 in the corners
  float amount = smear * smoothstep(0.08, 1.0, q);
  float r = soft * smoothstep(0.15, 1.0, q);
  vec4 c;
  if (amount < 0.0004 && r < 0.0004) {
    c = clean(src);
  } else {
    // the taps read a blurred copy of the frame, as blurred as they are far apart, so they flow into one smear
    float span = 2.0 * amount * length((src - center) * outSize) / float(TAPS);
    float lod = log2(max(1.0, max(span, r * outSize.y * 1.5)) * ssaa);
    vec3 sum = vec3(0.0), wsum = vec3(0.0);
    float a = 0.0;
    for (int i = 0; i < TAPS; i++) {
      float t = (float(i) + 0.5) / float(TAPS);
      vec3 w = spectrum(t);
      vec4 s = textureLod(tComp, center + (src - center) * (1.0 + amount * (t - 0.5) * 2.0), lod); // red spreads outwards
      sum += s.rgb * w; wsum += w; a += s.a;
    }
    c = vec4(sum / wsum, a / float(TAPS));
  }
  // the bright parts bleed light over everything near them: two blurred copies, only what is near white
  vec3 halo = max(textureLod(tComp, src, 4.5 + glowLod).rgb - 0.55, 0.0) + max(textureLod(tComp, src, 6.0 + glowLod).rgb - 0.5, 0.0);
  c.rgb += halo * glow * hasBg;
#ifdef RAYS
  vec3 ray = texture(tRays, src).rgb * rays * hasBg;
  c.rgb = 1.0 - (1.0 - c.rgb) * (1.0 - clamp(ray, 0.0, 1.0));
#endif
  c.rgb = clamp(c.rgb, 0.0, 1.0);
  c.rgb = mix(c.rgb, c.rgb * c.rgb * (3.0 - 2.0 * c.rgb), contrast * hasBg);
  float v = smoothstep(0.45, 1.0, q);
  float dark = 1.0 - vignette * v * sqrt(v); // bright almost to the edge, then sinking fast into black corners
  // half a step of 8-bit colour, different in every pixel: smooth dark gradients with no bands
  vec3 dither = vec3((ign(gl_FragCoord.xy) - 0.5) / 255.0);
  gl_FragColor = vec4(clamp(c.rgb * dark + dither, 0.0, 1.0), c.a);
}`;
