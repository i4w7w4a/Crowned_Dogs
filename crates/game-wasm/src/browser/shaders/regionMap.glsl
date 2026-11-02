// Region map shaders, one file split at the «//# name» marks by region_map.rs.
// Every map shader shares one set of uniforms: the clock, the picked region, the style wipe,
// the power-on slice, the fog, the sonar ping, the lightning flash and both regions' events.

//# lib
uniform float uTime, uSel, uWipe, uScan, uCut, uBoot;
uniform vec2 uFog, uEvents;
uniform vec3 uPing;
uniform vec4 uFlash;
const vec3 CYAN = vec3(0.45, 0.83, 0.89);
float aaLine(float v, float width) {
  float w = fwidth(v);
  float f = abs(fract(v - 0.5) - 0.5) / max(w, 1e-5);
  return (1.0 - smoothstep(width - 0.5, width + 0.5, f)) * (1.0 - smoothstep(0.2, 0.55, w));
}
float hash2(vec2 p) { return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453); }
float vnoise(vec2 p) {
  vec2 i = floor(p), f = fract(p); vec2 u = f * f * (3.0 - 2.0 * f);
  return mix(mix(hash2(i), hash2(i + vec2(1.0, 0.0)), u.x), mix(hash2(i + vec2(0.0, 1.0)), hash2(i + vec2(1.0, 1.0)), u.x), u.y);
}
float northOf(vec3 p) { return step(p.z, 0.3 + 0.18 * p.x); }
float mineOf(vec3 p) { float n = northOf(p); return mix(1.0 - n, n, uSel); }
float wipeEdge() { return mix(-1.62, 1.62, uWipe); }
float fogOf(float depth) { return mix(1.0, 0.45, smoothstep(uFog.x, uFog.y, depth)); }
vec3 flashOf(vec3 p) { return vec3(0.55, 0.7, 0.85) * uFlash.w * exp(-length(p.xz - uFlash.xy) / uFlash.z); }

//# terrain-vertex
attribute float salt;
#ifdef BARY
attribute vec3 bary; varying vec3 vBary;
#endif
uniform float uCut;
varying vec3 vPos; varying float vSalt; varying float vRaw; varying float vDepth;
void main() {
#ifdef BARY
  vBary = bary;
#endif
  vec3 p = position; vRaw = p.y; p.y = min(p.y, uCut);
  vPos = p; vSalt = salt;
  vec4 mv = modelViewMatrix * vec4(p, 1.0);
  vDepth = -mv.z;
  gl_Position = projectionMatrix * mv;
}

//# terrain
uniform vec3 uZones[4];
varying vec3 vPos; varying float vSalt; varying float vRaw; varying float vDepth;
void clipBlock() { vec2 q = abs(vPos.xz) - vec2(1.0, 1.45); if (max(q.x, q.y) > 0.0) discard; }
vec3 finish(vec3 col) {
  float north = northOf(vPos);
  float mine = mix(1.0 - north, north, uSel);
  float ev = mix(uEvents.y, uEvents.x, north);
  // Power-on: ground not raised yet is a flat cyan scan sheet.
  float sheet = step(uCut, vRaw - 0.0005);
  float grid = max(aaLine(vPos.x * 20.0, 0.5), aaLine(vPos.z * 20.0, 0.5));
  col = mix(col, vec3(0.03, 0.07, 0.08) + CYAN * grid * 0.35, sheet);
  // Shelled ground: an amber hatch.
  float d = 0.0;
  for (int i = 0; i < 4; i++) d = max(d, 1.0 - smoothstep(0.8, 1.0, length(vPos.xz - uZones[i].xy) / uZones[i].z));
  col = mix(col, vec3(0.91, 0.6, 0.22), d * (0.06 + aaLine((vPos.x + vPos.z) * 55.0, 0.6) * 0.3));
  // Cloud shadows drifting over the land, heavier under a storm.
  vec2 cq = vPos.xz * 1.5 + uTime * vec2(0.03, 0.012);
  float cloud = smoothstep(0.42, 0.78, vnoise(cq) * 0.62 + vnoise(cq * 2.4 + 3.7) * 0.38);
  col *= 1.0 - cloud * (abs(ev - 1.0) < 0.1 ? 0.5 : 0.3);
  // Dry wind: dust streaks running over the ground.
  if (abs(ev - 2.0) < 0.1) {
    float streak = smoothstep(0.58, 0.92, vnoise(vec2(vPos.x * 2.2 - uTime * 0.25, vPos.z * 18.0 + vPos.x * 3.0)));
    streak *= smoothstep(0.3, 0.7, vnoise(vPos.xz * 3.0 - vec2(uTime * 0.09, 0.0)));
    col = mix(col, vec3(0.62, 0.55, 0.4), streak * 0.4);
  }
  // Chemical release: a sick low haze in the hollows.
  if (abs(ev - 3.0) < 0.1) col = mix(col, vec3(0.55, 0.62, 0.3), 0.2 * smoothstep(0.3, 0.8, vnoise(vPos.xz * 2.5 + uTime * 0.04)) * (1.0 - smoothstep(0.05, 0.25, vPos.y)));
  col += flashOf(vPos) * 0.7;
  col *= mix(0.5, 1.0, mine);
  // Sonar ping from the base of the picked region.
  float r = (uTime - uPing.z) * 0.85;
  float pd = length(vPos.xz - uPing.xy);
  col += CYAN * exp(-pow((pd - r) * 24.0, 2.0)) * clamp(1.0 - r / 1.7, 0.0, 1.0) * step(0.0, r) * mine * 0.45;
  // The style wipe: a scan line runs across the block.
  col += CYAN * exp(-abs(vPos.z - wipeEdge()) * 70.0) * uScan;
  // Power-on: the rising slice glows, the fresh ground cools under it.
  float under = uCut - vRaw;
  col += CYAN * (exp(-abs(under) * 150.0) * 0.9 + step(0.0, under) * exp(-under * 22.0) * 0.18) * step(uCut, 1.0);
  return col * fogOf(vDepth);
}

//# contour
void main() {
  clipBlock();
  if (vPos.z < wipeEdge()) discard;
  vec3 n = normalize(cross(dFdx(vPos), dFdy(vPos)));
  float hs = clamp(dot(n, normalize(vec3(-0.55, 0.75, -0.35))), 0.0, 1.0);
  vec3 col = mix(vec3(0.045, 0.052, 0.056), vec3(0.09, 0.1, 0.1), hs);
  col = mix(col, vec3(0.16, 0.17, 0.16), vSalt * 0.6);
  float minor = aaLine(vPos.y * 90.0 - 0.3, 0.5);
  float major = aaLine(vPos.y * 18.0 - 0.06, 0.75);
  vec3 lineCol = mix(vec3(0.55, 0.58, 0.58), vec3(0.93, 0.92, 0.88), vSalt);
  col = mix(col, lineCol * 0.55, minor * 0.55);
  col = mix(col, lineCol, major * 0.8);
  float grid = max(aaLine(vPos.x * 5.0, 0.4), aaLine(vPos.z * 5.0, 0.4));
  col = mix(col, vec3(0.3, 0.33, 0.34), grid * 0.18);
  gl_FragColor = vec4(finish(col), 1.0);
}

//# relief
varying vec3 vBary;
void main() {
  clipBlock();
  if (vPos.z >= wipeEdge()) discard;
  vec3 n = normalize(cross(dFdx(vPos), dFdy(vPos)));
  float sun = clamp(dot(n, normalize(vec3(-0.75, 0.5, -0.3))), 0.0, 1.0);
  float sky = 0.5 + 0.5 * n.y;
  float h = clamp(vPos.y / 0.3, 0.0, 1.0);
  vec3 low = vec3(0.12, 0.13, 0.12), mid = vec3(0.2, 0.21, 0.18), high = vec3(0.34, 0.34, 0.31);
  vec3 base = h < 0.5 ? mix(low, mid, h * 2.0) : mix(mid, high, h * 2.0 - 1.0);
  base = mix(base, vec3(0.19, 0.2, 0.15), 0.5 * smoothstep(0.3, 0.7, hash2(floor(vPos.xz * 14.0))));
  base = mix(base, vec3(0.8, 0.81, 0.79), vSalt * 0.85);
  vec3 col = base * (0.35 * sky + 1.1 * sun) + vec3(0.01, 0.015, 0.02);
  vec3 e3 = smoothstep(vec3(0.0), fwidth(vBary) * 1.2, vBary);
  col = mix(col, vec3(0.75, 0.78, 0.78), (1.0 - min(min(e3.x, e3.y), e3.z)) * 0.16);
  gl_FragColor = vec4(finish(col), 1.0);
}

//# wall-vertex
attribute float ground; uniform float uCut;
varying float vG; varying vec3 vPos; varying float vRaw; varying float vDepth;
void main() {
  vG = ground; vRaw = position.y; vec3 p = position; p.y = min(p.y, uCut); vPos = p;
  vec4 mv = modelViewMatrix * vec4(p, 1.0); vDepth = -mv.z; gl_Position = projectionMatrix * mv;
}

//# wall
varying float vG; varying vec3 vPos; varying float vRaw; varying float vDepth;
void main() {
  float depth = clamp((vG - vPos.y) / 0.3, 0.0, 1.0);
  vec3 col = mix(vec3(0.07, 0.075, 0.08), vec3(0.025, 0.028, 0.03), depth);
  col = mix(col, vec3(0.3, 0.32, 0.32), aaLine(vPos.y * 60.0 + sin(vPos.x * 9.0 + vPos.z * 7.0) * 0.6, 0.5) * 0.35);
  if (vPos.y > vG) col = mix(vec3(0.03, 0.12, 0.16), CYAN, aaLine(vPos.y * 160.0 - uTime * 0.6, 0.5) * 0.5);
  col += CYAN * exp(-abs(uCut - vPos.y) * 150.0) * 0.8 * step(uCut, 1.0);
  gl_FragColor = vec4(col * fogOf(vDepth), 1.0);
}

//# flat-vertex
varying vec3 vPos; varying float vDepth;
void main() { vPos = position; vec4 mv = modelViewMatrix * vec4(position, 1.0); vDepth = -mv.z; gl_Position = projectionMatrix * mv; }

//# table
varying vec3 vPos;
void main() {
  float g = max(aaLine(vPos.x * 4.0, 0.5), aaLine(vPos.z * 4.0, 0.5));
  float rr = length(vPos.xz * vec2(1.0, 0.8));
  float fade = 1.0 - smoothstep(1.4, 3.6, rr);
  float R = uBoot * 4.0;
  float lit = 1.0 - smoothstep(R - 0.5, R, rr);
  float ring = exp(-abs(rr - R) * 10.0) * (1.0 - uBoot);
  float shadow = (1.0 - smoothstep(0.0, 0.35, max(abs(vPos.x) - 1.0, abs(vPos.z) - 1.45))) * step(-0.06, uCut);
  gl_FragColor = vec4(vec3(0.55, 0.6, 0.62) * g + CYAN * ring, (g * 0.16 * fade + shadow * 0.45 * fade) * lit + ring * 0.55);
}

//# water
uniform float uAlpha; varying vec3 vPos; varying float vDepth;
void main() {
  if (vPos.z > -0.36 || abs(vPos.x) > 1.0 || vPos.z < -1.45) discard;
  float wave = sin(vPos.x * 40.0 + uTime * 0.8) * 0.004 + sin(vPos.x * 13.0 - uTime * 0.5 + vPos.z * 5.0) * 0.003;
  float hatch = aaLine((vPos.z + wave) * 70.0, 0.45);
  vec3 col = mix(vec3(0.02, 0.07, 0.1), CYAN, hatch * 0.55);
  col += CYAN * smoothstep(0.994, 1.0, sin(vPos.x * 9.0 - uTime * 0.6 + vPos.z * 13.0)) * 0.3;
  vec2 cq = vPos.xz * 1.5 + uTime * vec2(0.03, 0.012);
  col *= 1.0 - 0.45 * smoothstep(0.42, 0.78, vnoise(cq) * 0.62 + vnoise(cq * 2.4 + 3.7) * 0.38);
  col += flashOf(vPos) * 0.9;
  col *= mix(0.5, 1.0, uSel);
  gl_FragColor = vec4(col * fogOf(vDepth), 0.92 * uAlpha);
}

//# solid-vertex
varying vec3 vW; varying vec3 vN; varying float vDepth;
void main() {
  vec4 w = modelMatrix * vec4(position, 1.0); vW = w.xyz; vN = normalize(mat3(modelMatrix) * normal);
  vec4 mv = viewMatrix * w; vDepth = -mv.z; gl_Position = projectionMatrix * mv;
}

//# solid
uniform vec3 uColor; varying vec3 vW; varying vec3 vN; varying float vDepth;
void main() {
  float relief = step(vW.z, wipeEdge());
  vec3 n = normalize(vN);
  float sun = clamp(dot(n, normalize(vec3(-0.75, 0.5, -0.3))), 0.0, 1.0);
  vec3 col = mix(vec3(0.043, 0.05, 0.055), uColor * (0.3 + 0.25 * n.y + 0.8 * sun), relief);
  col *= mix(0.5, 1.0, mineOf(vW));
  col += flashOf(vW) * 0.5;
  gl_FragColor = vec4(col * fogOf(vDepth), 1.0);
}

//# edge-vertex
varying vec3 vW; varying float vDepth;
void main() { vec4 w = modelMatrix * vec4(position, 1.0); vW = w.xyz; vec4 mv = viewMatrix * w; vDepth = -mv.z; gl_Position = projectionMatrix * mv; }

//# edge
uniform float uOpacity; varying vec3 vW; varying float vDepth;
void main() {
  float relief = step(vW.z, wipeEdge());
  vec3 col = mix(vec3(0.925, 0.91, 0.875), vec3(0.03, 0.035, 0.04), relief);
  gl_FragColor = vec4(col, uOpacity * mix(0.45, 1.0, mineOf(vW)) * fogOf(vDepth));
}

//# motes-vertex
attribute float seed; uniform float uTime, uPx; varying float vA;
void main() {
  vec3 p = position + vec3(sin(uTime * 0.11 + seed * 40.0) * 0.06, sin(uTime * 0.07 + seed * 20.0) * 0.03, cos(uTime * 0.09 + seed * 30.0) * 0.06);
  vec4 mv = modelViewMatrix * vec4(p, 1.0);
  gl_PointSize = uPx * (0.004 + 0.006 * seed) / -mv.z;
  vA = (0.35 + 0.65 * sin(uTime * (0.6 + seed) + seed * 50.0) * 0.5 + 0.325) * smoothstep(8.0, 3.0, -mv.z);
  gl_Position = projectionMatrix * mv;
}

//# motes
uniform float uAlpha; varying float vA;
void main() { float d = length(gl_PointCoord - 0.5); gl_FragColor = vec4(vec3(0.75, 0.85, 0.88), (1.0 - smoothstep(0.1, 0.5, d)) * vA * 0.5 * uAlpha); }

//# rain-vertex
attribute float seed; uniform float uTime, uPx; varying float vA;
void main() {
  float top = 0.62, y = top - mod(uTime * (0.55 + 0.25 * seed) + seed * top, top);
  vec3 p = vec3(position.x + (top - y) * 0.08, y, position.z);
  vec4 mv = modelViewMatrix * vec4(p, 1.0);
  gl_PointSize = uPx * 0.028 / -mv.z;
  vA = smoothstep(0.0, 0.08, y) * smoothstep(top, top - 0.12, y);
  gl_Position = projectionMatrix * mv;
}

//# rain
uniform float uAlpha; varying float vA;
void main() {
  vec2 c = gl_PointCoord - 0.5;
  float a = (1.0 - smoothstep(0.0, 0.05, abs(c.x + c.y * 0.15))) * (1.0 - smoothstep(0.15, 0.5, abs(c.y)));
  gl_FragColor = vec4(0.66, 0.76, 0.82, a * vA * 0.3 * uAlpha);
}

//# dust-vertex
attribute float seed; uniform float uTime, uPx, uNorth; varying float vA;
void main() {
  float x = mod(position.x + 1.0 + uTime * (0.07 + 0.08 * seed), 2.0) - 1.0;
  float z = position.z + sin(uTime * 0.6 + seed * 30.0) * 0.03;
  vec3 p = vec3(x, 0.12 + 0.2 * seed + sin(uTime * 0.9 + seed * 9.0) * 0.015, z);
  vec4 mv = modelViewMatrix * vec4(p, 1.0);
  gl_PointSize = uPx * (0.03 + 0.03 * seed) / -mv.z;
  float north = step(z, 0.3 + 0.18 * x);
  vA = mix(1.0 - north, north, uNorth) * smoothstep(1.0, 0.85, abs(x)) * smoothstep(1.45, 1.3, abs(z));
  gl_Position = projectionMatrix * mv;
}

//# dust
uniform float uAlpha; varying float vA;
void main() {
  vec2 c = gl_PointCoord - 0.5;
  float a = exp(-(c.x * c.x * 9.0 + c.y * c.y * 90.0));
  gl_FragColor = vec4(0.72, 0.64, 0.48, a * vA * 0.32 * uAlpha);
}

//# radar-vertex
varying vec2 vP;
void main() { vP = position.xz / 0.16; gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0); }

//# radar
uniform float uOn; varying vec2 vP;
void main() {
  float r = length(vP); if (r > 1.0) discard;
  float a = fract(atan(vP.y, vP.x) / 6.28318 + uTime * 0.22);
  float sweep = pow(a, 8.0) * 0.5;
  float rings = aaLine(r * 3.0, 0.5) * 0.25;
  gl_FragColor = vec4(CYAN, (sweep + rings) * (1.0 - smoothstep(0.85, 1.0, r)) * uOn);
}
