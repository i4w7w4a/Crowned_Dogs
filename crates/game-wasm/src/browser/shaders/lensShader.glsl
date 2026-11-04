
uniform sampler2D tComp;
uniform sampler2D tRays;
uniform vec2 compSize;
uniform vec2 outSize;
uniform float ssaa;
uniform float fsr; // 1: the screen is larger than the frame, which is enlarged by FSR1 EASU
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

float ign(vec2 p) { return fract(52.9829189 * fract(dot(p, vec2(0.06711056, 0.00583715)))); }

// red through green to blue as t goes 0..1: each tap of the smear lands in its own colour
vec3 spectrum(float t) {
  float w = clamp(1.0 - abs(2.0 * clamp((t - 1.0 / 6.0) / (4.0 / 6.0), 0.0, 1.0) - 1.0), 0.0, 1.0);
  float lo = step(t, 0.5);
  return vec3(lo, 1.0, 1.0 - lo) * vec3(1.0 - w, w, 1.0 - w);
}

// AMD FidelityFX Super Resolution 1.0, EASU (edge-adaptive spatial upsampling), after ffx_fsr1.h
// (MIT, Copyright (c) 2021 Advanced Micro Devices): 12 texels around the point, a Lanczos-like
// kernel stretched along the local edge, clamped to the 4 nearest texels so edges do not ring.
vec3 easuTexel(ivec2 p) { return texelFetch(tComp, clamp(p, ivec2(0), ivec2(compSize) - 1), 0).rgb; }
float easuLuma(vec3 c) { return c.g + 0.5 * (c.r + c.b); }
void easuSet(inout vec2 dir, inout float len, float w, float lA, float lB, float lC, float lD, float lE) {
  float lenX = max(abs(lD - lC), abs(lC - lB));
  float dirX = lD - lB;
  dir.x += dirX * w;
  lenX = clamp(abs(dirX) / max(lenX, 1e-5), 0.0, 1.0);
  len += lenX * lenX * w;
  float lenY = max(abs(lE - lC), abs(lC - lA));
  float dirY = lE - lA;
  dir.y += dirY * w;
  lenY = clamp(abs(dirY) / max(lenY, 1e-5), 0.0, 1.0);
  len += lenY * lenY * w;
}
void easuTap(inout vec3 aC, inout float aW, vec2 off, vec2 dir, vec2 len2, float lob, float clp, vec3 c) {
  vec2 v = vec2(dot(off, dir), dot(off, vec2(-dir.y, dir.x))) * len2;
  float d2 = min(dot(v, v), clp);
  float wB = 0.4 * d2 - 1.0;
  float wA = lob * d2 - 1.0;
  wB *= wB;
  wA *= wA;
  wB = 1.5625 * wB - 0.5625;
  float w = wB * wA;
  aW += w;
  aC += w * c;
}
vec3 easu(vec2 uv) {
  vec2 pp = uv * compSize - 0.5;
  vec2 fp = floor(pp);
  pp -= fp;
  ivec2 o = ivec2(fp);
  //    b c
  //  e f g h
  //  i j k l
  //    n o
  vec3 b = easuTexel(o + ivec2(0, -1)), c = easuTexel(o + ivec2(1, -1));
  vec3 e = easuTexel(o + ivec2(-1, 0)), f = easuTexel(o), g = easuTexel(o + ivec2(1, 0)), h = easuTexel(o + ivec2(2, 0));
  vec3 i = easuTexel(o + ivec2(-1, 1)), j = easuTexel(o + ivec2(0, 1)), k = easuTexel(o + ivec2(1, 1)), l = easuTexel(o + ivec2(2, 1));
  vec3 n = easuTexel(o + ivec2(0, 2)), oo = easuTexel(o + ivec2(1, 2));
  float bL = easuLuma(b), cL = easuLuma(c), eL = easuLuma(e), fL = easuLuma(f), gL = easuLuma(g), hL = easuLuma(h);
  float iL = easuLuma(i), jL = easuLuma(j), kL = easuLuma(k), lL = easuLuma(l), nL = easuLuma(n), oL = easuLuma(oo);
  vec2 dir = vec2(0.0);
  float len = 0.0;
  easuSet(dir, len, (1.0 - pp.x) * (1.0 - pp.y), bL, eL, fL, gL, jL);
  easuSet(dir, len, pp.x * (1.0 - pp.y), cL, fL, gL, hL, kL);
  easuSet(dir, len, (1.0 - pp.x) * pp.y, fL, iL, jL, kL, nL);
  easuSet(dir, len, pp.x * pp.y, gL, jL, kL, lL, oL);
  float dirR = dot(dir, dir);
  bool zro = dirR < 1.0 / 32768.0;
  dir = zro ? vec2(1.0, 0.0) : dir * inversesqrt(dirR);
  len = 0.5 * len;
  len *= len;
  float stretch = dot(dir, dir) / max(abs(dir.x), abs(dir.y));
  vec2 len2 = vec2(1.0 + (stretch - 1.0) * len, 1.0 - 0.5 * len);
  float lob = 0.5 + (0.21 - 0.5) * len;
  float clp = 1.0 / lob;
  vec3 aC = vec3(0.0);
  float aW = 0.0;
  easuTap(aC, aW, vec2(0.0, -1.0) - pp, dir, len2, lob, clp, b);
  easuTap(aC, aW, vec2(1.0, -1.0) - pp, dir, len2, lob, clp, c);
  easuTap(aC, aW, vec2(-1.0, 1.0) - pp, dir, len2, lob, clp, i);
  easuTap(aC, aW, vec2(0.0, 1.0) - pp, dir, len2, lob, clp, j);
  easuTap(aC, aW, vec2(0.0, 0.0) - pp, dir, len2, lob, clp, f);
  easuTap(aC, aW, vec2(-1.0, 0.0) - pp, dir, len2, lob, clp, e);
  easuTap(aC, aW, vec2(1.0, 1.0) - pp, dir, len2, lob, clp, k);
  easuTap(aC, aW, vec2(2.0, 1.0) - pp, dir, len2, lob, clp, l);
  easuTap(aC, aW, vec2(2.0, 0.0) - pp, dir, len2, lob, clp, h);
  easuTap(aC, aW, vec2(1.0, 0.0) - pp, dir, len2, lob, clp, g);
  easuTap(aC, aW, vec2(1.0, 2.0) - pp, dir, len2, lob, clp, oo);
  easuTap(aC, aW, vec2(0.0, 2.0) - pp, dir, len2, lob, clp, n);
  vec3 lo = min(min(f, g), min(j, k)), hi = max(max(f, g), max(j, k));
  return clamp(aC / aW, lo, hi);
}

// the frame where the glass is clean: one read, or four when the frame is drawn larger than the screen
vec4 clean(vec2 uv) {
  if (fsr > 0.5) return vec4(easu(uv), texture(tComp, uv).a);
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
#if TAPS < 1
  c = clean(src); // no smear: clean glass to the edges, and no glow, which reads the frame's mips
#else
  if (amount < 0.0004 && r < 0.0004) {
    c = clean(src);
  } else {
    // the taps read a blurred copy of the frame, as blurred as they are far apart, so they flow into one smear
    float span = 2.0 * amount * length((src - center) * outSize) / float(TAPS);
    float lod = log2(max(1.0, max(span * 2.0, r * outSize.y * 1.5)) * ssaa);
    // each pixel starts its taps at its own offset, so a busy pattern near the edge (a camouflage
    // net, sandbags) does not break into rows of copies
    float jitter = ign(gl_FragCoord.xy + 17.0);
    vec3 sum = vec3(0.0), wsum = vec3(0.0);
    float a = 0.0;
    for (int i = 0; i < TAPS; i++) {
      float t = (float(i) + 0.5) / float(TAPS);
      // half white: fine detail smears without splitting into red, green and blue copies
      vec3 w = mix(vec3(1.0), spectrum(t), 0.45);
      t = (float(i) + jitter) / float(TAPS);
      vec4 s = textureLod(tComp, center + (src - center) * (1.0 + amount * (t - 0.5) * 2.0), lod); // red spreads outwards
      sum += s.rgb * w; wsum += w; a += s.a;
    }
    c = vec4(sum / wsum, a / float(TAPS));
  }
#endif
  // the bright parts bleed light over everything near them: two blurred copies, only what is near white
#if TAPS > 0
  vec3 halo = max(textureLod(tComp, src, 4.5 + glowLod).rgb - 0.55, 0.0) + max(textureLod(tComp, src, 6.0 + glowLod).rgb - 0.5, 0.0);
  c.rgb += halo * glow * hasBg;
#endif
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
}