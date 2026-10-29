import * as THREE from "three";
import { RoomEnvironment } from "three/addons/environments/RoomEnvironment.js";
import { getItem, type Loadout, type WeaponId } from "@crowned-dogs/core";
import { loadModel } from "../arena/load-model.ts";
import { tuneFittingMaterial } from "./turntable-material.ts";
import { QUALITY, startQuality, type Quality } from "../arena/quality.ts";
import { TextureTiers } from "../arena/textures.ts";
import { Hand } from "../arena/weapons.ts";
import { fitVaryag, makeFittingStage, makeSling, PLATFORM_TOP, prepareVaryag } from "./fitting-stage.ts";
import { authoredWeaponFit } from "./user-fitting-layout.ts";
import { callRust } from "../rust/engine.ts";

export interface PersonaPreviewCoverage {
  exact: false;
  description: string;
  missingItemIds: string[];
}

const ROOM = `${import.meta.env.BASE_URL}img/base/fitting-room-v2.png`;
const VARYAG = `${import.meta.env.BASE_URL}preview/varyag.json`;
const AUTHORED_PERSONA = `${import.meta.env.BASE_URL}preview/persona-authored.json`;

/** The user's edited Persona and Kit A, on a lit, rotating service platform. */
export class PersonaViewer {
  readonly previewDescription = "3D: комплект A и Варяг · остальные предметы пока показаны условно";

  private readonly scene = new THREE.Scene();
  private readonly stage = makeFittingStage();
  private readonly equipment = new THREE.Group();
  private readonly camera = new THREE.PerspectiveCamera(33, 1, .05, 40);
  private readonly key = new THREE.DirectionalLight(0xe8eef2, 3.2);
  private readonly hemisphere = new THREE.HemisphereLight(0xc3cacd, 0x2b2d2e, .6);
  private renderer: THREE.WebGLRenderer | null = null;
  private environment: THREE.WebGLRenderTarget | null = null;
  private quality: Quality = QUALITY.medium;
  private figure: THREE.Object3D | null = null;
  private hand: Hand | null = null;
  private varyag: THREE.Object3D | null = null;
  private sling: THREE.Object3D | null = null;
  private tiers: TextureTiers | null = null;
  private texturesPending: Promise<void> = Promise.resolve();
  private readonly originalTextures = new Set<THREE.Texture>();
  private loadout: Loadout | null = null;
  private pending: Promise<void> | null = null;
  private gpuReleased = false;
  private disposed = false;
  private ready = false;
  private active = false;
  private width = 1;
  private height = 1;
  private frame = 0;
  /** Where the fit put the camera, and how far the backdrop's drift has moved her on screen (CSS pixels). */
  private distance = 3;
  private moved: [number, number] = [0, 0];
  private drag: { id: number; x: number } | null = null;
  private readonly previousTouchAction: string;
  private readonly canvas: HTMLCanvasElement;

  constructor(canvas: HTMLCanvasElement) {
    this.canvas = canvas;
    this.previousTouchAction = canvas.style.touchAction;
    canvas.style.touchAction = "pan-y";
    canvas.addEventListener("pointerdown", this.pointerDown);
    canvas.addEventListener("pointermove", this.pointerMove);
    canvas.addEventListener("pointerup", this.pointerUp);
    canvas.addEventListener("pointercancel", this.pointerUp);
    canvas.addEventListener("lostpointercapture", this.pointerUp);
    canvas.addEventListener("webglcontextlost", this.contextLost);
    canvas.addEventListener("webglcontextrestored", this.contextRestored);
    this.stage.rotating.rotation.y = -.16;
    this.equipment.name = "Body_Mounted_Equipment";
    this.equipment.position.y = PLATFORM_TOP;
    this.stage.rotating.add(this.equipment);
    this.scene.add(this.stage.fixed, this.stage.rotating, this.hemisphere);
    this.key.position.set(-2.4, 4.2, 2.7);
    this.key.target.position.set(0, .9, 0);
    this.key.shadow.camera.left = this.key.shadow.camera.bottom = -1.7;
    this.key.shadow.camera.right = this.key.shadow.camera.top = 1.7;
    this.key.shadow.camera.near = .5; this.key.shadow.camera.far = 10;
    this.key.shadow.bias = -.00015; this.key.shadow.normalBias = .014;
    this.scene.add(this.key, this.key.target);
    const fill = new THREE.DirectionalLight(0xc7ccd0, .85);
    fill.position.set(2.4, 1.9, 2.1); this.scene.add(fill);
    const rim = new THREE.DirectionalLight(0xe8eef2, 2.0);
    rim.position.set(1.4, 2.7, -1.8); this.scene.add(rim);
    // The two warm utility lamps in the plate also catch the platform's lower rim.
    for (const x of [-1.3, 1.3]) {
      const practical = new THREE.PointLight(0xe4cbb0, .7, 3.5, 2);
      practical.position.set(x, .65, .25); this.scene.add(practical);
    }
  }

  load(): Promise<void> {
    if (this.disposed) return Promise.reject(new Error("Просмотр 3D уже закрыт."));
    return this.pending ??= this.loadModels();
  }

  private async loadModels(): Promise<void> {
    try {
      const renderer = this.renderer = new THREE.WebGLRenderer({ canvas: this.canvas, alpha: true, antialias: true, powerPreference: "high-performance" });
      renderer.setClearColor(0x000000, 0);
      renderer.outputColorSpace = THREE.SRGBColorSpace;
      renderer.toneMapping = THREE.ACESFilmicToneMapping;
      renderer.toneMappingExposure = 1.05;
      renderer.shadowMap.type = THREE.PCFShadowMap;
      this.configureQuality();
      this.resize(this.width, this.height);
      const pmrem = new THREE.PMREMGenerator(renderer), room = new RoomEnvironment();
      this.environment = pmrem.fromScene(room, .06);
      this.scene.environment = this.quality.env ? this.environment.texture : null;
      room.dispose(); pmrem.dispose();
      // Optional art is loaded independently; the figure remains usable if either fails.
      const gunPromise = loadModel(VARYAG).then((gltf) => {
        if (this.disposed || !this.renderer) { disposeObject(gltf.scene); return; }
        this.varyag = gltf.scene; prepareVaryag(this.varyag);
        this.equipment.add(this.varyag); this.tuneObject(this.varyag);
      }).catch(() => { /* Existing arena stand-in remains available. */ });
      const environmentPromise = this.loadRoomEnvironment();
      const gltf = await loadModel(AUTHORED_PERSONA);
      if (this.disposed) { disposeObject(gltf.scene); return; }
      const figure = this.figure = gltf.scene;
      // This static fitting export already contains the user's pose, mesh edits and platform height.
      // Do not dress, re-pose, resize or recenter it. Stand-in guns use a detached anchor only.
      const standIns = new THREE.Group(), anchor = new THREE.Group();
      anchor.name = "hand_r"; standIns.add(anchor);
      this.hand = new Hand(standIns, new THREE.Matrix4(), new THREE.MeshStandardMaterial({ color: 0x1b1c1d, roughness: .5, metalness: .55 }));
      this.equipment.add(this.hand.mount);
      figure.name = "Persona_Authored_In_Blender";
      figure.traverse((node) => { if (node instanceof THREE.Mesh) { node.frustumCulled = false; node.castShadow = true; node.receiveShadow = true; } });
      this.stage.rotating.add(figure);
      collectTextures(figure, this.originalTextures);
      this.tiers = new TextureTiers([figure], AUTHORED_PERSONA);
      await this.updateTextureQuality();
      if (this.disposed) return;
      await Promise.all([gunPromise, environmentPromise]);
      if (this.disposed) return;
      this.applyWeapon(); this.tuneObject(this.scene); this.ready = true;
      this.resize(this.width, this.height); this.render();
    } catch {
      if (this.disposed) return;
      this.releaseGpu();
      throw new Error("Не удалось загрузить 3D-модель. Проверьте соединение и откройте примерочную ещё раз.");
    }
  }

  private async loadRoomEnvironment(): Promise<void> {
    let texture: THREE.Texture | undefined;
    try {
      texture = await new THREE.TextureLoader().loadAsync(ROOM);
      if (this.disposed || !this.renderer) return;
      texture.colorSpace = THREE.SRGBColorSpace;
      texture.mapping = THREE.EquirectangularReflectionMapping;
      const pmrem = new THREE.PMREMGenerator(this.renderer);
      const next = pmrem.fromEquirectangular(texture); pmrem.dispose();
      this.environment?.dispose(); this.environment = next;
      this.scene.environment = this.quality.env ? next.texture : null;
    } catch { /* The neutral studio environment is a valid offline fallback. */ }
    finally { texture?.dispose(); }
  }

  private configureQuality(): void {
    if (!this.renderer) return;
    this.quality = QUALITY[startQuality(this.renderer.getContext() as WebGL2RenderingContext).id];
    const q = this.quality;
    this.stage.setQuality(q.id);
    this.canvas.dataset.previewQuality = q.id;
    this.renderer.shadowMap.enabled = q.shadow > 0;
    this.key.castShadow = q.shadow > 0;
    if (this.key.shadow.mapSize.x !== q.shadow && this.key.shadow.map) { this.key.shadow.map.dispose(); this.key.shadow.map = null; }
    this.key.shadow.mapSize.setScalar(Math.max(1, q.shadow)); this.key.shadow.radius = q.shadowRadius;
    this.hemisphere.intensity = q.env ? .52 : .78;
    this.scene.environment = q.env ? this.environment?.texture ?? null : null;
    this.tuneObject(this.scene);
  }

  private tuneObject(root: THREE.Object3D): void {
    const anisotropy = Math.min(this.quality.anisotropy, this.renderer?.capabilities.getMaxAnisotropy() ?? 1);
    const materials = new Set<THREE.Material>();
    root.traverse((object) => {
      if (!(object instanceof THREE.Mesh)) return;
      for (const material of Array.isArray(object.material) ? object.material : [object.material]) materials.add(material);
    });
    for (const material of materials) {
      tuneFittingMaterial(material, this.quality);
      material.alphaToCoverage = material.alphaTest > 0;
      for (const value of Object.values(material)) if (value instanceof THREE.Texture) value.anisotropy = anisotropy;
    }
  }

  private updateTextureQuality(): Promise<void> {
    const tiers = this.tiers, q = this.quality;
    if (!tiers || !this.renderer) return Promise.resolve();
    const anisotropy = Math.min(q.anisotropy, this.renderer.capabilities.getMaxAnisotropy());
    // Serialise tier changes so closing cannot free maps while a change uses them.
    this.texturesPending = this.texturesPending.then(async () => {
      if (this.disposed) return;
      await tiers.set(q.textures, anisotropy);
      if (!this.disposed) { this.canvas.dataset.previewTextureSize = String(tiers.current); this.render(); }
    }).catch(() => { /* Embedded source textures remain usable. */ });
    return this.texturesPending;
  }

  resize(width: number, height: number): void {
    if (this.disposed) return;
    const fit = callRust<{ width: number; height: number; aspect: number; distance: number; ratio: number }>("graphics.fittingCamera", {
      width, height, fov: this.camera.fov, dpr: globalThis.devicePixelRatio || 1, ...this.quality,
    });
    this.width = fit.width; this.height = fit.height;
    this.camera.aspect = fit.aspect;
    this.camera.updateProjectionMatrix();
    this.distance = fit.distance; this.aim();
    if (this.renderer) {
      this.renderer.setPixelRatio(fit.ratio); this.renderer.setSize(this.width, this.height, false);
      this.canvas.dataset.previewDpr = fit.ratio.toFixed(2);
    }
    this.render();
  }

  setActive(active: boolean): void {
    if (this.disposed) return;
    this.active = active;
    if (active) {
      const before = this.quality.id; this.configureQuality();
      if (before !== this.quality.id) { void this.updateTextureQuality(); this.resize(this.width, this.height); }
      this.render();
    } else { cancelAnimationFrame(this.frame); this.frame = 0; this.drag = null; }
  }

  /**
   * Moves her on screen with the backdrop's depth drift: the camera slides sideways (it does not turn),
   * so she shifts and shows a sliver of her side just as the room behind her does.
   */
  shift(x: number, y: number): void {
    if (Math.abs(x - this.moved[0]) + Math.abs(y - this.moved[1]) < .05) return;
    this.moved = [x, y]; this.aim(); this.render();
  }

  private aim(): void {
    const aim = callRust<{ position: [number, number, number]; target: [number, number, number] }>("graphics.fittingAim", {
      distance: this.distance, height: this.height, fov: this.camera.fov, x: this.moved[0], y: this.moved[1],
    });
    this.camera.position.fromArray(aim.position); this.camera.lookAt(...aim.target);
    this.camera.updateMatrixWorld();
  }

  rotate(deltaRadians: number): void {
    if (this.disposed || !Number.isFinite(deltaRadians)) return;
    this.stage.rotating.rotation.y = callRust<number>("graphics.fittingRotation", { angle: this.stage.rotating.rotation.y, delta: deltaRadians });
    this.render();
  }

  updateLoadout(loadout: Loadout): PersonaPreviewCoverage {
    this.loadout = loadout; this.applyWeapon(); this.render();
    return callRust<PersonaPreviewCoverage>("graphics.fittingCoverage", { loadout, weapon: weaponOf(loadout.weapon1.item), hasRifleModel: !!this.varyag });
  }

  private applyWeapon(): void {
    if (!this.figure) return;
    const weapon = this.loadout ? weaponOf(this.loadout.weapon1.item) : "rifle";
    const realRifle = weapon === "rifle" && !!this.varyag;
    this.hand?.hold(realRifle ? null : weapon);
    if (this.varyag) { this.varyag.visible = realRifle; fitVaryag(this.varyag, this.loadout); }
    const mount = realRifle ? this.varyag : this.hand?.mount;
    if (!mount) return;
    const pose = callRust<{ position: [number, number, number]; rotation: [number, number, number] }>("graphics.equipmentPose", { weapon });
    mount.scale.setScalar(1);
    mount.position.fromArray(pose.position); mount.rotation.set(...pose.rotation);
    if (this.sling) { this.sling.removeFromParent(); disposeObject(this.sling); }
    this.sling = realRifle ? authoredWeaponFit(mount) : makeSling(mount, weapon === "pistol");
    this.equipment.add(this.sling); this.tuneObject(this.equipment);
  }

  private readonly pointerDown = (event: PointerEvent): void => {
    if (!this.active || event.button !== 0 || !event.isPrimary) return;
    this.drag = { id: event.pointerId, x: event.clientX }; this.canvas.setPointerCapture(event.pointerId);
  };
  private readonly pointerMove = (event: PointerEvent): void => {
    if (!this.drag || event.pointerId !== this.drag.id) return;
    this.rotate((event.clientX - this.drag.x) * .009); this.drag.x = event.clientX;
  };
  private readonly pointerUp = (event: PointerEvent): void => {
    if (this.drag?.id !== event.pointerId) return;
    this.drag = null;
    if (this.canvas.hasPointerCapture(event.pointerId)) this.canvas.releasePointerCapture(event.pointerId);
  };
  private readonly contextLost = (event: Event): void => {
    event.preventDefault(); cancelAnimationFrame(this.frame); this.frame = 0;
    this.canvas.dispatchEvent(new CustomEvent("persona-preview-error", { detail: "3D-просмотр временно недоступен: графический контекст потерян.", bubbles: true }));
  };
  private readonly contextRestored = (): void => { this.configureQuality(); this.render(); };

  private render(): void {
    if (this.disposed || !this.ready || !this.active || this.frame) return;
    // No idle RAF or animation mixer: render the assembly only when something changes.
    this.frame = requestAnimationFrame(() => {
      this.frame = 0;
      if (this.active && !this.disposed) this.renderer?.render(this.scene, this.camera);
    });
  }

  private releaseGpu(): void {
    if (this.gpuReleased) return;
    this.gpuReleased = true;
    const roots = new THREE.Group(); roots.add(this.stage.fixed, this.stage.rotating);
    // TextureTiers has no dispose API. Let its last write settle before releasing pixels.
    void this.texturesPending.finally(() => disposeObject(roots, this.originalTextures));
    this.figure = null; this.hand = null; this.varyag = null; this.tiers = null; this.sling = null;
    this.scene.environment = null; this.environment?.dispose(); this.environment = null;
    this.key.shadow.dispose(); this.renderer?.dispose(); this.renderer = null; this.ready = false;
  }

  dispose(): void {
    if (this.disposed) return;
    this.disposed = true; this.active = false; cancelAnimationFrame(this.frame); this.frame = 0;
    if (this.drag && this.canvas.hasPointerCapture(this.drag.id)) this.canvas.releasePointerCapture(this.drag.id);
    this.drag = null;
    this.canvas.removeEventListener("pointerdown", this.pointerDown);
    this.canvas.removeEventListener("pointermove", this.pointerMove);
    this.canvas.removeEventListener("pointerup", this.pointerUp);
    this.canvas.removeEventListener("pointercancel", this.pointerUp);
    this.canvas.removeEventListener("lostpointercapture", this.pointerUp);
    this.canvas.removeEventListener("webglcontextlost", this.contextLost);
    this.canvas.removeEventListener("webglcontextrestored", this.contextRestored);
    this.canvas.style.touchAction = this.previousTouchAction; this.releaseGpu();
  }
}

function weaponOf(itemId: string): WeaponId {
  const item = getItem(itemId);
  return item.kind === "weapon" ? item.weapon : "rifle";
}

function collectTextures(root: THREE.Object3D, textures: Set<THREE.Texture>): void {
  root.traverse((node) => {
    if (!(node instanceof THREE.Mesh)) return;
    for (const material of Array.isArray(node.material) ? node.material : [node.material]) {
      for (const value of Object.values(material)) if (value instanceof THREE.Texture) textures.add(value);
    }
  });
}

function disposeObject(root: THREE.Object3D, extraTextures = new Set<THREE.Texture>()): void {
  const materials = new Set<THREE.Material>(), textures = new Set(extraTextures), geometries = new Set<THREE.BufferGeometry>();
  collectTextures(root, textures);
  root.traverse((node) => {
    if (!(node instanceof THREE.Mesh)) return;
    geometries.add(node.geometry);
    for (const material of Array.isArray(node.material) ? node.material : [node.material]) materials.add(material);
  });
  for (const geometry of geometries) geometry.dispose();
  for (const material of materials) material.dispose();
  const images = new Set<unknown>();
  for (const texture of textures) {
    if (!images.has(texture.image) && typeof ImageBitmap !== "undefined" && texture.image instanceof ImageBitmap) texture.image.close();
    images.add(texture.image); texture.dispose();
  }
}
