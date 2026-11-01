// Weather lives in the air between the lens and the painting, so it grows with
// distance: the near fighter stays readable, the far hall goes soft.
if (hasBg > 0.5) {
  float inv = hasDepth > 0.5 ? texture(tDepth, vUv).r * 1.25 : 0.06; // the map holds 1/z
  inv = mix(inv, focusInv, step(0.5, f.a)); // the enemy stands at his own depth
  float z = min(1.0 / max(inv, 0.016), 60.0); // the open sky counts as far away
  float far = 1.0 - exp(-z * 0.075);
  float t = cdWeatherTime;
  vec2 p = vec2(vUv.x * bgSize.x / max(bgSize.y, 1.0), vUv.y);
  c *= cdWeatherDim;
  // fog: slow banks rolling across, thicker low down
  if (cdWeather.x > 0.0) {
    float bank = wxFbm(vec2(p.x * 2.2 + t * 0.018, p.y * 3.0 - t * 0.005));
    float low = 1.15 - 0.35 * vUv.y;
    float fog = cdWeather.x * low * (1.0 - exp(-z * (0.045 + 0.06 * bank)));
    c = mix(c, cdFogColor, clamp(fog, 0.0, 0.93));
  }
  // salt dust: gusts of grains streaming sideways, in layers that move at different speeds
  if (cdWeather.y > 0.0) {
    float gust = 0.7 + 0.3 * sin(t * 0.6 + sin(t * 0.21) * 3.0);
    float wave = sin(p.x * 3.0 + t * 0.4) * 0.25;
    float s1 = wxFbm(vec2(p.x * 1.1 - t * 0.5, p.y * 9.0 + wave));
    float s2 = wxFbm(vec2(p.x * 2.6 - t * 1.3, p.y * 24.0 - wave) + 7.3);
    float veil = smoothstep(0.42, 0.72, s1) * 0.7 + smoothstep(0.48, 0.78, s2) * 0.5;
    float low = 1.2 - 0.4 * vUv.y; // the grains ride low over the floor
    float dust = cdWeather.y * gust * low * (0.2 + 0.8 * far) * (0.12 + 1.1 * veil);
    // grains darken the sunlit air a touch and lift the shadows to the dust's tone
    c = mix(c, cdDustColor * (0.82 + 0.18 * veil), clamp(dust, 0.0, 0.88));
    // single grains flicking past the lens
    vec2 g = vec2(p.x * 90.0 - t * 40.0, vUv.y * 55.0 + sin(p.x * 9.0 + t) * 0.6);
    float grain = step(0.975, wxHash(floor(g))) * smoothstep(0.5, 0.0, length(fract(g) - 0.5) - 0.12);
    c += cdDustColor * grain * cdWeather.y * 0.5;
  }
  // rain: thin slanted streaks in three depth layers, hidden behind what stands nearer
  if (cdWeather.z > 0.0) {
    float rain = 0.0;
    for (int i = 0; i < 3; i++) {
      float fi = float(i);
      float cols = 70.0 + fi * 60.0;
      vec2 q = vec2((p.x + vUv.y * 0.07) * cols, vUv.y * cols * 0.09 + t * (2.6 - fi * 0.55));
      vec2 cell = floor(q);
      float h = wxHash(cell + fi * 17.0);
      float x = fract(q.x) - 0.5 + (h - 0.5) * 0.5;
      float y = fract(q.y + h * 7.0);
      float streak = smoothstep(0.09, 0.0, abs(x)) * smoothstep(0.0, 0.3, y) * smoothstep(1.0, 0.6, y) * step(0.62, h);
      rain += streak * step(2.5 + fi * 4.5, z) * (0.5 - fi * 0.12);
    }
    c = mix(c, cdRainColor, clamp(rain * cdWeather.z, 0.0, 0.55));
  }
  // lightning: the far, open parts flare first, the floor near the lens least
  if (cdWeather.w > 0.0) {
    float lit = cdWeather.w * (0.3 + 0.7 * far);
    c = c * (1.0 + lit * 1.5) + vec3(0.78, 0.84, 0.92) * lit * 0.1;
  }
}
vec4 n = smoothRead(tNear, vUv, nearSize);
