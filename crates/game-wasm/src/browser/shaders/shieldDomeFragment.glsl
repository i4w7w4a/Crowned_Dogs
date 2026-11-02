// The dome never shows whole: only the section over the zone that was hit lights up.
// A hot point where the shot lands, a wave that runs out over that section and lights
// its hex cells, the glassy curve of the section, then a fade. The section's edge
// follows whole cells. Light is added (alpha 0), so it never covers the picture.
// In heat and in rain the field shows faintly at rest, a small patch here and there.
uniform vec3 uHit;
uniform float uAge;
uniform float uLife;
uniform float uPower;
uniform float uBroken;
uniform float uCells;
uniform float uReach;
uniform float uDetail;
uniform float uHeat;
uniform float uWet;
uniform float uTime;
uniform vec3 uColor;
varying vec3 vLocal;
varying vec3 vNormal;
varying vec3 vView;

float hash(vec2 p) {
  return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453);
}

void main() {
  float k = clamp(uAge / max(uLife, 0.001), 0.0, 1.0);
  vec3 p = normalize(vLocal);
  float d = acos(clamp(dot(p, uHit), -1.0, 1.0));
  float resting = uHeat + uWet;
  if (d > uReach * 1.3 && resting <= 0.0) discard;

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
  light *= uAge < uLife ? 1.0 : 0.0;
  light *= gl_FrontFacing ? 1.0 : 0.35;
  vec3 color = mix(uColor, vec3(1.0), clamp(core * 0.4 + glow * 0.25, 0.0, 1.0));
  vec3 outLight = color * light * uPower;

  // The field at rest never shows whole: a small patch of cells surfaces here and there for
  // a moment and sinks again, hot air rippling through it or rain running down it. The
  // patches go anywhere on the dome, so they never tell which zones are guarded.
  if (resting > 0.0) {
    float edgeOn = pow(1.0 - abs(dot(normalize(vNormal), normalize(-vView))), 2.0);
    float lon = atan(p.z, p.x);
    float spots = 0.0;
    for (int i = 0; i < 3; i++) {
      float fi = float(i);
      float clock = uTime * 0.45 + fi * 0.37;
      float slot = floor(clock);
      float y = mix(-0.35, 0.85, hash(vec2(fi + 4.1, slot)));
      float around = hash(vec2(slot, fi)) * 6.2831853;
      vec3 spot = vec3(sqrt(1.0 - y * y) * cos(around), y, sqrt(1.0 - y * y) * sin(around));
      float size = 0.2 + 0.16 * hash(vec2(slot + 2.7, fi));
      // measured from cell centres, so a patch is made of whole cells with a ragged edge
      float ds = acos(clamp(dot(cp, spot), -1.0, 1.0)) + (cell - 0.5) * 0.12;
      spots = max(spots, smoothstep(size, size * 0.45, ds) * sin(3.1415927 * fract(clock)));
    }
    float ripple = 0.6 + 0.4 * sin(p.y * 9.0 - uTime * 1.7 + sin(lon * 5.0 + uTime * 0.6) * 1.5);
    float glass = line * 0.1 + 0.02 + edgeOn * 0.05 + smoothstep(0.8, 1.0, cell) * 0.025;
    float heat = uHeat * ripple * glass;
    float col = floor(lon / 6.2831853 * uCells * 1.5);
    float hc = hash(vec2(col, 3.0));
    float phase = fract(p.y * 2.5 + uTime * (0.35 + 0.4 * hc) + hc * 9.0);
    float across = abs(fract(lon / 6.2831853 * uCells * 1.5) - 0.5);
    float run = step(0.5, hc) * smoothstep(0.2, 0.0, across) * (smoothstep(0.05, 0.0, phase) + 0.25 * exp(-phase * 10.0));
    float wet = uWet * (glass + run * 0.25);
    float rest = (heat + wet) * spots * (gl_FrontFacing ? 1.0 : 0.35);
    outLight += mix(uColor, vec3(1.0), 0.35) * rest;
  }
  gl_FragColor = vec4(outLight, 0.0);
}
