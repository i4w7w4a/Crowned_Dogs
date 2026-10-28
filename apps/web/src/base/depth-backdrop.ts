import * as THREE from "three";

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
    this.material = new THREE.ShaderMaterial({
      uniforms: this.uniforms,
      vertexShader: "varying vec2 vUv; void main() { vUv = uv; gl_Position = vec4(position.xy, 0., 1.); }",
      fragmentShader: `
        uniform sampler2D uColor, uDepth; uniform vec2 uCover, uShift; varying vec2 vUv;
        void main() {
          vec2 base = (vUv - .5) * uCover + .5, uv = base;
          // a few steps toward the pixel whose own depth lands it here
          for (int i = 0; i < 5; i++) uv = base + uShift * texture2D(uDepth, uv).r;
          gl_FragColor = texture2D(uColor, uv);
        }`,
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
    if (w !== this.width || h !== this.height) {
      this.width = w; this.height = h; this.last = "";
      this.renderer.setPixelRatio(Math.min(globalThis.devicePixelRatio || 1, 2));
      this.renderer.setSize(w, h, false);
    }
    const key = `${x.toFixed(2)} ${y.toFixed(2)}`;
    if (key === this.last) return;
    this.last = key;
    // object-fit: cover, then the zoom; the picture's size on screen turns pixels into its own share
    const fit = Math.max(w / this.ratio, h) * this.zoom, iw = fit * this.ratio, ih = fit;
    this.uniforms.uCover.value.set(w / iw, h / ih);
    this.uniforms.uShift.value.set(-x / iw, y / ih);
    this.renderer.render(this.scene, this.camera);
  }
}
