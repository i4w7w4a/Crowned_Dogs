// The bodycam lens. The figures are drawn into an off-screen picture, laid over the
// painted backdrop here, and the whole frame then goes through one lens: colours
// split and soften towards the edges and the corners darken a little, so the
// painting and the live figures look shot by the same camera.

import * as THREE from "three";
import { FullScreenQuad } from "three/addons/postprocessing/Pass.js";

export interface LensLook {
  /** Colour split at the very edge, as a share of the distance from the centre. */
  split: number;
  /** Blur at the very edge, pixels of the drawing buffer. */
  blur: number;
  /** How much the corners darken, 0..1. */
  vignette: number;
  /** Colour of the live figures against the painting: saturation (1 = as lit) and a tint. */
  saturation: number;
  tint: number;
}

const vertexShader = /* glsl */ `
varying vec2 vUv;
void main() { vUv = uv; gl_Position = vec4(position.xy, 0.0, 1.0); }`;

const fragmentShader = /* glsl */ `
uniform sampler2D tFig;
uniform sampler2D tBg;
uniform float hasBg;
uniform float exposure;
uniform float aspect;
uniform vec2 texel;
uniform float split;
uniform float blur;
uniform float vignette;
uniform float saturation;
uniform vec3 tint;
varying vec2 vUv;

// three.js ACES filmic, so the figures keep the tone they had when drawn straight to the screen
vec3 RRTAndODTFit(vec3 v) {
  vec3 a = v * (v + 0.0245786) - 0.000090537;
  vec3 b = v * (0.983729 * v + 0.4329510) + 0.238081;
  return a / b;
}
vec3 aces(vec3 color) {
  const mat3 ACESInputMat = mat3(vec3(0.59719, 0.07600, 0.02840), vec3(0.35458, 0.90834, 0.13383), vec3(0.04823, 0.01566, 0.83777));
  const mat3 ACESOutputMat = mat3(vec3(1.60475, -0.10208, -0.00327), vec3(-0.53108, 1.10813, -0.07276), vec3(-0.07367, -0.00605, 1.07602));
  color *= exposure / 0.6;
  color = ACESInputMat * color;
  color = RRTAndODTFit(color);
  color = ACESOutputMat * color;
  return clamp(color, 0.0, 1.0);
}
vec3 toSRGB(vec3 c) {
  return mix(c * 12.92, 1.055 * pow(c, vec3(1.0 / 2.4)) - 0.055, step(vec3(0.0031308), c));
}

// the figures (linear, premultiplied) over the painting (sRGB as stored), the way the browser
// would lay a transparent canvas over a picture
vec4 frame(vec2 uv) {
  vec4 f = texture2D(tFig, uv);
  vec3 lin = f.a > 0.001 ? f.rgb / f.a : f.rgb;
  vec3 c = toSRGB(aces(lin));
  c = mix(vec3(dot(c, vec3(0.2126, 0.7152, 0.0722))), c, saturation) * tint;
  vec3 over = f.a > 0.001 ? c * f.a : c; // sparks add light without covering
  vec3 bg = hasBg > 0.5 ? texture2D(tBg, uv).rgb : vec3(0.0);
  return vec4(over + bg * (1.0 - f.a), hasBg > 0.5 ? 1.0 : f.a);
}

// red and blue land a little apart away from the centre, as through a cheap wide lens
vec4 split3(vec2 uv, float k) {
  if (k < 0.0002) return frame(uv);
  vec2 d = uv - 0.5;
  vec4 g = frame(uv);
  return vec4(frame(0.5 + d * (1.0 + k)).r, g.g, frame(0.5 + d * (1.0 - k)).b, g.a);
}

void main() {
  vec2 d = (vUv - 0.5) * vec2(aspect, 1.0);
  float r = length(d) / length(vec2(aspect, 1.0) * 0.5); // 0 in the centre, 1 in the corners
  float edge = smoothstep(0.45, 1.0, r);
  float k = split * edge * edge;
  vec4 c = split3(vUv, k);
  float b = blur * edge;
  if (b > 0.35) {
    vec2 o = texel * b;
    c = c * 0.36
      + (split3(vUv + vec2(o.x, 0.0), k) + split3(vUv - vec2(o.x, 0.0), k)
      + split3(vUv + vec2(0.0, o.y), k) + split3(vUv - vec2(0.0, o.y), k)) * 0.16;
  }
  float dark = 1.0 - vignette * smoothstep(0.55, 1.1, r);
  gl_FragColor = vec4(c.rgb * dark, c.a);
}`;

export class Lens {
  private target: THREE.WebGLRenderTarget;
  private quad: FullScreenQuad;
  private material: THREE.ShaderMaterial;
  private loading = "";
  private renderer: THREE.WebGLRenderer;

  constructor(renderer: THREE.WebGLRenderer) {
    this.renderer = renderer;
    this.target = new THREE.WebGLRenderTarget(1, 1, { samples: 4, type: THREE.HalfFloatType });
    this.material = new THREE.ShaderMaterial({
      vertexShader, fragmentShader, depthTest: false, depthWrite: false, toneMapped: false,
      uniforms: {
        tFig: { value: this.target.texture }, tBg: { value: null }, hasBg: { value: 0 }, exposure: { value: 1 },
        aspect: { value: 2 / 3 }, texel: { value: new THREE.Vector2(1, 1) },
        split: { value: 0 }, blur: { value: 0 }, vignette: { value: 0 }, saturation: { value: 1 }, tint: { value: new THREE.Color(0xffffff) },
      },
    });
    this.quad = new FullScreenQuad(this.material);
  }

  /** The painted backdrop under the figures. Until it loads, the canvas stays see-through over the page's picture. */
  setBackdrop(url: string): void {
    if (url === this.loading) return;
    this.loading = url;
    this.material.uniforms.hasBg!.value = 0;
    new THREE.TextureLoader().load(url, (tex) => {
      if (this.loading !== url) { tex.dispose(); return; }
      (this.material.uniforms.tBg!.value as THREE.Texture | null)?.dispose();
      tex.colorSpace = THREE.NoColorSpace; // composited as stored, the same as the <img> shows it
      tex.minFilter = THREE.LinearFilter; tex.generateMipmaps = false;
      this.material.uniforms.tBg!.value = tex;
      this.material.uniforms.hasBg!.value = 1;
    });
  }

  setLook(look: LensLook, exposure: number): void {
    const u = this.material.uniforms;
    u.split!.value = look.split; u.blur!.value = look.blur; u.vignette!.value = look.vignette;
    u.saturation!.value = look.saturation; (u.tint!.value as THREE.Color).set(look.tint);
    u.exposure!.value = exposure;
  }

  /** Drawing buffer size, pixels. */
  setSize(width: number, height: number): void {
    this.target.setSize(width, height);
    this.material.uniforms.aspect!.value = width / height;
    (this.material.uniforms.texel!.value as THREE.Vector2).set(1 / width, 1 / height);
  }

  render(scene: THREE.Scene, camera: THREE.Camera): void {
    const r = this.renderer;
    r.setRenderTarget(this.target);
    r.clear();
    r.render(scene, camera);
    r.setRenderTarget(null);
    this.quad.render(r);
  }
}
