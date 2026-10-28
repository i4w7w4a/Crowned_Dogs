import {
  ACESFilmicToneMapping, AnimationMixer, Box3, Color, DirectionalLight, Group, HemisphereLight,
  Material, MathUtils, Mesh, MeshStandardMaterial, Object3D, PerspectiveCamera, PMREMGenerator, Scene,
  SRGBColorSpace, Texture, Vector3, WebGLRenderer, type AnimationClip,
} from "three";
import { RoomEnvironment } from "three/addons/environments/RoomEnvironment.js";
import { getItem, type Loadout, type WeaponId } from "@crowned-dogs/core";
import { dress, KIT_A } from "../arena/dress.ts";
import { loadModel, PERSONA_MODEL } from "../arena/load-model.ts";
import { Hand, MODELLED_GUNS, MOD_MESHES, gripOf, loadGuns } from "../arena/weapons.ts";

export interface PersonaPreviewCoverage {
  /** Kit A is an art preview, not a complete catalogue of fitted equipment. */
  exact: false;
  description: string;
  missingItemIds: string[];
}

/**
 * A transparent, full-body preview of the Persona the arena fights with, in Kit A, standing ready with
 * the first weapon of the kit (the real gun where it is modelled, with the mods that have meshes; else the
 * arena's stand-in block). Catalogue armour and skins are not modelled yet and do NOT reskin it; the caller shows
 * previewDescription next to the live loadout controls.
 */
export class PersonaViewer {
  readonly previewDescription = "3D: Персона в комплекте A · броня, раскраска и большинство модов пока не меняют вид";

  private readonly canvas: HTMLCanvasElement;
  private readonly scene = new Scene();
  private readonly model = new Group();
  private readonly camera = new PerspectiveCamera(33, 1, 0.05, 40);
  private renderer: WebGLRenderer | null = null;
  private environment: ReturnType<PMREMGenerator["fromScene"]> | null = null;
  private figure: Object3D | null = null;
  private clips: AnimationClip[] = [];
  private mixer: AnimationMixer | null = null;
  private hand: Hand | null = null;
  private loadout: Loadout | null = null;
  private pending: Promise<void> | null = null;
  private disposed = false;
  private ready = false;
  private active = false;
  private width = 1;
  private height = 1;
  private frame = 0;
  private drag: { id: number; x: number } | null = null;
  private readonly previousTouchAction: string;

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

    this.model.name = "Persona_Kit_A_Preview";
    this.model.rotation.y = -0.16;
    this.scene.add(this.model);

    // Soft cool workshop light with a small warm edge; the room is a separate image.
    this.scene.add(new HemisphereLight(0xcbd6de, 0x242525, 1.15));
    const key = new DirectionalLight(0xe0e8ee, 3.5);
    key.position.set(-2.2, 3.8, 3.4);
    this.scene.add(key);
    const fill = new DirectionalLight(0x9fb1c2, 1.0);
    fill.position.set(2, 1.3, 1.2);
    this.scene.add(fill);
    const rim = new DirectionalLight(0xd4af85, 1.8);
    rim.position.set(1.4, 2.5, -2.2);
    this.scene.add(rim);
    this.camera.position.set(0, 0.98, 4.2);
    this.camera.lookAt(0, 0.92, 0);
  }

  load(): Promise<void> {
    if (this.disposed) return Promise.reject(new Error("Просмотр 3D уже закрыт."));
    if (!this.pending) this.pending = this.loadModels();
    return this.pending;
  }

  private async loadModels(): Promise<void> {
    try {
      this.renderer = new WebGLRenderer({
        canvas: this.canvas, alpha: true, antialias: true,
        powerPreference: "high-performance",
      });
      this.renderer.setClearColor(new Color(0x000000), 0);
      this.renderer.outputColorSpace = SRGBColorSpace;
      this.renderer.toneMapping = ACESFilmicToneMapping;
      this.renderer.toneMappingExposure = 0.9;
      this.renderer.setPixelRatio(Math.min(globalThis.devicePixelRatio || 1, 2));
      this.renderer.setSize(this.width, this.height, false);
      const pmrem = new PMREMGenerator(this.renderer);
      const room = new RoomEnvironment();
      this.environment = pmrem.fromScene(room, 0.04);
      this.scene.environment = this.environment.texture;
      this.scene.environmentIntensity = 0.24;
      room.dispose();
      pmrem.dispose();

      const [gltf, guns] = await Promise.all([loadModel(PERSONA_MODEL), loadGuns()]);
      if (this.disposed) { disposeObject(gltf.scene); return; }
      const figure = gltf.scene;
      dress(figure, KIT_A);
      figure.traverse((o) => { if (o instanceof Mesh) o.frustumCulled = false; });
      // the grip comes from the rest pose, before any clip moves the hand
      const grip = gripOf(figure);
      if (grip) this.hand = new Hand(figure, grip, new MeshStandardMaterial({ color: 0x1b1c1d, roughness: 0.45, metalness: 0.55 }), { guns });
      figure.name = "Persona_Kit_A";
      this.figure = figure;
      this.clips = gltf.animations;
      this.mixer = new AnimationMixer(figure);
      this.model.add(figure);
      this.applyWeapon();

      this.model.traverse((object) => {
        if (!(object instanceof Mesh)) return;
        const materials = Array.isArray(object.material) ? object.material : [object.material];
        for (const material of materials) {
          for (const value of Object.values(material)) {
            if (value instanceof Texture) value.anisotropy = Math.min(this.renderer!.capabilities.getMaxAnisotropy(), 4);
          }
        }
      });
      this.ready = true;
      this.resize(this.width, this.height);
      this.render();
      this.start();
    } catch {
      if (this.disposed) return;
      this.releaseGpu();
      throw new Error("Не удалось загрузить 3D-модель. Проверьте соединение и откройте примерочную ещё раз.");
    }
  }

  resize(width: number, height: number): void {
    if (this.disposed) return;
    this.width = Math.max(1, Math.round(width));
    this.height = Math.max(1, Math.round(height));
    this.camera.aspect = this.width / this.height;
    // Keep both boots and the helmet visible, including on a narrow phone.
    const vFov = MathUtils.degToRad(this.camera.fov);
    const hFov = 2 * Math.atan(Math.tan(vFov / 2) * this.camera.aspect);
    const distance = Math.max(1.02 / Math.tan(vFov / 2), 0.73 / Math.tan(hFov / 2));
    this.camera.position.set(0, 0.98, distance);
    this.camera.lookAt(0, 0.92, 0);
    this.camera.updateProjectionMatrix();
    this.renderer?.setSize(this.width, this.height, false);
    this.render();
  }

  setActive(active: boolean): void {
    if (this.disposed) return;
    this.active = active;
    if (active) { this.render(); this.start(); }
    else { cancelAnimationFrame(this.frame); this.frame = 0; this.drag = null; }
  }

  rotate(deltaRadians: number): void {
    if (this.disposed || !Number.isFinite(deltaRadians)) return;
    this.model.rotation.y = MathUtils.euclideanModulo(this.model.rotation.y + deltaRadians, Math.PI * 2);
    this.render();
  }

  updateLoadout(loadout: Loadout): PersonaPreviewCoverage {
    this.loadout = loadout;
    this.applyWeapon();
    this.render();
    // Kit A, and only the modelled guns and mods are drawn as themselves.
    const missing = [loadout.head, loadout.body, loadout.suit, loadout.boots, loadout.knife, loadout.grenade];
    if (!MODELLED_GUNS.includes(loadout.weapon1.item)) missing.push(loadout.weapon1.item);
    if (loadout.weapon2) missing.push(loadout.weapon2.item);
    missing.push(...Object.values(loadout.weapon1.mods ?? {}).filter((id) => !(id in MOD_MESHES)));
    if (loadout.weapon1.skin) missing.push(loadout.weapon1.skin);
    return { exact: false, description: this.previewDescription, missingItemIds: [...new Set(missing)] };
  }

  /** Stands the Persona in the ready pose of the kit's first weapon, with that gun in hand. */
  private applyWeapon(): void {
    if (!this.figure || !this.mixer) return;
    const weapon: WeaponId = this.loadout ? weaponOf(this.loadout.weapon1.item) : "rifle";
    this.mixer.stopAllAction();
    const clip = this.clips.find((c) => c.name === `ready_${weapon === "sniper" ? "shotgun" : weapon}`);
    if (clip) { this.mixer.clipAction(clip).play(); this.mixer.update(0.5); }
    this.hand?.hold(weapon);
    this.hand?.fit(this.loadout?.weapon1.mods);
    // stand on the floor at the centre, whatever the pose
    this.figure.position.set(0, 0, 0);
    this.figure.updateMatrixWorld(true);
    const box = new Box3().setFromObject(this.figure, true), centre = box.getCenter(new Vector3());
    this.figure.position.set(-centre.x, -box.min.y, -centre.z);
  }

  private readonly pointerDown = (event: PointerEvent): void => {
    if (!this.active || event.button !== 0 || !event.isPrimary) return;
    this.drag = { id: event.pointerId, x: event.clientX };
    this.canvas.setPointerCapture(event.pointerId);
  };

  private readonly pointerMove = (event: PointerEvent): void => {
    if (!this.drag || event.pointerId !== this.drag.id) return;
    this.rotate((event.clientX - this.drag.x) * 0.009);
    this.drag.x = event.clientX;
  };

  private readonly pointerUp = (event: PointerEvent): void => {
    if (this.drag?.id !== event.pointerId) return;
    this.drag = null;
    if (this.canvas.hasPointerCapture(event.pointerId)) this.canvas.releasePointerCapture(event.pointerId);
  };

  private readonly contextLost = (event: Event): void => {
    event.preventDefault();
    cancelAnimationFrame(this.frame);
    this.frame = 0;
    this.canvas.dispatchEvent(new CustomEvent("persona-preview-error", {
      detail: "3D-просмотр временно недоступен: графический контекст потерян.", bubbles: true,
    }));
  };

  private readonly contextRestored = (): void => { this.render(); this.start(); };

  private start(): void {
    if (!this.active || !this.ready || this.disposed || this.frame) return;
    // Static model: render on input/resize only. No idle animation or background loop.
    this.frame = requestAnimationFrame(() => { this.frame = 0; this.render(); });
  }

  private render(): void {
    if (!this.disposed && this.ready) this.renderer?.render(this.scene, this.camera);
  }

  private releaseGpu(): void {
    disposeObject(this.model);
    this.model.clear();
    this.figure = null;
    this.mixer = null;
    this.hand = null;
    this.scene.environment = null;
    this.environment?.dispose();
    this.environment = null;
    this.renderer?.dispose();
    this.renderer = null;
    this.ready = false;
  }

  dispose(): void {
    if (this.disposed) return;
    this.disposed = true;
    this.active = false;
    cancelAnimationFrame(this.frame);
    this.frame = 0;
    if (this.drag && this.canvas.hasPointerCapture(this.drag.id)) this.canvas.releasePointerCapture(this.drag.id);
    this.drag = null;
    this.canvas.removeEventListener("pointerdown", this.pointerDown);
    this.canvas.removeEventListener("pointermove", this.pointerMove);
    this.canvas.removeEventListener("pointerup", this.pointerUp);
    this.canvas.removeEventListener("pointercancel", this.pointerUp);
    this.canvas.removeEventListener("lostpointercapture", this.pointerUp);
    this.canvas.removeEventListener("webglcontextlost", this.contextLost);
    this.canvas.removeEventListener("webglcontextrestored", this.contextRestored);
    this.canvas.style.touchAction = this.previousTouchAction;
    this.releaseGpu();
  }
}

function weaponOf(itemId: string): WeaponId {
  const item = getItem(itemId);
  return item.kind === "weapon" ? item.weapon : "rifle";
}

function disposeObject(object: Object3D): void {
  const materials = new Set<Material>();
  const textures = new Set<Texture>();
  object.traverse((node) => {
    if (!(node instanceof Mesh)) return;
    node.geometry.dispose();
    for (const material of Array.isArray(node.material) ? node.material : [node.material]) materials.add(material);
  });
  for (const material of materials) {
    for (const value of Object.values(material)) if (value instanceof Texture) textures.add(value);
    material.dispose();
  }
  for (const texture of textures) {
    // GLTFLoader may use ImageBitmap; explicitly release those decoded pixels too.
    if (typeof ImageBitmap !== "undefined" && texture.image instanceof ImageBitmap) texture.image.close();
    texture.dispose();
  }
}
