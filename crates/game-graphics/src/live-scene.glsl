// A still base photograph lives from the medium quality up (game-wasm live.rs): lamps hum and now and
// then dip, screens refresh, radio lights blink, bubbles rise in the water column, steam leaves a
// mug, drops ring the wet floor, the net stirs in the wind, water runs down the dam and clouds pass
// over the sun. Spots are in the picture's own coordinates, u from the left and v from the top.
uniform float uTime;
uniform float uLive;
uniform float uAspect;
uniform vec4 uLamps[8];   // u, v, reach (picture heights), how deep it dips
uniform vec4 uScreens[3]; // u0, v0, u1, v1
uniform vec4 uLeds[6];    // u, v, size, blink period (s)
uniform vec4 uSteam[2];   // u, v of the source, width, height
uniform vec4 uDrips[4];   // u, v, period (s), phase
uniform vec4 uTube;       // the glowing water column
uniform vec4 uWet;        // floor that trembles and takes the drips
uniform vec4 uNet;        // cloth that hangs from its top edge and stirs
uniform vec4 uFlow;       // water falling down
uniform vec4 uWindow;     // the world outside: cloud shadows pass over it
uniform vec4 uSun;        // sunlit patches inside, dimmed by the same clouds
uniform vec4 uMist;       // depth below which mist drifts, its strength

float cdHash(float n) { return fract(sin(n) * 43758.5453); }
float cdHash2(vec2 p) { return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453); }
float cdNoise(vec2 p) {
  vec2 i = floor(p);
  vec2 f = fract(p);
  f = f * f * (3.0 - 2.0 * f);
  return mix(mix(cdHash2(i), cdHash2(i + vec2(1.0, 0.0)), f.x),
             mix(cdHash2(i + vec2(0.0, 1.0)), cdHash2(i + vec2(1.0, 1.0)), f.x), f.y);
}
float cdNoise1(float x) {
  float i = floor(x);
  float f = fract(x);
  return mix(cdHash(i), cdHash(i + 1.0), f * f * (3.0 - 2.0 * f));
}
float cdFbm(vec2 p) {
  return 0.5 * cdNoise(p) + 0.3 * cdNoise(p * 2.03 + 7.1) + 0.2 * cdNoise(p * 4.07 + 3.3);
}
float cdBox(vec2 p, vec4 r, float soft) {
  if (r.z <= r.x) return 0.0;
  vec2 a = smoothstep(r.xy - soft, r.xy + soft, p);
  vec2 b = smoothstep(r.zw + soft, r.zw - soft, p);
  return a.x * a.y * b.x * b.y;
}
// The clouds over the sun, shared by the window and the sunlit floor: mostly clear, now and then
// a long shadow.
float cdCloud(float t) {
  return smoothstep(0.42, 0.72, cdNoise1(t * 0.06 + 3.0));
}

// Where the picture is sampled (offsets in picture coordinates).
vec2 cdLiveWarp(vec2 p, float t) {
  vec2 d = vec2(0.0);
  float wet = cdBox(p, uWet, 0.03);
  if (wet > 0.0) {
    d += wet * 0.0004 * vec2(sin(p.y * 820.0 + t * 2.1 + 3.0 * sin(p.x * 130.0)),
                             sin(p.x * 640.0 - t * 1.6));
    for (int i = 0; i < 4; i++) {
      vec4 drop = uDrips[i];
      if (drop.z <= 0.0) continue;
      float age = fract(t / drop.z + drop.w) * drop.z;
      // The floor recedes: a ring lies as a flat ellipse.
      vec2 q = (p - drop.xy) * vec2(uAspect, 3.4);
      float r = length(q);
      float ring = exp(-pow((r - age * 0.03) / 0.0035, 2.0)) * exp(-age * 1.4);
      d += wet * ring * 0.0022 * (q / max(r, 1e-4)) * vec2(1.0, 0.3);
    }
  }
  float net = cdBox(p, uNet, 0.006);
  if (net > 0.0) {
    float hang = clamp((p.y - uNet.y) / (uNet.w - uNet.y), 0.0, 1.0);
    float gust = 0.55 + 0.45 * cdNoise1(t * 0.35);
    d.x += net * hang * gust * 0.0032 * sin(t * 1.7 + p.x * 34.0);
    d.y += net * hang * gust * 0.0012 * sin(t * 2.2 + p.x * 47.0);
  }
  float flow = cdBox(p, uFlow, 0.004);
  if (flow > 0.0) {
    d.y += flow * (cdNoise(vec2(p.x * 300.0, p.y * 45.0 - t * 2.4)) - 0.5) * 0.003;
  }
  return d;
}

// The light in the picture at `p`; `depth` is the depth map there (bright is near).
vec3 cdLiveLight(vec3 col, vec2 p, float depth, float t) {
  vec2 aspect = vec2(uAspect, 1.0);
  float luma = dot(col, vec3(0.299, 0.587, 0.114));
  float lit = smoothstep(0.05, 0.6, luma);
  for (int i = 0; i < 8; i++) {
    vec4 lamp = uLamps[i];
    if (lamp.z <= 0.0) continue;
    float seed = float(i) * 7.31 + 1.7;
    float d = length((p - lamp.xy) * aspect) / lamp.z;
    float reach = exp(-d * d * 2.0);
    float hum = 0.02 * sin(t * 7.0 + seed) * cdNoise1(t * 0.8 + seed);
    float breathe = 0.05 * (cdNoise1(t * 0.4 + seed * 3.0) - 0.5);
    // A failing tube: rare stutters, deeper on some lamps than others.
    float trouble = smoothstep(0.8, 0.95, cdNoise1(t * 1.3 + seed * 11.0));
    float stutter = trouble * step(0.45, cdHash(floor(t * 14.0) + seed));
    float level = 1.0 + hum + breathe - lamp.w * stutter;
    col *= mix(1.0, level, reach * (0.3 + 0.7 * lit));
  }
  for (int i = 0; i < 3; i++) {
    vec4 r = uScreens[i];
    float m = cdBox(p, r, 0.003) * smoothstep(0.06, 0.32, luma);
    if (m <= 0.0) continue;
    float seed = float(i) * 3.7;
    vec2 q = (p - r.xy) / (r.zw - r.xy);
    float band = exp(-pow((q.y - fract(t * 0.21 + seed)) * 10.0, 2.0));
    // Panels of data refresh in turn.
    float cell = cdHash2(floor(q * vec2(5.0, 4.0)) + floor(t * 1.1 + seed) * 13.0);
    float level = 1.0 + 0.14 * band + 0.03 * sin(t * 29.0 + seed) + 0.14 * (cell - 0.5);
    col *= mix(1.0, level, m);
    col += m * band * 0.025 * vec3(0.8, 0.92, 1.0);
  }
  for (int i = 0; i < 6; i++) {
    vec4 led = uLeds[i];
    if (led.z <= 0.0) continue;
    float d = length((p - led.xy) * aspect) / led.z;
    float m = exp(-d * d * 2.5);
    float phase = fract(t / led.w + float(i) * 0.37);
    float on = smoothstep(0.0, 0.02, phase) * smoothstep(0.16, 0.12, phase);
    col *= mix(1.0, 0.5 + 1.0 * on, m * smoothstep(0.08, 0.35, luma));
    col += m * on * 0.06 * vec3(1.0, 0.55, 0.18);
  }
  float tube = cdBox(p, uTube, 0.004);
  if (tube > 0.0) {
    vec2 size = (uTube.zw - uTube.xy) * aspect;
    vec2 q = (p - uTube.xy) / (uTube.zw - uTube.xy);
    col *= 1.0 + tube * (0.06 * sin(t * 1.2) + 0.03 * sin(t * 3.3));
    float bubbles = 0.0;
    for (int i = 0; i < 12; i++) {
      float fi = float(i);
      float y = 1.0 - fract(t * (0.12 + 0.16 * cdHash(fi * 3.1)) + cdHash(fi * 5.3));
      float x = 0.18 + 0.64 * cdHash(fi * 1.7) + 0.05 * sin(t * 2.0 + fi * 1.9);
      float r = 0.0014 + 0.0012 * cdHash(fi * 9.7);
      float d = length((q - vec2(x, y)) * size);
      bubbles += smoothstep(r, r * 0.35, d) * smoothstep(0.0, 0.08, y) * smoothstep(1.0, 0.9, y);
    }
    col += tube * min(bubbles, 1.0) * vec3(0.5, 0.6, 0.6);
  }
  for (int i = 0; i < 2; i++) {
    vec4 s = uSteam[i];
    if (s.w <= 0.0) continue;
    vec2 q = vec2((p.x - s.x) / s.z, (s.y - p.y) / s.w);
    if (q.y < 0.0 || q.y > 1.0 || abs(q.x) > 2.0) continue;
    float x = q.x - 0.35 * q.y * sin(q.y * 4.5 - t * 1.1 + float(i) * 2.0);
    float width = 0.22 + 0.7 * q.y;
    float shape = exp(-x * x / (width * width) * 2.0) * smoothstep(0.0, 0.15, q.y) * (1.0 - q.y);
    float wisp = shape * smoothstep(0.38, 0.78, cdFbm(vec2(x * 2.5, q.y * 2.6 - t * 0.8 + float(i) * 5.0)));
    col = 1.0 - (1.0 - col) * (1.0 - wisp * 0.3 * vec3(0.78, 0.8, 0.82));
  }
  float shade = cdCloud(t);
  float outside = cdBox(p, uWindow, 0.004);
  if (outside > 0.0) {
    float shadow = cdFbm(vec2(p.x * 4.0 - t * 0.03, p.y * 11.0 + 0.3));
    col *= 1.0 - outside * (0.1 * shadow + 0.12 * shade);
    // Haze shimmers over the far ground.
    col *= 1.0 + outside * 0.03 * sin(p.y * 900.0 + t * 3.0) * cdNoise(vec2(p.x * 80.0, t * 0.5));
  }
  float sun = cdBox(p, uSun, 0.03);
  if (sun > 0.0) {
    col *= 1.0 - sun * smoothstep(0.3, 0.75, luma) * (0.08 + 0.22 * shade);
  }
  if (uMist.y > 0.0) {
    float far = smoothstep(uMist.x, uMist.x - 0.25, depth);
    float n = cdFbm(vec2(p.x * 5.0 - t * 0.035, p.y * 8.0 + t * 0.012));
    col = mix(col, vec3(0.44, 0.46, 0.48) * (0.6 + luma), far * uMist.y * smoothstep(0.3, 0.8, n));
  }
  float wet = cdBox(p, uWet, 0.03);
  if (wet > 0.0) {
    // Glints on the water come and go.
    float glint = cdNoise(vec2(p.x * 260.0, p.y * 420.0 + t * 0.7));
    col *= 1.0 + wet * smoothstep(0.45, 0.85, luma) * 0.35 * (glint - 0.5);
  }
  float flow = cdBox(p, uFlow, 0.004);
  if (flow > 0.0) {
    float streak = cdNoise(vec2(p.x * 420.0, p.y * 70.0 - t * 3.0));
    col *= 1.0 + flow * smoothstep(0.25, 0.7, luma) * 0.3 * (streak - 0.5);
  }
  // The bodycam's exposure hunts a little.
  return col * (1.0 + 0.012 * sin(t * 0.23));
}
