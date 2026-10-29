
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
}