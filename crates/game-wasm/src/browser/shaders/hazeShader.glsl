
precision highp sampler2DShadow;
uniform mat4 invProj;
uniform mat4 camWorld;
uniform vec3 camPos;
uniform sampler2D tFig;
uniform float enemyDist;
uniform sampler2D tDepth;
uniform float hasDepth;
uniform sampler2DShadow tShadow;
uniform mat4 shadowMatrix;
uniform float hasShadow;
uniform vec3 sunDir;
uniform vec3 sunColor;
uniform vec3 skyColor;
uniform float density;
uniform float time;
in vec2 vUv;

float ign(vec2 p) { return fract(52.9829189 * fract(dot(p, vec2(0.06711056, 0.00583715)))); }

float hash(vec3 p) { p = fract(p * 0.3183099 + 0.1); p *= 17.0; return fract(p.x * p.y * p.z * (p.x + p.y + p.z)); }
float noise(vec3 x) {
  vec3 i = floor(x), f = fract(x);
  f = f * f * (3.0 - 2.0 * f);
  return mix(mix(mix(hash(i), hash(i + vec3(1, 0, 0)), f.x), mix(hash(i + vec3(0, 1, 0)), hash(i + vec3(1, 1, 0)), f.x), f.y),
             mix(mix(hash(i + vec3(0, 0, 1)), hash(i + vec3(1, 0, 1)), f.x), mix(hash(i + vec3(0, 1, 1)), hash(i + vec3(1, 1, 1)), f.x), f.y), f.z);
}
float clouds(vec3 p) {
  float v = 0.0, a = 0.55;
  for (int o = 0; o < OCTAVES; o++) { v += a * noise(p); p = p * 2.03 + vec3(1.7, 9.2, 4.1); a *= 0.5; }
  return v;
}
// Henyey-Greenstein: haze glows when you look towards the light through it
float phase(float cosT, float g) { float g2 = g * g; return (1.0 - g2) / (12.566 * pow(1.0 + g2 - 2.0 * g * cosT, 1.5)); }

void main() {
  vec4 v = invProj * vec4(vUv * 2.0 - 1.0, 1.0, 1.0);
  vec3 dir = normalize((camWorld * vec4(v.xyz / v.w, 0.0)).xyz);
  float tMax = 24.0;
  if (dir.y < -0.001) tMax = min(tMax, camPos.y / -dir.y); // the painted floor
  if (hasDepth > 0.5) tMax = min(tMax, 1.0 / max(textureLod(tDepth, vUv, log2(max(1.0, float(textureSize(tDepth, 0).y) * 0.04))).r * 1.25, 0.01) / max(-dir.z, 0.05)); // the painted walls and machines
  if (texture(tFig, vUv).a > 0.5) tMax = min(tMax, enemyDist / max(-dir.z, 0.2)); // the enemy's body
  float cosT = dot(dir, sunDir);
  float lobe = phase(cosT, 0.6) * 3.0 + phase(cosT, -0.2) * 1.5;
  vec3 drift = vec3(0.06, 0.012, 0.035) * time;
  float jit = ign(gl_FragCoord.xy);
  vec3 light = vec3(0.0);
  float T = 1.0;
  // steps bunch up near the lens, where the figures and their shadows are
  for (int i = 0; i < STEPS; i++) {
    float u0 = (float(i) + jit) / float(STEPS), u1 = (float(i) + 1.0 + jit) / float(STEPS);
    float t = tMax * u0 * u0, dt = tMax * (u1 * u1 - u0 * u0);
    vec3 p = camPos + dir * t;
    float d = density * exp(-max(p.y, 0.0) * 0.32) * smoothstep(0.15, 1.0, clouds(p * vec3(0.32, 0.5, 0.32) + drift));
    float vis = 1.0;
    if (hasShadow > 0.5) {
      vec4 sc = shadowMatrix * vec4(p, 1.0);
      sc.xyz /= sc.w;
      if (all(greaterThan(sc.xyz, vec3(0.0))) && all(lessThan(sc.xyz, vec3(1.0)))) vis = texture(tShadow, vec3(sc.xy, sc.z - 0.002));
    }
    light += T * d * dt * (skyColor + sunColor * lobe * vis);
    T *= exp(-d * dt);
  }
  gl_FragColor = vec4(min(light, vec3(0.6)), T);
}