// The bodycam lens. The figures are drawn into off-screen pictures and laid over the
// painted backdrop here: the enemy sharp, our own figure (right in front of the lens)
// drawn small and so soft, out of focus. The whole frame then goes through one cheap
// wide lens: a slight fisheye bend, colours fanning out into rainbow fringes and the
// picture smearing and blurring towards the edges, and edges that sink into black
// corners. The middle, where the enemy stands, stays sharp. Painting and live figures
// go through the same glass.
//
// The graphics level (quality.ts) decides how clean the frame is drawn and how much
// light it gets: haze in the air with the figures' shadows in it, and shafts of light
// streaming from the bright openings of the painting.

import * as THREE from "three";
import { FullScreenQuad } from "three/addons/postprocessing/Pass.js";
import { composeShader, hazeShader, lensShader, raysBlurShader, raysMaskShader, vertexShader } from "./lens-shaders.ts";
import type { Quality } from "./quality.ts";

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
  /** Depth of field around the enemy: blur radius (share of the picture height) per 1/m behind him, and how much of it in front. */
  dof: number;
  dofNear: number;
  /** Light shafts from the openings, 0..1, and the haze in the air (density per metre, colour of its light). */
  rays: number;
  haze: number;
  hazeColor: number;
}

/** The visible part of the picture, as fractions of the canvas from its top left. */
export interface View { x: number; y: number; w: number; h: number }

/** What the haze needs from the scene each frame. */
export interface HazeScene {
  /** Distance to the enemy along the view, metres. */
  enemyDepth: number;
  /** The light whose shadows show in the haze. */
  light: THREE.DirectionalLight | null;
}

/** Our figure is drawn at this many pixels per CSS pixel divided by look.near: soft the same way on every screen. */
const NEAR_DPR = 2;

const shaderMaterial = (fragmentShader: string, uniforms: Record<string, THREE.IUniform>) => new THREE.ShaderMaterial({
  vertexShader, fragmentShader, uniforms, depthTest: false, depthWrite: false, toneMapped: false,
});

export class Lens {
  private renderer: THREE.WebGLRenderer;
  private q: Quality | null = null;
  private float: THREE.TextureDataType;
  /** The enemy, the floor shadows, sparks and dust. */
  private fig: THREE.WebGLRenderTarget;
  /** Our own figure drawn small: stretched back over the frame, it comes back soft. */
  private near: THREE.WebGLRenderTarget;
  /** The whole frame before the glass, with blurred copies for the smear and the glow. */
  private comp: THREE.WebGLRenderTarget;
  private haze: THREE.WebGLRenderTarget | null = null;
  private raysA: THREE.WebGLRenderTarget | null = null;
  private raysB: THREE.WebGLRenderTarget | null = null;
  private composeMat: THREE.ShaderMaterial;
  private hazeMat: THREE.ShaderMaterial;
  private maskMat: THREE.ShaderMaterial;
  private blurMat: THREE.ShaderMaterial;
  private lensMat: THREE.ShaderMaterial;
  private quad = new FullScreenQuad();
  private bg: THREE.Texture | null = null;
  private srcs: { one: string; two?: string; depth?: string } = { one: "" };
  private depthMap: THREE.Texture | null = null;
  private depthSrc = "";
  private loading = "";
  private look: LensLook | null = null;
  private view: View = { x: 0, y: 0, w: 1, h: 1 };
  private px = new THREE.Vector2(1, 1);
  private css = new THREE.Vector2(1, 1);
  /** Where the light comes from in the painting (texture coordinates, y up), found in its brightest opening. */
  private sun = new THREE.Vector2(0.8, 0.8);
  private horizon = 0.4;
  private clock = new THREE.Timer();

  constructor(renderer: THREE.WebGLRenderer) {
    this.renderer = renderer;
    const ext = renderer.extensions;
    // devices that cannot draw into half floats still get the picture, with less room for highlights
    this.float = ext.has("EXT_color_buffer_float") || ext.has("EXT_color_buffer_half_float") ? THREE.HalfFloatType : THREE.UnsignedByteType;
    this.fig = new THREE.WebGLRenderTarget(1, 1, { samples: 4, type: this.float });
    this.near = new THREE.WebGLRenderTarget(1, 1, { samples: 4, type: this.float });
    this.comp = new THREE.WebGLRenderTarget(1, 1, { type: this.float, generateMipmaps: true, minFilter: THREE.LinearMipmapLinearFilter });
    const v2 = () => ({ value: new THREE.Vector2(1, 1) });
    this.composeMat = shaderMaterial(composeShader, {
      tFig: { value: this.fig.texture }, tNear: { value: this.near.texture }, tBg: { value: null }, tHaze: { value: null },
      tDepth: { value: null }, hasDepth: { value: 0 }, focusInv: { value: 1 / 8 }, dof: { value: 0 }, dofNear: { value: 0 },
      bgSize: v2(), nearSize: v2(), hazeSize: v2(), hasBg: { value: 0 }, exposure: { value: 1 }, saturation: { value: 1 }, tint: { value: new THREE.Color(0xffffff) },
    });
    this.hazeMat = shaderMaterial(hazeShader, {
      invProj: { value: new THREE.Matrix4() }, camWorld: { value: new THREE.Matrix4() }, camPos: { value: new THREE.Vector3() },
      tFig: { value: this.fig.texture }, enemyDist: { value: 10 }, tDepth: { value: null }, hasDepth: { value: 0 }, tShadow: { value: null }, shadowMatrix: { value: new THREE.Matrix4() }, hasShadow: { value: 0 },
      sunDir: { value: new THREE.Vector3(0, 0.5, -1).normalize() }, sunColor: { value: new THREE.Color() }, skyColor: { value: new THREE.Color() },
      density: { value: 0 }, time: { value: 0 },
    });
    this.maskMat = shaderMaterial(raysMaskShader, {
      tComp: { value: this.comp.texture }, tFig: { value: this.fig.texture }, lod: { value: 1 }, horizon: { value: 0.6 }, threshold: { value: 0.72 },
    });
    this.blurMat = shaderMaterial(raysBlurShader, { tSrc: { value: null }, light: { value: this.sun }, reach: { value: 0.9 }, decay: { value: 0.97 } });
    this.lensMat = shaderMaterial(lensShader, {
      tComp: { value: this.comp.texture }, tRays: { value: null }, compSize: v2(), outSize: v2(), ssaa: { value: 1 }, hasBg: { value: 0 },
      rays: { value: 0 }, glowLod: { value: 0 },
      center: { value: new THREE.Vector2(0.5, 0.5) }, halfView: { value: new THREE.Vector2(0.5, 0.5) }, aspect: { value: 2 / 3 },
      smear: { value: 0 }, soft: { value: 0 }, barrel: { value: 0 }, vignette: { value: 0 }, glow: { value: 0 }, contrast: { value: 0 },
    });
  }

  /** The painted backdrop under the figures, its double-size copy for the sharp levels, and its depth map. Until it loads, the canvas stays see-through over the page's picture. */
  setBackdrop(one: string, two?: string, depth?: string): void {
    this.srcs = { one, two, depth };
    this.loadBackdrop();
    this.loadDepth();
  }

  /** The painting's distances: 1/z in metres, 0..1 for 0..1/0.8 m, so the blur and the haze know what is near and far. */
  private loadDepth(): void {
    const url = this.srcs.depth ?? "";
    if (url === this.depthSrc) return;
    this.depthSrc = url;
    const set = (t: THREE.Texture | null) => {
      this.depthMap?.dispose();
      this.depthMap = t;
      for (const m of [this.composeMat, this.hazeMat]) { m.uniforms.tDepth!.value = t; m.uniforms.hasDepth!.value = t ? 1 : 0; }
    };
    if (!url) { set(null); return; }
    new THREE.TextureLoader().load(url, (tex) => {
      if (this.depthSrc !== url) { tex.dispose(); return; }
      tex.colorSpace = THREE.NoColorSpace;
      tex.generateMipmaps = false;
      tex.minFilter = THREE.LinearFilter;
      set(tex);
    });
  }

  /** How far down the picture the horizon is (share from the top): only the sky and the windows above it shine shafts of light. */
  setHorizon(fromTop: number): void {
    this.horizon = fromTop;
  }

  setLook(look: LensLook, exposure: number): void {
    this.look = look;
    this.sizeTargets();
    const u = this.lensMat.uniforms, c = this.composeMat.uniforms;
    u.smear!.value = look.smear; u.soft!.value = look.soft; u.barrel!.value = look.barrel; u.vignette!.value = look.vignette;
    u.glow!.value = look.glow; u.contrast!.value = look.contrast;
    c.saturation!.value = look.saturation; (c.tint!.value as THREE.Color).set(look.tint);
    c.exposure!.value = exposure;
    c.dof!.value = look.dof; c.dofNear!.value = look.dofNear;
    const h = this.hazeMat.uniforms, col = new THREE.Color(look.hazeColor);
    (h.sunColor!.value as THREE.Color).copy(col).multiplyScalar(0.55);
    (h.skyColor!.value as THREE.Color).copy(col).multiplyScalar(0.16);
    h.density!.value = look.haze;
  }

  /** The graphics level: how clean the frame is drawn and which light it gets. */
  setQuality(q: Quality): void {
    const was = this.q;
    this.q = q;
    if (!was || was.msaa !== q.msaa) {
      this.fig.samples = q.msaa; this.fig.dispose();
      this.near.samples = Math.min(4, q.msaa); this.near.dispose();
    }
    if (q.haze && !this.haze) this.haze = new THREE.WebGLRenderTarget(1, 1, { type: this.float, depthBuffer: false });
    if (!q.haze && this.haze) { this.haze.dispose(); this.haze = null; }
    if (q.rays && !this.raysA) {
      this.raysA = new THREE.WebGLRenderTarget(1, 1, { type: this.float, depthBuffer: false });
      this.raysB = new THREE.WebGLRenderTarget(1, 1, { type: this.float, depthBuffer: false });
    }
    if (!q.rays && this.raysA) { this.raysA.dispose(); this.raysB!.dispose(); this.raysA = this.raysB = null; }

    const set = (m: THREE.ShaderMaterial, defines: Record<string, string | number | boolean>) => {
      for (const k of Object.keys(m.defines)) if (!(k in defines)) delete m.defines[k];
      Object.assign(m.defines, defines);
      m.needsUpdate = true;
    };
    set(this.composeMat, { DOF_TAPS: q.dofTaps, ...(q.sharpBackdrop ? { SHARP_BG: 1 } : {}), ...(q.haze ? { HAZE: 1 } : {}) });
    set(this.lensMat, { TAPS: q.taps, ...(q.rays ? { RAYS: 1 } : {}) });
    set(this.blurMat, { STEPS: Math.max(1, q.rays) });
    set(this.hazeMat, { STEPS: Math.max(1, q.haze), OCTAVES: q.haze >= 48 ? 3 : 2 });
    this.composeMat.uniforms.tHaze!.value = this.haze?.texture ?? null;
    this.lensMat.uniforms.tRays!.value = this.raysA?.texture ?? null;
    this.loadBackdrop();
    this.sizeTargets();
  }

  /** Drawing buffer size in pixels, its size in CSS pixels, and the part of the canvas the screen shows (the lens centres on it). */
  setSize(width: number, height: number, cssWidth: number, cssHeight: number, view: View): void {
    this.px.set(width, height);
    this.css.set(cssWidth, cssHeight);
    this.view = view;
    this.sizeTargets();
    const u = this.lensMat.uniforms;
    (u.center!.value as THREE.Vector2).set(view.x + view.w / 2, 1 - (view.y + view.h / 2));
    (u.halfView!.value as THREE.Vector2).set(view.w / 2, view.h / 2);
    u.aspect!.value = (view.w * width) / (view.h * height);
  }

  /** Where a point of the undistorted picture shows after the fisheye bend (canvas fractions from the top left). */
  toScreen(x: number, y: number): [number, number] {
    const v = this.view, k = this.look?.barrel ?? 0;
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

  /** Draws the scene (layer 0), our own figure (layer `near`) on its own, the haze and the shafts, then the lens to the screen. */
  render(scene: THREE.Scene, camera: THREE.Camera, near: number, haze: HazeScene): void {
    const r = this.renderer, mask = camera.layers.mask, autoShadows = r.shadowMap.autoUpdate;
    const look = this.look, q = this.q, hasBg = this.bg ? 1 : 0;
    this.clock.update();
    camera.layers.set(0);
    r.setRenderTarget(this.fig);
    r.clear();
    r.render(scene, camera);
    camera.layers.set(near);
    r.shadowMap.autoUpdate = false; // the shadows of this frame are drawn already
    r.setRenderTarget(this.near);
    r.clear();
    r.render(scene, camera);
    r.shadowMap.autoUpdate = autoShadows;
    camera.layers.mask = mask;

    if (this.haze && look && hasBg) {
      const h = this.hazeMat.uniforms;
      (h.invProj!.value as THREE.Matrix4).copy(camera.projectionMatrixInverse);
      (h.camWorld!.value as THREE.Matrix4).copy(camera.matrixWorld);
      (h.camPos!.value as THREE.Vector3).setFromMatrixPosition(camera.matrixWorld);
      h.enemyDist!.value = haze.enemyDepth;
      h.time!.value = this.clock.getElapsed();
      // the haze glows towards the opening the shafts come from
      this.sunDirection(camera, h.sunDir!.value as THREE.Vector3);
      const shadow = haze.light?.castShadow ? haze.light.shadow : null;
      h.hasShadow!.value = shadow?.map?.depthTexture && r.shadowMap.enabled ? 1 : 0;
      h.tShadow!.value = h.hasShadow!.value ? shadow!.map!.depthTexture : null;
      if (shadow) (h.shadowMatrix!.value as THREE.Matrix4).copy(shadow.matrix);
      this.pass(this.hazeMat, this.haze);
    }

    // the focus follows the enemy as he walks
    this.composeMat.uniforms.focusInv!.value = 1 / Math.max(0.5, haze.enemyDepth);
    this.pass(this.composeMat, this.comp);

    if (this.raysA && this.raysB && look && hasBg) {
      this.maskMat.uniforms.horizon!.value = 1 - this.horizon;
      this.pass(this.maskMat, this.raysA);
      const b = this.blurMat.uniforms;
      // long steps first, then short ones across the gaps between them
      b.tSrc!.value = this.raysA.texture; b.reach!.value = 0.92; b.decay!.value = 0.975;
      this.pass(this.blurMat, this.raysB);
      b.tSrc!.value = this.raysB.texture; b.reach!.value = 0.92 / Math.max(1, q!.rays) * 1.6; b.decay!.value = 1;
      this.pass(this.blurMat, this.raysA);
      this.lensMat.uniforms.rays!.value = look.rays;
    }

    this.lensMat.uniforms.hasBg!.value = hasBg;
    this.pass(this.lensMat, null);
  }

  /** The direction in the world towards the painting's light opening. */
  sunDirection(camera: THREE.Camera, out: THREE.Vector3): THREE.Vector3 {
    return out.set(this.sun.x * 2 - 1, this.sun.y * 2 - 1, 1).applyMatrix4(camera.projectionMatrixInverse).transformDirection(camera.matrixWorld);
  }

  /** The light shafts of the last frame (screen-sized), or null on levels without them. */
  get raysTexture(): THREE.Texture | null {
    return this.raysA?.texture ?? null;
  }

  private pass(m: THREE.ShaderMaterial, target: THREE.WebGLRenderTarget | null): void {
    this.quad.material = m;
    this.renderer.setRenderTarget(target);
    this.quad.render(this.renderer);
  }

  private loadBackdrop(): void {
    const url = this.q?.sharpBackdrop && this.srcs.two ? this.srcs.two : this.srcs.one;
    if (!url || url === this.loading) return;
    this.loading = url;
    new THREE.TextureLoader().load(url, (tex) => {
      if (this.loading !== url) { tex.dispose(); return; }
      this.bg?.dispose();
      tex.colorSpace = THREE.NoColorSpace; // composited as stored, the same as the <img> shows it
      tex.minFilter = THREE.LinearMipmapLinearFilter;
      tex.anisotropy = 4;
      this.bg = tex;
      this.findSun(tex.image as HTMLImageElement);
      const c = this.composeMat.uniforms;
      c.tBg!.value = tex; c.hasBg!.value = 1;
      (c.bgSize!.value as THREE.Vector2).set(tex.image.width, tex.image.height);
    });
  }

  /** The light comes from the brightest opening above the horizon: the middle of the near-white sky in the painting. */
  private findSun(img: HTMLImageElement): void {
    const w = 48, h = Math.round(48 * img.height / img.width);
    const c = document.createElement("canvas"); c.width = w; c.height = h;
    const g = c.getContext("2d", { willReadFrequently: true });
    if (!g) return;
    g.drawImage(img, 0, 0, w, h);
    const px = g.getImageData(0, 0, w, h).data;
    let sx = 0, sy = 0, sw = 0;
    for (let y = 0; y < Math.round(h * this.horizon); y++) for (let x = 0; x < w; x++) {
      const i = (y * w + x) * 4, l = (0.2126 * px[i]! + 0.7152 * px[i + 1]! + 0.0722 * px[i + 2]!) / 255;
      const wt = Math.max(0, l - 0.72) ** 2;
      sx += wt * (x + 0.5); sy += wt * (y + 0.5); sw += wt;
    }
    if (sw > 0) this.sun.set(sx / sw / w, 1 - sy / sw / h);
  }

  private sizeTargets(): void {
    const q = this.q, look = this.look;
    if (!q) return;
    const max = Math.min(this.renderer.capabilities.maxTextureSize, 8192);
    const ss = Math.min(q.ssaa, max / Math.max(this.px.x, this.px.y));
    const w = Math.max(1, Math.round(this.px.x * ss)), h = Math.max(1, Math.round(this.px.y * ss));
    this.fig.setSize(w, h);
    this.comp.setSize(w, h);
    const nk = NEAR_DPR / Math.max(1, look?.near ?? 1);
    const nw = Math.max(1, Math.round(Math.min(this.px.x, this.css.x * nk))), nh = Math.max(1, Math.round(Math.min(this.px.y, this.css.y * nk)));
    this.near.setSize(nw, nh);
    (this.composeMat.uniforms.nearSize!.value as THREE.Vector2).set(nw, nh);
    if (this.haze) {
      const hw = Math.max(1, Math.round(this.px.x * q.hazeScale)), hh = Math.max(1, Math.round(this.px.y * q.hazeScale));
      this.haze.setSize(hw, hh);
      (this.composeMat.uniforms.hazeSize!.value as THREE.Vector2).set(hw, hh);
    }
    if (this.raysA && this.raysB) {
      const rw = Math.max(1, Math.round(this.px.x * q.raysScale)), rh = Math.max(1, Math.round(this.px.y * q.raysScale));
      this.raysA.setSize(rw, rh); this.raysB.setSize(rw, rh);
      this.maskMat.uniforms.lod!.value = Math.max(0, Math.log2(w / rw));
    }
    const u = this.lensMat.uniforms;
    (u.compSize!.value as THREE.Vector2).set(w, h);
    (u.outSize!.value as THREE.Vector2).set(this.px.x, this.px.y);
    u.ssaa!.value = w / this.px.x;
    u.glowLod!.value = Math.log2(w / 941);
  }
}
