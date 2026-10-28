import * as THREE from "three";
import { GLTFLoader } from "three/addons/loaders/GLTFLoader.js";
import { RoomEnvironment } from "three/addons/environments/RoomEnvironment.js";
import { liquidGlass } from "../hud/liquid.ts";
import type { LoadingPresentation } from "./battle-loading.ts";
import "./maw-loader.css";

const base = import.meta.env.BASE_URL;
const d2r = Math.PI / 180;

/** The approved brand loader, driven by real arena readiness instead of prototype timestamps. */
export class MawLoader implements LoadingPresentation {
  private readonly dialog = document.createElement("dialog");
  private readonly stage: HTMLElement;
  private readonly canvas: HTMLCanvasElement;
  private readonly status: HTMLElement;
  private readonly retry: HTMLButtonElement;
  private readonly back: HTMLButtonElement;
  private readonly reduced = matchMedia("(prefers-reduced-motion: reduce)").matches;
  private observer: ResizeObserver;
  private renderer: THREE.WebGLRenderer | null = null;
  private scene: THREE.Scene | null = null;
  private camera: THREE.PerspectiveCamera | null = null;
  private environment: THREE.WebGLRenderTarget | null = null;
  private pivot: THREE.Group | null = null;
  private mixer: THREE.AnimationMixer | null = null;
  private idle: THREE.AnimationAction | null = null;
  private radius = 1.6;
  private generation = 0;
  private shown = false;
  private pending: Promise<void> | null = null;
  private lastFrame = 0;
  private elapsed = 0;
  private finishTimer: ReturnType<typeof setTimeout> | null = null;
  private finishResolve: (() => void) | null = null;

  constructor() {
    this.dialog.className = "cd-maw-loader";
    this.dialog.setAttribute("aria-label", "Загрузка боя");
    this.dialog.innerHTML = `<div class="cd-maw-inner">
      <div class="cd-maw-stage" aria-hidden="true"><img class="cd-maw-fallback" src="${base}brand/maw.svg" alt=""><canvas></canvas></div>
      <div class="cd-maw-hud"><h1 class="cd-maw-word"><img src="${base}brand/wordmark.svg" alt="CROWNED DOGS"></h1><p class="cd-maw-tag">PMC · 2051</p>
        <div class="cd-maw-progress" role="progressbar" aria-label="Загрузка арены"></div>
        <p class="cd-maw-status" role="status">Загрузка арены…</p>
        <div class="cd-maw-actions"><button class="glass cd-maw-retry" type="button" hidden>Повторить</button><button class="glass cd-maw-back" type="button">На базу</button></div>
      </div></div>`;
    this.stage = this.dialog.querySelector(".cd-maw-stage")!;
    this.canvas = this.dialog.querySelector("canvas")!;
    this.status = this.dialog.querySelector(".cd-maw-status")!;
    this.retry = this.dialog.querySelector(".cd-maw-retry")!;
    this.back = this.dialog.querySelector(".cd-maw-back")!;
    this.dialog.addEventListener("cancel", (event) => { event.preventDefault(); this.back.click(); });
    this.canvas.addEventListener("webglcontextlost", (event) => { event.preventDefault(); this.stage.classList.remove("is-3d"); this.stopAnimation(); });
    this.canvas.addEventListener("webglcontextrestored", () => { if (this.shown && this.pivot) { this.stage.classList.add("is-3d"); this.startAnimation(); } });
    document.body.appendChild(this.dialog);
    liquidGlass(this.dialog);
    this.observer = new ResizeObserver(() => this.resize());
    this.observer.observe(this.stage);
    document.addEventListener("visibilitychange", this.visibility);
  }

  show(onBack: () => void): void {
    this.shown = true;
    this.dialog.classList.remove("is-ready", "is-error");
    this.retry.hidden = true;
    this.status.textContent = "Загрузка арены…";
    this.dialog.setAttribute("aria-busy", "true");
    this.back.onclick = onBack;
    if (!this.dialog.open) this.dialog.showModal();
    this.back.focus({ preventScroll: true });
    if (this.pivot) { this.resize(); this.startAnimation(); }
    else if (this.pending) {
      // A previous visit may still be fetching after cancellation. Restart once it settles.
      void this.pending.then(() => { if (this.shown && !this.renderer) this.beginMaw(); });
    } else this.beginMaw();
  }

  ready(): Promise<void> {
    this.dialog.setAttribute("aria-busy", "false");
    this.status.textContent = "Арена готова";
    this.dialog.classList.add("is-ready");
    if (this.reduced || !this.shown) { this.hide(); return Promise.resolve(); }
    return new Promise((resolve) => {
      this.finishResolve = resolve;
      this.finishTimer = setTimeout(() => { this.finishTimer = null; this.finishResolve = null; this.hide(); resolve(); }, 360);
    });
  }

  failed(): Promise<"retry" | "back"> {
    this.dialog.classList.remove("is-ready");
    this.dialog.classList.add("is-error");
    this.dialog.setAttribute("aria-busy", "false");
    this.status.textContent = "Не удалось загрузить арену";
    this.retry.hidden = false;
    this.stopAnimation();
    this.retry.focus({ preventScroll: true });
    return new Promise((resolve) => {
      this.retry.onclick = () => resolve("retry");
      this.back.onclick = () => resolve("back");
    });
  }

  hide(): void {
    this.shown = false;
    this.stopAnimation();
    if (this.finishTimer) clearTimeout(this.finishTimer);
    this.finishTimer = null;
    this.finishResolve?.(); this.finishResolve = null;
    if (this.dialog.open) this.dialog.close();
    this.release();
  }

  private beginMaw(): void {
    if (!this.pending) this.pending = this.loadMaw().finally(() => { this.pending = null; });
  }

  private async loadMaw(): Promise<void> {
    const generation = ++this.generation;
    try {
      this.renderer = new THREE.WebGLRenderer({ canvas: this.canvas, antialias: true, alpha: true });
      this.renderer.setPixelRatio(Math.min(devicePixelRatio, 2));
      this.renderer.toneMapping = THREE.ACESFilmicToneMapping;
      this.renderer.toneMappingExposure = .95;
      this.renderer.shadowMap.enabled = true;
      this.renderer.shadowMap.type = THREE.PCFSoftShadowMap;
      this.scene = new THREE.Scene();
      const pmrem = new THREE.PMREMGenerator(this.renderer), room = new RoomEnvironment();
      this.environment = pmrem.fromScene(room, .04);
      this.scene.environment = this.environment.texture;
      this.scene.environmentIntensity = .2;
      room.dispose(); pmrem.dispose();
      this.camera = new THREE.PerspectiveCamera(24, 1, .1, 60);
      const key = new THREE.DirectionalLight(0xffeedd, 3.6);
      key.position.set(-4.2, 4.2, 2.6); key.castShadow = true;
      key.shadow.mapSize.set(1024, 1024);
      key.shadow.camera.left = key.shadow.camera.bottom = -2.2;
      key.shadow.camera.right = key.shadow.camera.top = 2.2;
      key.shadow.bias = -.0004; key.shadow.normalBias = .02;
      const rim = new THREE.DirectionalLight(0xc8dcff, 4.2); rim.position.set(4, 2.2, -4.5);
      const under = new THREE.DirectionalLight(0xffffff, .35); under.position.set(1, -4, 2.5);
      this.scene.add(key, rim, under);

      const response = await fetch(`${base}brand/maw.json`);
      if (!response.ok) throw new Error("Maw asset unavailable");
      const data: { data: string } = await response.json();
      const binary = Uint8Array.from(atob(data.data), (char) => char.charCodeAt(0));
      const gltf = await new GLTFLoader().parseAsync(binary.buffer, "");
      if (generation !== this.generation || !this.shown) { disposeObject(gltf.scene); return; }
      const model = gltf.scene;
      model.traverse((object) => { if (object instanceof THREE.Mesh) object.castShadow = object.receiveShadow = true; });
      const bounds = new THREE.Box3().setFromObject(model);
      model.position.sub(bounds.getCenter(new THREE.Vector3()));
      this.radius = bounds.getBoundingSphere(new THREE.Sphere()).radius * .78;
      this.pivot = new THREE.Group(); this.pivot.add(model); this.scene.add(this.pivot);
      const idleClip = THREE.AnimationClip.findByName(gltf.animations, "Idle");
      if (idleClip) { this.mixer = new THREE.AnimationMixer(model); this.idle = this.mixer.clipAction(idleClip); this.idle.play(); }
      this.stage.classList.add("is-3d");
      this.resize(); this.startAnimation();
    } catch { if (generation === this.generation) this.release(); /* The immediate SVG remains usable. */ }
  }

  private resize(): void {
    if (!this.renderer || !this.camera) return;
    const width = this.stage.clientWidth, height = this.stage.clientHeight;
    if (!width || !height) return;
    this.renderer.setSize(width, height, false);
    this.camera.aspect = width / height;
    const vfov = this.camera.fov * d2r / 2, hfov = Math.atan(Math.tan(vfov) * this.camera.aspect);
    this.camera.position.set(0, 0, this.radius / Math.sin(Math.min(vfov, hfov)) * 1.02);
    this.camera.lookAt(0, 0, 0); this.camera.updateProjectionMatrix();
    this.draw(0);
  }

  private readonly visibility = (): void => { if (document.hidden) this.stopAnimation(); else if (this.shown) this.startAnimation(); };
  private stopAnimation(): void { this.renderer?.setAnimationLoop(null); this.lastFrame = 0; }
  private startAnimation(): void {
    if (!this.shown || document.hidden || !this.renderer || !this.pivot) return;
    this.draw(0);
    if (this.reduced) return;
    this.renderer.setAnimationLoop((time) => {
      const dt = this.lastFrame ? Math.min((time - this.lastFrame) / 1000, .05) : 0;
      this.lastFrame = time; this.draw(dt);
    });
  }
  private draw(dt: number): void {
    if (!this.renderer || !this.scene || !this.camera || !this.pivot) return;
    this.elapsed += dt; this.mixer?.update(dt);
    const t = this.elapsed, sway = this.reduced ? 0 : 1;
    this.pivot.rotation.set((16 + sway * 2 * Math.sin(t * .7)) * d2r, (-22 + sway * 9 * Math.sin(t * .45)) * d2r, (-13 + sway * 1.5 * Math.cos(t * .33)) * d2r, "ZXY");
    const breath = this.idle ? .5 - .5 * Math.cos(2 * Math.PI * (this.idle.time % 4) / 4) : 0;
    this.pivot.scale.setScalar(1 + .012 * breath);
    this.renderer.render(this.scene, this.camera);
  }
  private release(): void {
    ++this.generation;
    this.stopAnimation();
    this.stage.classList.remove("is-3d");
    this.mixer?.stopAllAction(); this.mixer = null; this.idle = null;
    if (this.scene) { disposeObject(this.scene); this.scene.clear(); }
    this.environment?.dispose(); this.environment = null;
    this.scene = null; this.pivot = null; this.camera = null;
    this.renderer?.dispose(); this.renderer = null;
  }
  dispose(): void {
    this.hide(); this.observer.disconnect();
    document.removeEventListener("visibilitychange", this.visibility);
    this.dialog.remove();
  }
}

function disposeObject(root: THREE.Object3D): void {
  const materials = new Set<THREE.Material>();
  root.traverse((object) => {
    if (object instanceof THREE.DirectionalLight || object instanceof THREE.SpotLight || object instanceof THREE.PointLight) object.shadow.dispose();
    if (!(object instanceof THREE.Mesh)) return;
    object.geometry.dispose();
    for (const material of Array.isArray(object.material) ? object.material : [object.material]) materials.add(material);
  });
  for (const material of materials) material.dispose();
}
