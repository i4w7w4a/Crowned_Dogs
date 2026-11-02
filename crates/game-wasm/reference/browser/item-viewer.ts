import * as THREE from "three";
import { motionCompute } from "../../../../packages/runtime/src/index.ts";
import { RoomEnvironment } from "three/addons/environments/RoomEnvironment.js";
import { MOD_SLOTS, type Loadout, type ModSlot } from "@crowned-dogs/core";
import { loadModel } from "../arena/load-model.ts";
import { QUALITY, startQuality, type Quality } from "../arena/quality.ts";
import { TextureTiers } from "../arena/textures.ts";
import { tune } from "../arena/materials.ts";
import { fitVaryag, prepareVaryag } from "./fitting-stage.ts";
import "./item-viewer.css";
import { ITEM_MODELS } from "./model-catalog.ts";
export { ITEM_MODELS } from "./model-catalog.ts";

const RIFLE = `${import.meta.env.BASE_URL}preview/varyag.json`;
type Model = { root: THREE.Group; textures: Set<THREE.Texture> };
type ViewerOptions = { workbench?: boolean; onSockets?: (points: Partial<Record<ModSlot, [number, number]>>) => void };

/** One on-demand canvas per open item view. Models and their GPU resources belong to this view. */
export class ItemViewer {
  private canvas: HTMLCanvasElement;
  private status: HTMLElement;
  private retry: HTMLButtonElement;
  private renderer: THREE.WebGLRenderer | null = null;
  private environment: THREE.WebGLRenderTarget | null = null;
  private scene = new THREE.Scene();
  private camera = new THREE.PerspectiveCamera(33, 1, .01, 50);
  private pivot = new THREE.Group();
  private offset = new THREE.Group();
  private models = new Map<string, Promise<Model>>();
  private quality: Quality = QUALITY.medium;
  private observer: ResizeObserver;
  private active = false;
  private disposed = false;
  private serial = 0;
  private request: { id: string; name: string; loadout?: Loadout } | null = null;
  private radius = 1;
  private host: HTMLElement;
  private drag: { id: number; x: number; y: number } | null = null;
  private displayed: THREE.Group | null = null;
  private options: ViewerOptions;

  constructor(host: HTMLElement, options: ViewerOptions = {}) {
    this.options = options;
    this.host = host;
    host.classList.add("ci-viewer");
    host.innerHTML = `<canvas tabindex="0" role="img"></canvas><div class="ci-message" role="status"><span></span><button class="glass" type="button" hidden>Повторить</button></div><p class="ci-hint">Потяни, чтобы повернуть</p>`;
    if (options.workbench) host.querySelector(".ci-hint")!.textContent = "Влево-вправо — поворот · вверх-вниз — наклон";
    this.canvas = host.querySelector("canvas")!; this.status = host.querySelector(".ci-message span")!; this.retry = host.querySelector("button")!;
    this.retry.addEventListener("click", () => { if (this.request) void this.show(this.request.id, this.request.name, this.request.loadout); });
    this.canvas.addEventListener("pointerdown", this.down); this.canvas.addEventListener("pointermove", this.move);
    for (const type of ["pointerup", "pointercancel", "lostpointercapture"] as const) this.canvas.addEventListener(type, this.up);
    this.canvas.addEventListener("keydown", this.key);
    this.canvas.addEventListener("webglcontextlost", this.lost); this.canvas.addEventListener("webglcontextrestored", this.restored);
    this.observer = new ResizeObserver(() => this.resize()); this.observer.observe(host);
    this.scene.add(this.pivot); this.pivot.add(this.offset);
    this.scene.add(new THREE.HemisphereLight(0xdce2e8, 0x323335, 1.2));
    for (const [x, y, z, power] of [[-2, 3, 3, 3.6], [3, 1, 2, 1.4], [1, 2, -3, 2.8]] as const) {
      const light = new THREE.DirectionalLight(0xe9eef2, power); light.position.set(x, y, z); this.scene.add(light);
    }
  }

  private init(): void {
    if (this.renderer) return;
    const renderer = new THREE.WebGLRenderer({ canvas: this.canvas, alpha: true, antialias: true });
    this.renderer = renderer; renderer.setClearColor(0x000000, 0);
    renderer.outputColorSpace = THREE.SRGBColorSpace; renderer.toneMapping = THREE.ACESFilmicToneMapping; renderer.toneMappingExposure = .95;
    this.quality = QUALITY[startQuality(renderer.getContext() as WebGL2RenderingContext).id];
    const pmrem = new THREE.PMREMGenerator(renderer), room = new RoomEnvironment();
    this.environment = pmrem.fromScene(room, .06); room.dispose(); pmrem.dispose();
    this.scene.environment = this.quality.env ? this.environment.texture : null;
  }

  private model(url: string): Promise<Model> {
    let pending = this.models.get(url);
    if (!pending) {
      pending = (async () => {
        const { scene: root } = await loadModel(url);
        const entry = { root, textures: texturesOf(root) };
        try {
          if (!this.disposed) {
            if (url === RIFLE) prepareVaryag(root);
            const tiers = new TextureTiers([root], url);
            await tiers.set(this.quality.textures, Math.min(this.quality.anisotropy, this.renderer!.capabilities.getMaxAnisotropy()));
            texturesOf(root).forEach(t => entry.textures.add(t));
            root.traverse(node => { if (node instanceof THREE.Mesh) for (const mat of Array.isArray(node.material) ? node.material : [node.material]) tune(mat, this.quality); });
          }
          return entry;
        } catch (error) { release(entry); throw error; }
      })();
      this.models.set(url, pending);
      void pending.catch(() => { if (this.models.get(url) === pending) this.models.delete(url); });
    }
    return pending;
  }

  async show(id: string, name: string, loadout?: Loadout): Promise<void> {
    if (this.disposed || !ITEM_MODELS[id]) return;
    const keepAngle = this.request?.id === id;
    this.request = { id, name, loadout }; const serial = ++this.serial;
    this.active = true; this.host.hidden = false; this.canvas.hidden = true; this.retry.hidden = true;
    this.status.textContent = "Загрузка…"; this.canvas.setAttribute("aria-label", `${name}. Объёмная модель, вращение перетаскиванием или стрелками.${this.options.workbench ? " Влево-вправо — поворот, вверх-вниз — наклон." : ""}`);
    this.host.dataset.item = id;
    try {
      this.init();
      const rifle = id === "rifle-varyag", { root } = await this.model(`${import.meta.env.BASE_URL}preview/${ITEM_MODELS[id]}.json`);
      if (this.disposed || serial !== this.serial) return;
      this.offset.clear(); this.offset.position.set(0, 0, 0); this.offset.scale.setScalar(1);
      const angle = this.pivot.rotation.clone(); this.pivot.rotation.set(0, 0, 0);
      this.offset.add(root); this.displayed = root;
      if (rifle) fitVaryag(root, loadout ?? null);
      this.scene.updateMatrixWorld(true);
      const bounds = visibleBounds(root), centre = bounds.getCenter(new THREE.Vector3()), size = bounds.getSize(new THREE.Vector3());
      const scale = 1.8 / Math.max(size.x, size.y, size.z, .001);
      this.offset.scale.setScalar(scale); this.offset.position.copy(centre).multiplyScalar(-scale);
      this.radius = size.length() * scale / 2;
      if (keepAngle) this.pivot.rotation.copy(angle);
      else this.pivot.rotation.set(.06, rifle ? -.22 : .35, rifle ? (this.options.workbench ? .68 : -.3) : 0);
      this.status.textContent = ""; this.canvas.hidden = false;
      this.host.dataset.state = "ready"; this.resize();
    } catch {
      if (this.disposed || serial !== this.serial) return;
      this.host.dataset.state = "error"; this.status.textContent = "Модель не загрузилась"; this.retry.hidden = false;
    }
  }

  setActive(active: boolean): void { this.active = active; if (!active) this.drag = null; else this.resize(); }
  hide(): void { ++this.serial; this.active = false; this.host.hidden = true; this.drag = null; }
  private resize(): void {
    if (!this.renderer || !this.active || this.disposed) return;
    const w = this.host.clientWidth, h = this.host.clientHeight; if (!w || !h) return;
    const q = this.quality, dpr = Math.min(devicePixelRatio * q.ssaa, q.maxDpr, Math.sqrt(q.maxPixels / (w * h)));
    this.renderer.setPixelRatio(dpr); this.renderer.setSize(w, h, false);
    this.camera.aspect = w / h; this.camera.updateProjectionMatrix();
    const half = THREE.MathUtils.degToRad(this.camera.fov / 2);
    let distance = this.radius / Math.sin(Math.min(half, Math.atan(Math.tan(half) * w / h))) * 1.10;
    if (this.displayed) {
      this.scene.updateMatrixWorld(true);
      const b = visibleBounds(this.displayed), tanY = Math.tan(half), tanX = tanY * w / h;
      const corners: number[] = [];
      for (const x of [b.min.x, b.max.x]) for (const y of [b.min.y, b.max.y]) for (const z of [b.min.z, b.max.z]) {
        corners.push(x, y, z);
      }
      distance = motionCompute("cameraDistance", new Float64Array([tanY, tanX, this.options.workbench ? 1.28 : 1.16, ...corners]))[0]!;
    }
    this.camera.position.set(0, 0, distance);
    this.camera.lookAt(0, 0, 0); this.render();
  }
  private render(): void {
    if (!this.active || this.disposed) return;
    this.renderer?.render(this.scene, this.camera);
    if (this.options.onSockets && this.displayed) {
      const points: Partial<Record<ModSlot, [number, number]>> = {};
      for (const slot of MOD_SLOTS) {
        const socket = this.displayed.getObjectByName(`socket_${slot}`);
        if (!socket) continue;
        const p = socket.getWorldPosition(new THREE.Vector3()).project(this.camera);
        if (p.z < 1) points[slot] = [(p.x + 1) * this.host.clientWidth / 2, (1 - p.y) * this.host.clientHeight / 2];
      }
      this.options.onSockets(points);
    }
  }
  private down = (e: PointerEvent): void => { if (e.button || !e.isPrimary) return; this.drag = { id: e.pointerId, x: e.clientX, y: e.clientY }; this.canvas.setPointerCapture(e.pointerId); };
  private move = (e: PointerEvent): void => {
    if (!this.drag || e.pointerId !== this.drag.id) return;
    this.turn((e.clientX - this.drag.x) * .012, (e.clientY - this.drag.y) * .008);
    this.drag.x = e.clientX; this.drag.y = e.clientY; this.render();
  };
  private turn(horizontal: number, vertical: number): void {
    if (this.options.workbench) {
      // Camera-facing roll remains usable at every viewing angle, including a side view.
      this.pivot.quaternion.fromArray(motionCompute("viewerTurn", new Float64Array([...this.pivot.quaternion.toArray(), horizontal, vertical])));
    } else {
      this.pivot.rotation.y += horizontal;
      this.pivot.rotation.x = THREE.MathUtils.clamp(this.pivot.rotation.x + vertical, -1.1, 1.1);
    }
  }
  private up = (e: PointerEvent): void => { if (this.drag?.id === e.pointerId) { this.drag = null; if (this.canvas.hasPointerCapture(e.pointerId)) this.canvas.releasePointerCapture(e.pointerId); } };
  private key = (e: KeyboardEvent): void => {
    if (!["ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown"].includes(e.key)) return;
    e.preventDefault();
    if (e.key === "ArrowLeft" || e.key === "ArrowRight") this.turn(e.key === "ArrowLeft" ? -.2 : .2, 0);
    else this.turn(0, e.key === "ArrowUp" ? -.15 : .15);
    this.render();
  };
  private lost = (e: Event): void => { e.preventDefault(); this.status.textContent = "Восстановление изображения…"; };
  private restored = (): void => { this.status.textContent = ""; this.resize(); };
  dispose(): void {
    this.disposed = true; ++this.serial; this.observer.disconnect();
    this.canvas.removeEventListener("pointerdown", this.down); this.canvas.removeEventListener("pointermove", this.move);
    for (const type of ["pointerup", "pointercancel", "lostpointercapture"] as const) this.canvas.removeEventListener(type, this.up);
    this.canvas.removeEventListener("keydown", this.key); this.canvas.removeEventListener("webglcontextlost", this.lost); this.canvas.removeEventListener("webglcontextrestored", this.restored);
    for (const pending of this.models.values()) void pending.then(release, () => {});
    this.models.clear(); this.environment?.dispose(); this.renderer?.dispose(); this.host.replaceChildren();
  }
}

function visibleBounds(root: THREE.Object3D): THREE.Box3 {
  const bounds = new THREE.Box3();
  root.traverseVisible(node => {
    if (!(node instanceof THREE.Mesh)) return;
    node.geometry.computeBoundingBox();
    if (node.geometry.boundingBox) bounds.union(node.geometry.boundingBox.clone().applyMatrix4(node.matrixWorld));
  });
  return bounds;
}

function texturesOf(root: THREE.Object3D): Set<THREE.Texture> {
  const textures = new Set<THREE.Texture>();
  root.traverse(node => { if (node instanceof THREE.Mesh) for (const mat of Array.isArray(node.material) ? node.material : [node.material]) for (const value of Object.values(mat)) if (value instanceof THREE.Texture) textures.add(value); });
  return textures;
}
function release(model: Model): void {
  const materials = new Set<THREE.Material>(), geometries = new Set<THREE.BufferGeometry>();
  model.root.traverse(node => { if (node instanceof THREE.Mesh) { geometries.add(node.geometry); for (const m of Array.isArray(node.material) ? node.material : [node.material]) materials.add(m); } });
  geometries.forEach(g => g.dispose()); materials.forEach(m => m.dispose()); model.textures.forEach(t => t.dispose());
}
