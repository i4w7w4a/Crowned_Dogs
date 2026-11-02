// Sparks thrown off the dome, each a short streak between where it is and where it was.
// Flight is worked out here from the hit, so the CPU only writes the buffers once per hit.
attribute vec3 aDir;
attribute vec4 aSpark;  // speed m/s, life s, width, warm (1 = the bullet going off)
attribute vec2 aCorner; // along the streak (0 tail, 1 head), across it (-1, 1)
uniform vec3 uOrigin;
uniform float uAge;
uniform float uWidth;
uniform float uAspect;
uniform float wideK;
uniform float wideY;
varying vec2 vCorner;
varying float vHeat;
varying float vWarm;

vec3 at(float t) {
  // drag slows a spark fast, then it falls; the bullet keeps its speed
  float drag = mix(4.5, 0.4, aSpark.w);
  return uOrigin + aDir * aSpark.x * (1.0 - exp(-drag * t)) / drag
    + vec3(0.0, -4.9 * (1.0 - aSpark.w), 0.0) * t * t;
}

void main() {
  float life = aSpark.y;
  float t = min(uAge, life);
  vec4 head = projectionMatrix * modelViewMatrix * vec4(at(t), 1.0);
  vec4 tail = projectionMatrix * modelViewMatrix * vec4(at(max(t - 0.035 - 0.035 * aSpark.w, 0.0)), 1.0);
  vec4 c = mix(tail, head, aCorner.x);
  vec2 s = head.xy / max(head.w, 1e-3) - tail.xy / max(tail.w, 1e-3);
  s.x *= uAspect;
  vec2 across = dot(s, s) > 1e-10 ? normalize(vec2(-s.y, s.x)) : vec2(0.0, 1.0);
  across.x /= uAspect;
  c.xy += across * aCorner.y * uWidth * aSpark.z * c.w;
  vCorner = aCorner;
  vHeat = 1.0 - t / life;
  vWarm = aSpark.w;
  if (uAge >= life || head.w < 0.05 || tail.w < 0.05) c = vec4(2.0, 2.0, 2.0, 1.0);
  gl_Position = c;
  gl_Position.xy = mix(vec2(0.0, wideY) * gl_Position.w, gl_Position.xy, wideK);
}
