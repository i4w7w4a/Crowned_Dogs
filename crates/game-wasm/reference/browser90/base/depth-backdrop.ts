import * as THREE from "three";
import { callRust } from "../rust/engine.ts";

/**
 * A screen's picture with depth (parallax occlusion): a grey depth map, white for near, says how far
 * each pixel moves when the camera does, so the picture shifts like a room and not like a flat sheet.
 * Drawn like object-fit: cover, zoomed a little so the moving edges never show.
 */
export class DepthBackdrop {
  private readonly renderer: THREE.WebGLRenderer;
  private readonly scene = new THREE.Scene();
  private readonly camera = new THREE.OrthographicCamera(-1, 1, 1, -1, 0, 1);
  private readonly material: THREE.ShaderMaterial;
  private readonly uniforms = {
    uColor: { value: null as THREE.Texture | null }, uDepth: { value: null as THREE.Texture | null },
    uCover: { value: new THREE.Vector2(1, 1) }, uShift: { value: new THREE.Vector2() },
  };
  private readonly canvas: HTMLCanvasElement;
  private readonly ratio: number;
  private readonly zoom: number;
  private width = 0;
  private height = 0;
  private last = "";
  /** Resolves once both pictures are on the card; the caller hides the flat picture then. */
  readonly ready: Promise<void>;

  constructor(canvas: HTMLCanvasElement, color: string, depth: string, ratio: number, zoom: number) {
    this.canvas = canvas; this.ratio = ratio; this.zoom = zoom;
    this.renderer = new THREE.WebGLRenderer({ canvas, alpha: false, antialias: false, powerPreference: "low-power" });
    this.renderer.outputColorSpace = THREE.LinearSRGBColorSpace; // the picture is passed through untouched
    const shader = callRust<{ vertex: string; fragment: string }>("graphics.backdropShader", {});
    this.material = new THREE.ShaderMaterial({
      uniforms: this.uniforms,
      vertexShader: shader.vertex,
      fragmentShader: shader.fragment,
      depthTest: false, depthWrite: false,
    });
    const quad = new THREE.Mesh(new THREE.PlaneGeometry(2, 2), this.material);
    quad.frustumCulled = false; this.scene.add(quad);
    const loader = new THREE.TextureLoader();
    const load = (url: string) => loader.loadAsync(url).then((t) => { t.colorSpace = THREE.NoColorSpace; t.wrapS = t.wrapT = THREE.ClampToEdgeWrapping; t.generateMipmaps = false; t.minFilter = THREE.LinearFilter; return t; });
    this.ready = Promise.all([load(color), load(depth)]).then(([c, d]) => {
      this.uniforms.uColor.value = c; this.uniforms.uDepth.value = d;
      this.last = ""; this.draw(0, 0);
    });
  }

  /** The camera's move as the nearest pixel sees it, in CSS pixels (right and down). */
  draw(x: number, y: number): void {
    const w = this.canvas.clientWidth, h = this.canvas.clientHeight;
    if (!w || !h || !this.uniforms.uDepth.value) return;
    const frame = callRust<{ cover: [number, number]; shift: [number, number]; dpr: number }>("graphics.backdrop", {
      width: w, height: h, ratio: this.ratio, zoom: this.zoom, x, y, dpr: globalThis.devicePixelRatio || 1,
    });
    if (w !== this.width || h !== this.height) {
      this.width = w; this.height = h; this.last = "";
      this.renderer.setPixelRatio(frame.dpr);
      this.renderer.setSize(w, h, false);
    }
    const key = `${x.toFixed(2)} ${y.toFixed(2)}`;
    if (key === this.last) return;
    this.last = key;
    // object-fit: cover, then the zoom; the picture's size on screen turns pixels into its own share
    this.uniforms.uCover.value.fromArray(frame.cover);
    this.uniforms.uShift.value.fromArray(frame.shift);
    this.renderer.render(this.scene, this.camera);
  }
}
