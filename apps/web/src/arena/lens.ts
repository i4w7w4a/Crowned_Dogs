// The bodycam lens. The figures are drawn into off-screen pictures and laid over the
// painted backdrop here: the enemy sharp, our own figure (right in front of the lens)
// drawn small and so soft, out of focus. The whole frame then goes through one cheap
// wide lens: a slight fisheye bend, colours fanning out into rainbow fringes and the
// picture smearing and blurring towards the edges, and edges that sink into black
// corners. The middle, where the enemy stands, stays sharp. Painting and live figures
// go through the same glass.

import * as THREE from "three";
import { FullScreenQuad } from "three/addons/postprocessing/Pass.js";

export interface LensLook {
  /** Radial smear and colour fan at the edges, as a share of the distance from the centre. */
  smear: number;
  /** Soft blur in the corners, as a share of the picture's height. */
  soft: number;
  /** Fisheye bend: how much further out the corners are sampled. */
  barrel: number;
  /** How much the edges darken, 0..1: the corners go to black at 1. */
  vignette: number;
  /** Glow bleeding from the bright windows and sky over everything near them. */
  glow: number;
  /** Extra contrast of the whole frame, 0..1. */
  contrast: number;
  /** How soft our own figure is: it is drawn at 1/near of the size. */
  near: number;
  /** Colour of the live figures against the painting: saturation (1 = as lit) and a tint. */
  saturation: number;
  tint: number;
}

/** The visible part of the picture, as fractions of the canvas from its top left. */
export interface View { x: number; y: number; w: number; h: number }

const TAPS = 12;

const vertexShader = /* glsl */ `
varying vec2 vUv;
void main() { vUv = uv; gl_Position = vec4(position.xy, 0.0, 1.0); }`;

const fragmentShader = /* glsl */ `
#define TAPS ${TAPS}
uniform sampler2D tFig;
uniform sampler2D tNear;
uniform sampler2D tBg;
uniform float hasBg;
uniform float exposure;
uniform vec2 center;
uniform vec2 halfView;
uniform float aspect;
uniform float smear;
uniform float soft;
uniform float barrel;
uniform float vignette;
uniform float glow;
uniform float contrast;
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

// a drawn layer (linear, premultiplied) toned and graded like the painting, still premultiplied
vec3 tone(vec4 f) {
  vec3 lin = f.a > 0.001 ? f.rgb / f.a : f.rgb;
  vec3 c = toSRGB(aces(lin));
  c = mix(vec3(dot(c, vec3(0.2126, 0.7152, 0.0722))), c, saturation) * tint;
  return f.a > 0.001 ? c * f.a : c; // sparks add light without covering
}

// the figures over the painting (sRGB as stored), the way the browser would lay a transparent
// canvas over a picture; our own figure last
vec4 frame(vec2 uv) {
  vec4 f = texture2D(tFig, uv);
  vec3 bg = hasBg > 0.5 ? texture2D(tBg, uv, -1.0).rgb : vec3(0.0); // the sharp picture, not its blurred copies
  vec3 c = tone(f) + bg * (1.0 - f.a);
  vec4 n = texture2D(tNear, uv);
  c = tone(n) + c * (1.0 - n.a);
  return vec4(c, hasBg > 0.5 ? 1.0 : max(f.a, n.a));
}

// red through green to blue as t goes 0..1: each tap of the smear lands in its own colour
vec3 spectrum(float t) {
  float w = clamp(1.0 - abs(2.0 * clamp((t - 1.0 / 6.0) / (4.0 / 6.0), 0.0, 1.0) - 1.0), 0.0, 1.0);
  float lo = step(t, 0.5);
  return vec3(lo, 1.0, 1.0 - lo) * vec3(1.0 - w, w, 1.0 - w);
}

void main() {
  vec2 d = (vUv - center) / halfView; // -1..1 across the visible part
  vec2 e = d * vec2(aspect, 1.0) / length(vec2(aspect, 1.0));
  vec2 src = center + (vUv - center) * (1.0 + barrel * dot(e, e)); // the bend, round like the glass
  float q = 0.5 * dot(d, d); // the frame's own oval: 0.5 on the middle of each edge, 1 in the corners
  float amount = smear * smoothstep(0.08, 1.0, q);
  float r = soft * smoothstep(0.15, 1.0, q);
  vec4 c;
  if (amount < 0.0004 && r < 0.0004) {
    c = frame(src);
  } else {
    // a different start in every pixel turns the steps between taps into fine grain
    float jit = fract(52.9829189 * fract(dot(gl_FragCoord.xy, vec2(0.06711056, 0.00583715))));
    vec2 ring = vec2(halfView.x / aspect, halfView.y) * 2.0 * r; // round in pixels
    vec3 sum = vec3(0.0), wsum = vec3(0.0);
    float a = 0.0;
    for (int i = 0; i < TAPS; i++) {
      float t = (float(i) + jit) / float(TAPS);
      float ang = 6.2831853 * fract(jit * 7.31 + float(i) * 0.618034);
      vec2 o = vec2(cos(ang), sin(ang)) * ring * sqrt(fract(t * 3.0 + 0.37));
      vec3 w = spectrum(t);
      vec4 s = frame(center + (src - center) * (1.0 + amount * (t - 0.5) * 2.0) + o); // red spreads outwards
      sum += s.rgb * w; wsum += w; a += s.a;
    }
    c = vec4(sum / wsum, a / float(TAPS));
  }
  // the bright parts of the painting bleed light: two blurred copies, only what is near white
  vec3 halo = max(texture2D(tBg, src, 4.5).rgb - 0.55, 0.0) + max(texture2D(tBg, src, 6.0).rgb - 0.5, 0.0);
  c.rgb += halo * glow * hasBg;
  c.rgb = mix(c.rgb, c.rgb * c.rgb * (3.0 - 2.0 * c.rgb), contrast * hasBg);
  float v = smoothstep(0.45, 1.0, q);
  float dark = 1.0 - vignette * v * sqrt(v); // bright almost to the edge, then sinking fast into black corners
  gl_FragColor = vec4(clamp(c.rgb, 0.0, 1.0) * dark, c.a);
}`;

export class Lens {
  private target: THREE.WebGLRenderTarget;
  /** Our own figure drawn small: stretched back over the frame, it comes back soft. */
  private nearTarget: THREE.WebGLRenderTarget;
  private quad: FullScreenQuad;
  private material: THREE.ShaderMaterial;
  private loading = "";
  private renderer: THREE.WebGLRenderer;
  private look: LensLook = { smear: 0, soft: 0, barrel: 0, vignette: 0, glow: 0, contrast: 0, near: 1, saturation: 1, tint: 0xffffff };
  private view: View = { x: 0, y: 0, w: 1, h: 1 };
  private px = new THREE.Vector2(1, 1);

  constructor(renderer: THREE.WebGLRenderer) {
    this.renderer = renderer;
    this.target = new THREE.WebGLRenderTarget(1, 1, { samples: 4, type: THREE.HalfFloatType });
    this.nearTarget = new THREE.WebGLRenderTarget(1, 1, { type: THREE.HalfFloatType });
    this.material = new THREE.ShaderMaterial({
      vertexShader, fragmentShader, depthTest: false, depthWrite: false, toneMapped: false,
      uniforms: {
        tFig: { value: this.target.texture }, tNear: { value: this.nearTarget.texture }, tBg: { value: null }, hasBg: { value: 0 }, exposure: { value: 1 },
        center: { value: new THREE.Vector2(0.5, 0.5) }, halfView: { value: new THREE.Vector2(0.5, 0.5) }, aspect: { value: 2 / 3 },
        smear: { value: 0 }, soft: { value: 0 }, barrel: { value: 0 }, vignette: { value: 0 }, glow: { value: 0 }, contrast: { value: 0 }, saturation: { value: 1 }, tint: { value: new THREE.Color(0xffffff) },
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
      tex.minFilter = THREE.LinearMipmapLinearFilter; // the small copies give the glow
      this.material.uniforms.tBg!.value = tex;
      this.material.uniforms.hasBg!.value = 1;
    });
  }

  setLook(look: LensLook, exposure: number): void {
    this.look = look;
    this.sizeNear();
    const u = this.material.uniforms;
    u.smear!.value = look.smear; u.soft!.value = look.soft; u.barrel!.value = look.barrel; u.vignette!.value = look.vignette;
    u.glow!.value = look.glow; u.contrast!.value = look.contrast;
    u.saturation!.value = look.saturation; (u.tint!.value as THREE.Color).set(look.tint);
    u.exposure!.value = exposure;
  }

  /** Drawing buffer size in pixels, and the part of the canvas the screen shows (the lens centres on it). */
  setSize(width: number, height: number, view: View): void {
    this.target.setSize(width, height);
    this.px.set(width, height);
    this.sizeNear();
    this.view = view;
    const u = this.material.uniforms;
    (u.center!.value as THREE.Vector2).set(view.x + view.w / 2, 1 - (view.y + view.h / 2));
    (u.halfView!.value as THREE.Vector2).set(view.w / 2, view.h / 2);
    u.aspect!.value = (view.w * width) / (view.h * height);
  }

  /** Where a point of the undistorted picture shows after the fisheye bend (canvas fractions from the top left). */
  toScreen(x: number, y: number): [number, number] {
    const v = this.view, k = this.look.barrel;
    if (!k) return [x, y];
    const cx = v.x + v.w / 2, cy = v.y + v.h / 2, hx = v.w / 2, hy = v.h / 2;
    const aspect = (v.w * this.px.x) / (v.h * this.px.y), n = Math.hypot(aspect, 1);
    // the shader samples s = c + (p - c)(1 + k r²(p)); solve for p a few times over
    let px = x, py = y;
    for (let i = 0; i < 4; i++) {
      const ex = ((px - cx) / hx) * aspect / n, ey = ((py - cy) / hy) / n;
      const f = 1 + k * (ex * ex + ey * ey);
      px = cx + (x - cx) / f; py = cy + (y - cy) / f;
    }
    return [px, py];
  }

  /** Draws the scene (layer 0), then our own figure (layer `near`) on its own, then the lens to the screen. */
  render(scene: THREE.Scene, camera: THREE.Camera, near: number): void {
    const r = this.renderer, mask = camera.layers.mask, autoShadows = r.shadowMap.autoUpdate;
    camera.layers.set(0);
    r.setRenderTarget(this.target);
    r.clear();
    r.render(scene, camera);
    camera.layers.set(near);
    r.shadowMap.autoUpdate = false; // the shadows of this frame are drawn already
    r.setRenderTarget(this.nearTarget);
    r.clear();
    r.render(scene, camera);
    r.shadowMap.autoUpdate = autoShadows;
    camera.layers.mask = mask;
    r.setRenderTarget(null);
    this.quad.render(r);
  }

  private sizeNear(): void {
    const k = Math.max(1, this.look.near);
    this.nearTarget.setSize(Math.max(1, Math.round(this.px.x / k)), Math.max(1, Math.round(this.px.y / k)));
  }
}
