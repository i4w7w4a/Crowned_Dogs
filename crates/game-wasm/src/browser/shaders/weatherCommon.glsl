uniform vec4 cdWeather;
uniform float cdWeatherTime;
uniform float cdWeatherDim;
uniform vec3 cdFogColor;
uniform vec3 cdDustColor;
uniform vec3 cdRainColor;
float wxHash(vec2 p) { p = fract(p * vec2(123.34, 456.21)); p += dot(p, p + 45.32); return fract(p.x * p.y); }
float wxNoise(vec2 p) {
  vec2 i = floor(p), f = fract(p);
  f = f * f * (3.0 - 2.0 * f);
  return mix(mix(wxHash(i), wxHash(i + vec2(1.0, 0.0)), f.x), mix(wxHash(i + vec2(0.0, 1.0)), wxHash(i + vec2(1.0, 1.0)), f.x), f.y);
}
float wxFbm(vec2 p) { return 0.55 * wxNoise(p) + 0.3 * wxNoise(p * 2.03 + 1.7) + 0.15 * wxNoise(p * 4.1 + 5.2); }
uniform vec4 cdWeather2;
// Drops sitting on the glass, one or none per cell: each lands, sits and dries, and the
// next one lands somewhere else in the cell. Gives the drop's slope in xy, its cover in z.
vec3 wxDrops(vec2 q, float cell, float t, float seed, float amount) {
  vec2 g = q * cell, id = floor(g);
  float rate = 0.03 + 0.05 * wxHash(id + seed + 1.3);
  float life = t * rate + wxHash(id + seed) * 5.0;
  vec2 k = id + seed + floor(life) * 1.618;
  life = fract(life);
  float live = step(1.0 - amount, wxHash(k + 9.7)) * smoothstep(0.0, 0.02, life) * smoothstep(1.0, 0.55, life);
  vec2 o = vec2(wxHash(k + 2.1), wxHash(k + 4.7)) * 0.44 + 0.28;
  float r = (0.09 + 0.12 * wxHash(k + 6.1)) * mix(1.0, 0.7, life);
  vec2 d = (fract(g) - o) / r;
  d.y *= 0.9 + 0.2 * wxHash(k + 8.3); // never quite round
  float cover = smoothstep(1.0, 0.5, length(d)) * live; // the glass is out of focus: soft edges
  return vec3(d * cover, cover);
}
// A drop running down the glass in its own column, leaving a broken trail of beads.
vec3 wxSlide(vec2 q, float cols, float t, float seed) {
  float col = floor(q.x * cols);
  float h = wxHash(vec2(col, seed));
  if (h < 0.6) return vec3(0.0);
  float speed = 0.09 + 0.16 * wxHash(vec2(col, seed + 3.3));
  float head = 1.2 - fract(t * speed + h * 7.0) * 1.6;
  float x = (fract(q.x * cols) - 0.5 + 0.18 * sin(q.y * 21.0 + h * 6.0)) / cols; // the track wanders
  vec2 d = vec2(x, (q.y - head) * 0.85) / 0.012;
  float cover = smoothstep(1.0, 0.5, length(d));
  vec3 drop = vec3(d * cover, cover);
  float above = q.y - head;
  if (above > 0.0 && above < 0.3) {
    float row = floor(q.y * 110.0);
    vec2 b = vec2(x, (fract(q.y * 110.0) - 0.5) / 110.0) / 0.0032;
    float bead = smoothstep(1.0, 0.7, length(b)) * step(0.55, wxHash(vec2(col, row) + seed)) * (1.0 - above / 0.3);
    if (bead > drop.z) drop = vec3(b * bead, bead);
  }
  return drop;
}
