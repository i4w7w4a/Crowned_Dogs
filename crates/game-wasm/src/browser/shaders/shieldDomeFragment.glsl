// The dome never shows whole: only the section over the zone that was hit lights up.
// A hot point where the shot lands, a wave that runs out over that section and lights
// its hex cells, the glassy curve of the section, then a fade. The section's edge
// follows whole cells. Light is added (alpha 0), so it never covers the picture.
uniform vec3 uHit;
uniform float uAge;
uniform float uLife;
uniform float uPower;
uniform float uBroken;
uniform float uCells;
uniform float uReach;
uniform float uDetail;
uniform vec3 uColor;
varying vec3 vLocal;
varying vec3 vNormal;
varying vec3 vView;

float hash(vec2 p) {
  return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453);
}

void main() {
  float k = clamp(uAge / uLife, 0.0, 1.0);
  vec3 p = normalize(vLocal);
  float d = acos(clamp(dot(p, uHit), -1.0, 1.0));
  if (d > uReach * 1.3) discard;

  // Hex cells: the longitude holds a whole number of cells, so the seam does not show.
  float toAngle = 6.2831853 / uCells;
  vec2 uv = vec2(atan(p.z, p.x), asin(p.y)) / toAngle;
  const vec2 R = vec2(1.0, 1.7320508);
  vec2 a = mod(uv, R) - 0.5 * R;
  vec2 b = mod(uv - 0.5 * R, R) - 0.5 * R;
  vec2 g = dot(a, a) < dot(b, b) ? a : b;
  vec2 q = abs(g);
  float edge = 0.5 - max(dot(q, vec2(0.5, 0.8660254)), q.x);
  float line = 1.0 - smoothstep(0.0, fwidth(edge) * 1.2 + 0.015, edge);
  vec2 centre = (uv - g) * toAngle;
  float cell = hash(floor((uv - g) * 2.0 + 0.5));
  vec3 cp = vec3(cos(centre.y) * cos(centre.x), sin(centre.y), cos(centre.y) * sin(centre.x));
  float dc = acos(clamp(dot(cp, uHit), -1.0, 1.0));

  // The lit section: whole cells near the hit, its rim a little ragged, fading off softly.
  float reach = uReach * (0.55 + 0.45 * smoothstep(0.0, 0.12, uAge));
  float inside = smoothstep(reach, reach - 0.06, dc + (cell - 0.5) * 0.1);
  float soft = smoothstep(uReach * 1.25, uReach * 0.3, d);
  float mask = inside * soft;

  // The wave runs out from the hit to the section's edge; a weaker echo follows.
  float front = uReach * 1.1 * (1.0 - exp(-uAge * 7.0));
  float band = 0.035 + 0.06 * k;
  float wave = exp(-pow((d - front) / band, 2.0));
  float echo = uDetail * 0.5 * exp(-pow((d - front * 0.55) / (band * 0.8), 2.0));
  float fade = pow(1.0 - k, 1.4);
  float rise = smoothstep(0.0, 0.03, uAge);

  // Seen edge-on the section reads like thick curved glass.
  float rim = pow(1.0 - abs(dot(normalize(vNormal), normalize(-vView))), 2.0);
  float core = exp(-d * d * 400.0) * exp(-uAge * 14.0) * 4.0;
  float glow = exp(-d * d * 40.0) * exp(-uAge * 9.0);
  float fill = smoothstep(0.7, 1.0, cell) * glow * (0.2 + 0.3 * uDetail);

  float field = (wave + echo) * fade;
  float plate = (0.04 + rim * 0.3) * rise * fade;
  float hex = line * (field * 1.4 + glow * 0.9 + plate * 2.0);
  float light = (plate + field * 0.3 + glow * 0.35 + hex + fill) * mask + core;

  // A broken guard: cells round the hit drop out and the section stutters.
  if (uBroken > 0.5) {
    float hole = smoothstep(uReach * 0.8, uReach * 0.2, d) * step(cell, 0.2 + k * 1.1);
    light *= (1.0 - hole) * (0.5 + 0.5 * step(0.4, hash(vec2(floor(uAge * 32.0), 7.0))));
  }
  light *= gl_FrontFacing ? 1.0 : 0.35;
  vec3 color = mix(uColor, vec3(1.0), clamp(core * 0.4 + glow * 0.25, 0.0, 1.0));
  gl_FragColor = vec4(color * light * uPower, 0.0);
}
