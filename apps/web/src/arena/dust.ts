// Salt dust hanging in the hall: specks drifting slowly through the air, bright where
// the shafts of light fall and when you look towards the opening, soft and large when
// they float right in front of the lens. Drawn with the enemy, so his body hides the
// specks behind him.

import * as THREE from "three";

/** The air the dust floats in, metres: in front of the camera (z forward is negative), up to the roof trusses. */
const BOX = { min: new THREE.Vector3(-4, 0.1, -16), size: new THREE.Vector3(8, 4.6, 15) };

const vertexShader = /* glsl */ `
uniform float time;
uniform float pxPerMetre;
uniform vec3 boxMin;
uniform vec3 boxSize;
uniform vec3 sunDir;
uniform sampler2D tRays;
uniform float hasRays;
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
  float drawn = max(2.5, size + blur);
  gl_PointSize = drawn;
  // a speck smaller than its disc is fainter: the same light spread over more pixels, no sparkle
  vAlpha = clamp(size * size / (drawn * drawn), 0.0, 1.0) * smoothstep(0.8, 1.6, depth) * smoothstep(boxSize.z, boxSize.z * 0.7, depth);
  vec3 view = normalize(p - cameraPosition);
  vec2 sp = gl_Position.xy / gl_Position.w * 0.5 + 0.5;
  // specks show where the shafts fall and towards the opening, and hardly anywhere else
  float shaft = hasRays > 0.5 ? dot(texture2D(tRays, sp).rgb, vec3(0.333)) * 10.0 : 0.3;
  vLight = 0.08 + phase(dot(view, sunDir), 0.6) * 2.0 + shaft;
}`;

const fragmentShader = /* glsl */ `
uniform vec3 color;
varying float vAlpha;
varying float vLight;
void main() {
  float r = length(gl_PointCoord - 0.5) * 2.0;
  float a = vAlpha * smoothstep(1.0, 0.55, r);
  if (a < 0.002) discard;
  gl_FragColor = vec4(color * vLight * a, 0.0); // adds light, covers nothing
}`;

export class Dust {
  readonly points: THREE.Points;
  private material: THREE.ShaderMaterial;
  private capacity = 0;

  constructor() {
    this.material = new THREE.ShaderMaterial({
      vertexShader, fragmentShader,
      uniforms: {
        time: { value: 0 }, pxPerMetre: { value: 800 }, boxMin: { value: BOX.min }, boxSize: { value: BOX.size },
        sunDir: { value: new THREE.Vector3(0, 0.5, -1).normalize() }, tRays: { value: null }, hasRays: { value: 0 },
        color: { value: new THREE.Color(0xe9eef2).multiplyScalar(0.55) },
      },
      transparent: true, depthWrite: false, fog: false,
      blending: THREE.CustomBlending, blendSrc: THREE.OneFactor, blendDst: THREE.OneFactor, blendSrcAlpha: THREE.ZeroFactor, blendDstAlpha: THREE.OneFactor,
    });
    this.points = new THREE.Points(new THREE.BufferGeometry(), this.material);
    this.points.frustumCulled = false;
    this.points.visible = false;
  }

  /** How many specks float; 0 hides the dust. */
  setCount(n: number): void {
    this.points.visible = n > 0;
    if (n > this.capacity) {
      const pos = new Float32Array(n * 3), seed = new Float32Array(n * 4);
      for (let i = 0; i < n; i++) {
        pos[i * 3] = BOX.min.x + Math.random() * BOX.size.x;
        pos[i * 3 + 1] = BOX.min.y + Math.random() * BOX.size.y;
        pos[i * 3 + 2] = BOX.min.z + Math.random() * BOX.size.z;
        for (let k = 0; k < 4; k++) seed[i * 4 + k] = Math.random();
      }
      this.points.geometry.dispose();
      const g = new THREE.BufferGeometry();
      g.setAttribute("position", new THREE.BufferAttribute(pos, 3));
      g.setAttribute("seed", new THREE.BufferAttribute(seed, 4));
      this.points.geometry = g;
      this.capacity = n;
    }
    this.points.geometry.setDrawRange(0, n);
  }

  /** Each frame: the time, how many pixels a metre is at one metre away, where the light comes from, and the shafts it lies in. */
  update(time: number, pxPerMetre: number, sunDir: THREE.Vector3, rays: THREE.Texture | null): void {
    const u = this.material.uniforms;
    u.time!.value = time;
    u.pxPerMetre!.value = pxPerMetre;
    (u.sunDir!.value as THREE.Vector3).copy(sunDir);
    u.tRays!.value = rays;
    u.hasRays!.value = rays ? 1 : 0;
  }
}
