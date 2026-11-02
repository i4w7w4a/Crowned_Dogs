uniform float time;
uniform float pxPerMetre;
uniform vec3 boxMin;
uniform vec3 boxSize;
uniform vec3 sunDir;
uniform sampler2D tRays;
uniform float hasRays;
uniform float focusInv;
uniform float dofPx;
attribute vec4 seed;
varying float vAlpha;
varying float vLight;

float phase(float cosT, float g) { float g2 = g * g; return (1.0 - g2) / (12.566 * pow(1.0 + g2 - 2.0 * g * cosT, 1.5)); }

void main() {
  // each speck drifts with the air and wobbles on its own
  vec3 p = position + vec3(0.07, 0.012, 0.03) * time
         + 0.22 * vec3(sin(time * (0.21 + 0.17 * seed.x) + seed.y * 6.28), sin(time * (0.15 + 0.11 * seed.z) + seed.x * 6.28), cos(time * (0.18 + 0.13 * seed.y) + seed.z * 6.28));
  p = boxMin + mod(p - boxMin, boxSize);
  vec4 mv = viewMatrix * vec4(p, 1.0);
  gl_Position = projectionMatrix * mv;
  float depth = -mv.z;
  float size = (0.003 + 0.004 * seed.w) * pxPerMetre / depth;
  // out of focus in front of the lens: a bigger, fainter disc
  float blur = 3.0 * max(0.0, 1.6 / depth - 0.35);
  // behind the enemy: soft like the painting there
  blur += 2.0 * dofPx * clamp(focusInv - 1.0 / depth, 0.0, 0.13);
  float drawn = max(2.5, size + blur);
  gl_PointSize = drawn;
  // a speck smaller than its disc is fainter: the same light spread over more pixels, no sparkle
  vAlpha = clamp(size * size / (drawn * drawn), 0.0, 1.0) * smoothstep(0.8, 1.6, depth) * smoothstep(boxSize.z, boxSize.z * 0.7, depth);
  vec3 view = normalize(p - cameraPosition);
  vec2 sp = gl_Position.xy / gl_Position.w * 0.5 + 0.5;
  // specks show where the shafts fall and towards the opening, and hardly anywhere else
  float shaft = hasRays > 0.5 ? dot(texture2D(tRays, sp).rgb, vec3(0.333)) * 10.0 : 0.3;
  vLight = 0.08 + phase(dot(view, sunDir), 0.6) * 2.0 + shaft;
}
