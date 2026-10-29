
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

float ign(vec2 p) { return fract(52.9829189 * fract(dot(p, vec2(0.06711056, 0.00583715)))); }

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
}