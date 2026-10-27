// Live 3D figures standing in a painted frame. The three.js canvas lies exactly
// over the picture, an off-centre lens matched to the painting puts the figures'
// feet on the painted floor, and the bodycam lens (lens.ts) lays them over the
// painting and shoots both through the same glass. The fight code only calls arm, move,
// attack, hit, fall and reset; everything else here is about looking right.

import type { Zone } from "@crowned-dogs/core";
import * as THREE from "three";
import { MeshoptDecoder } from "three/addons/libs/meshopt_decoder.module.js";
import { GLTFLoader, type GLTF } from "three/addons/loaders/GLTFLoader.js";
import * as SkeletonUtils from "three/addons/utils/SkeletonUtils.js";
import { Dust } from "./dust.ts";
import type { Frame } from "./frames.ts";
import { Lens, type View } from "./lens.ts";
import { tune } from "./materials.ts";
import { Motion, type Attack, type Stance } from "./motion.ts";
import { QUALITY, type Quality } from "./quality.ts";
import { findBones, pitch, roll, type Bones } from "./rig.ts";
import { TextureTiers } from "./textures.ts";
import { Hand, gripOf } from "./weapons.ts";

export type Who = "me" | "enemy";
export type HitKind = "hit" | "blocked" | "grenade";

/** Every figure is scaled to this height, metres. */
const HEIGHT = 1.8;
/** Each step changes the distance by 7%, so every step reads the same on screen. */
const STEP_GROWTH = 1.07;
/** Where the sync connector sits on the nape: share of the head height up, share of it back. */
const PORT = { up: 0.08, depth: 0.45 };
/** The render layer of our own figure. */
const NEAR = 1;

interface Look {
  body: number;
  joints: number;
  jointsRough?: number;
  jointsMetal?: number;
  /** Faint glow that keeps a backlit figure from turning into a hole. */
  lift?: number;
  /** Which way the figure falls: -1 to its right as seen from the camera. */
  fallSide: number;
  /** Our own figure, right in front of the lens: drawn on its own layer and out of focus. */
  near?: boolean;
}

interface Figure {
  group: THREE.Group;
  tilt: THREE.Group;
  model: THREE.Object3D;
  blob: THREE.Mesh;
  mixer: THREE.AnimationMixer;
  motion: Motion;
  /** What it holds in the right hand (stand-in guns for now); null if the rig has no fingers to hold with. */
  hand: Hand | null;
  port: THREE.Mesh;
  kick: Record<Zone, number>;
  /** 0..1 while falling, -1 when standing. */
  fallT: number;
  fallSide: number;
  bones: Bones;
}

interface Mirror {
  group: THREE.Group;
  tilt: THREE.Group;
  pairs: [THREE.Object3D, THREE.Object3D][];
  src: Figure;
  model: THREE.Object3D;
  mats: THREE.Material[];
}

interface Calibrated {
  /** Focal length in picture heights. */
  fn: number;
  aspect: number;
  /** Distance to a figure 7 steps away, metres. */
  d7: number;
  camH: number;
  /** Horizon height as a share of the picture from the top. */
  hz: number;
  /** Sideways offset of the enemy per metre of depth. */
  kx: number;
  me: Frame["cal"]["me"];
}

function calibrate(f: Frame): Calibrated {
  const c = f.cal, aspect = f.w / f.h;
  const fn = 0.5 / Math.tan(THREE.MathUtils.degToRad(c.fov) / 2);
  return {
    fn, aspect, d7: HEIGHT * fn / c.height, camH: c.camH,
    hz: c.feet[1] - c.camH * c.height / HEIGHT,
    kx: (c.feet[0] - 0.5) * aspect / fn,
    me: c.me,
  };
}

function softTexture(stops: [number, string][]): THREE.CanvasTexture {
  const c = document.createElement("canvas"); c.width = c.height = 64;
  const g = c.getContext("2d")!, r = g.createRadialGradient(32, 32, 0, 32, 32, 32);
  for (const [at, col] of stops) r.addColorStop(at, col);
  g.fillStyle = r; g.fillRect(0, 0, 64, 64);
  const t = new THREE.CanvasTexture(c); t.colorSpace = THREE.SRGBColorSpace;
  return t;
}

/** Fetches and parses a figure model: a .glb, or a .json with the .glb inside as base64. */
async function loadModel(url: string): Promise<GLTF> {
  const res = await fetch(url);
  if (!res.ok) throw new Error(`model: ${res.status}`);
  let bytes: ArrayBuffer;
  if (url.endsWith(".json")) {
    const bin = atob(((await res.json()) as { data: string }).data);
    const arr = new Uint8Array(bin.length);
    for (let i = 0; i < bin.length; i++) arr[i] = bin.charCodeAt(i);
    bytes = arr.buffer;
  } else {
    bytes = await res.arrayBuffer();
  }
  return new GLTFLoader().setMeshoptDecoder(MeshoptDecoder).parseAsync(bytes, "");
}

const smooth = (u: number) => u * u * (3 - 2 * u);
const tmpA = new THREE.Vector3(), tmpB = new THREE.Vector3(), tmpSize = new THREE.Vector2();

/** The reflection fades away from the feet, like one in a thin layer of water, and is lit as the body above it is. */
function mirrorShader(sh: THREE.WebGLProgramParametersWithUniforms): void {
  sh.vertexShader = sh.vertexShader.replace("#include <common>", "#include <common>\nvarying float vWorldY;")
    .replace("#include <project_vertex>", "#include <project_vertex>\nvWorldY = (modelMatrix * vec4(transformed, 1.0)).y;");
  sh.fragmentShader = sh.fragmentShader.replace("#include <common>", "#include <common>\nvarying float vWorldY;")
    // flipped upside down, its surfaces face the floor; light them as the real body's surfaces are lit
    .replace("#include <normal_fragment_maps>", "#include <normal_fragment_maps>\n{ vec3 w = transpose(mat3(viewMatrix)) * normal; w.y = -w.y; normal = normalize(mat3(viewMatrix) * w); }")
    .replace("#include <dithering_fragment>", "#include <dithering_fragment>\ngl_FragColor *= clamp(1.0 + vWorldY / 1.1, 0.0, 1.0);");
}

export class Arena {
  /** True once the model is loaded and the figures stand. */
  ready = false;
  /** Called after every rendered frame, e.g. to move the DOM rings with the body. */
  onFrame: (() => void) | null = null;

  private renderer: THREE.WebGLRenderer;
  private scene = new THREE.Scene();
  private camera = new THREE.PerspectiveCamera(45, 1024 / 1536, 0.1, 80);
  private timer = new THREE.Timer();
  private hemi: THREE.HemisphereLight;
  private rim: THREE.DirectionalLight;
  private fill: THREE.SpotLight;
  private meKey: THREE.SpotLight;
  private bounce: THREE.PointLight;
  private spark: THREE.Sprite;
  private sparkLife = 0;
  private sparkMax = 0.18;
  private k: Calibrated | null = null;
  private depth = 0;
  private walk: { from: number; to: number; t: number; dur: number } | null = null;
  private en: Figure | null = null;
  private me: Figure | null = null;
  private mirror: Mirror | null = null;
  private lens: Lens;
  private q: Quality = QUALITY.medium;
  private catcher: THREE.Mesh;
  private dust = new Dust();
  private tiers: TextureTiers[] = [];
  /** The hall's light for reflections on the figures, made from the painting. */
  private env: THREE.Texture | null = null;
  private envSrc = "";
  private size = { w: 1, h: 1, view: { x: 0, y: 0, w: 1, h: 1 } as View };
  private sunDir = new THREE.Vector3(0, 0.5, -1);

  /** Throws when the device has no WebGL. */
  constructor(canvas: HTMLCanvasElement) {
    // the figures are drawn off screen with their own smoothing (lens.ts); the screen itself only gets the finished frame
    this.renderer = new THREE.WebGLRenderer({ canvas, alpha: true, antialias: false, powerPreference: "high-performance" });
    const r = this.renderer;
    r.setClearColor(0x000000, 0);
    r.outputColorSpace = THREE.SRGBColorSpace;
    r.toneMapping = THREE.ACESFilmicToneMapping;
    r.shadowMap.enabled = true;
    r.shadowMap.type = THREE.PCFShadowMap;
    this.lens = new Lens(r);
    this.scene.fog = new THREE.Fog(0x3d342c, 2.5, 30);

    // Light from behind the enemy (the opening in the hall), soft sky and ground; each frame sets the colours.
    const hemi = this.hemi = new THREE.HemisphereLight(0xc9b49a, 0x231e1a, 0.6);
    this.rim = new THREE.DirectionalLight(0xffb06a, 3.4);
    this.rim.castShadow = true;
    this.rim.shadow.mapSize.set(1024, 1024);
    this.rim.shadow.radius = 5;
    this.rim.shadow.bias = -0.0004;
    this.rim.shadow.normalBias = 0.02;
    Object.assign(this.rim.shadow.camera, { left: -5, right: 5, top: 5, bottom: -5, near: 1, far: 40 });
    // A narrow warm spot on the enemy only, and a soft cool one on our own back from over the camera,
    // so our Persona reads as a body, not a hole.
    this.fill = new THREE.SpotLight(0xd9c3a3, 2.4, 0, THREE.MathUtils.degToRad(17), 0.7, 0);
    this.meKey = new THREE.SpotLight(0xb4b4b0, 5, 0, THREE.MathUtils.degToRad(15), 0.8, 0);
    this.bounce = new THREE.PointLight(0x46d8d8, 0.22, 3, 2); // a teal puddle near the enemy, where the frame has one
    const back = new THREE.DirectionalLight(0x8ea3b5, 0.45); // the open hall behind us
    back.position.set(1, 3, 8);
    this.scene.add(hemi, this.rim, this.rim.target, this.fill, this.fill.target, this.meKey, this.meKey.target, this.bounce, back);
    // our own figure is drawn on its own layer, out of focus (lens.ts); the lights shine there too
    for (const l of [hemi, this.rim, this.fill, this.meKey, this.bounce, back]) l.layers.enable(NEAR);

    // Shadows fall on the painted floor through an invisible catcher.
    const catcher = this.catcher = new THREE.Mesh(new THREE.PlaneGeometry(40, 40), new THREE.ShadowMaterial({ color: 0x0a0705, opacity: 0.58, depthWrite: false }));
    catcher.rotation.x = -Math.PI / 2; catcher.receiveShadow = true;
    this.scene.add(catcher, this.dust.points);

    this.spark = new THREE.Sprite(new THREE.SpriteMaterial({
      map: softTexture([[0, "rgba(255,248,230,1)"], [0.25, "rgba(255,190,110,.85)"], [1, "rgba(255,120,40,0)"]]),
      blending: THREE.AdditiveBlending, depthWrite: false, depthTest: false, transparent: true, fog: false,
    }));
    this.spark.visible = false;
    this.scene.add(this.spark);
    this.timer.connect(document);
  }

  /** Loads both figures' models (.glb, or .json with the .glb inside as base64) and starts drawing. */
  async load(meUrl: string, enemyUrl: string, distance: number): Promise<void> {
    const [mine, theirs] = await Promise.all([loadModel(meUrl), enemyUrl === meUrl ? null : loadModel(enemyUrl)]);
    const foe = theirs ?? mine;
    this.en = this.makeFigure(foe, { body: 0x77756f, joints: 0x1c1d1d, fallSide: -1 });
    this.me = this.makeFigure(mine, { body: 0x8a8780, joints: 0x1c1d1d, jointsRough: 0.85, jointsMetal: 0, lift: 0x0a0908, fallSide: -1, near: true });
    this.mirror = this.makeMirror(foe, this.en);
    this.giveWeapons(mine, foe);
    const foeLooks = [this.en.model, this.mirror.model];
    this.tiers = theirs ? [new TextureTiers([this.me.model], meUrl), new TextureTiers(foeLooks, enemyUrl)] : [new TextureTiers([this.me.model, ...foeLooks], meUrl)];
    this.ready = true;
    this.setQuality(this.q);
    this.reset(distance);
    this.renderer.setAnimationLoop((t) => this.tick(t));
  }

  /** The WebGL context, to tell what the device can carry. */
  get gl(): WebGL2RenderingContext {
    return this.renderer.getContext() as WebGL2RenderingContext;
  }

  /** The graphics level (quality.ts): drawing size, smoothing, shadows, reflections, light in the air, textures. */
  setQuality(q: Quality): void {
    this.q = q;
    const r = this.renderer;
    this.lens.setQuality(q);
    r.shadowMap.enabled = q.shadow > 0;
    this.rim.castShadow = q.shadow > 0;
    this.catcher.visible = q.shadow > 0;
    if (this.rim.shadow.mapSize.x !== q.shadow && q.shadow > 0) {
      this.rim.shadow.mapSize.set(q.shadow, q.shadow);
      this.rim.shadow.map?.dispose(); this.rim.shadow.map = null;
    }
    // softness in centimetres stays the same at every map size
    this.rim.shadow.radius = q.shadowRadius;
    this.scene.environment = q.env ? this.env : null;
    // the hall's light takes over from the flat sky light, so the figures stay as bright as before
    this.hemi.intensity = q.env ? 0.22 : 0.6;
    this.dust.setCount(q.dust);
    const mats = new Set<THREE.Material>([this.catcher.material as THREE.Material]);
    for (const f of [this.en, this.me]) f?.model.traverse((o) => { if (o instanceof THREE.Mesh) mats.add(o.material as THREE.Material); });
    for (const m of mats) tune(m, q);
    if (this.mirror) {
      this.mirror.group.visible = q.mirror;
      this.mirror.mats.forEach((m) => tune(m, q, mirrorShader));
    }
    for (const t of this.tiers) void t.set(q.textures, Math.min(q.anisotropy, r.capabilities.getMaxAnisotropy()));
    this.resize(this.size.w, this.size.h, this.size.view);
  }

  setFrame(f: Frame): void {
    const k = this.k = calibrate(f);
    this.camera.position.set(0, k.camH, 0);
    // Off-centre lens: the painted horizon is not in the middle of the picture, verticals stay vertical.
    const n = this.camera.near;
    const top = n * k.hz / k.fn, bottom = -n * (1 - k.hz) / k.fn, half = n * k.aspect / 2 / k.fn;
    this.camera.projectionMatrix.makePerspective(-half, half, top, bottom, n, this.camera.far);
    this.camera.projectionMatrixInverse.copy(this.camera.projectionMatrix).invert();
    this.renderer.toneMappingExposure = f.look.exposure;
    this.lens.setHorizon(k.hz);
    this.lens.setBackdrop(f.src, f.src2x);
    this.lens.setLook(f.look.lens, f.look.exposure);
    this.makeEnv(f.src);
    this.rim.color.set(f.look.rim);
    this.fill.color.set(f.look.fill);
    this.hemi.color.set(f.look.sky); this.hemi.groundColor.set(f.look.ground);
    this.bounce.intensity = f.look.puddle;
    (this.scene.fog as THREE.Fog).color.set(f.look.fog);
  }

  /** The hall's light for the figures' reflections: the painting blurred all round them. */
  private makeEnv(src: string): void {
    if (src === this.envSrc) return;
    this.envSrc = src;
    new THREE.TextureLoader().load(src, (tex) => {
      if (this.envSrc !== src) { tex.dispose(); return; }
      tex.mapping = THREE.EquirectangularReflectionMapping;
      tex.colorSpace = THREE.SRGBColorSpace;
      const pmrem = new THREE.PMREMGenerator(this.renderer);
      const env = pmrem.fromEquirectangular(tex).texture;
      pmrem.dispose(); tex.dispose();
      this.env?.dispose();
      this.env = env;
      if (this.q.env) this.scene.environment = env;
    });
  }

  /** Size of the drawing buffer in CSS pixels (the canvas element is sized by the page), and the part of it on screen. */
  resize(width: number, height: number, view: View = { x: 0, y: 0, w: 1, h: 1 }): void {
    this.size = { w: width, h: height, view };
    // as sharp as the screen allows, within the level's pixel budget
    const q = this.q, area = Math.max(1, width * height);
    const ratio = Math.max(0.5, Math.min(window.devicePixelRatio || 1, q.maxDpr, Math.sqrt(q.maxPixels / area)));
    this.renderer.setPixelRatio(ratio);
    this.renderer.setSize(Math.round(width), Math.round(height), false);
    const px = this.renderer.getDrawingBufferSize(new THREE.Vector2());
    this.lens.setSize(px.x, px.y, width, height, view);
  }

  /** The gun a figure has in hand: its stance and the gun it holds. */
  arm(who: Who, gun: Stance): void {
    if (!this.ready) return;
    (who === "enemy" ? this.en! : this.me!).motion.arm(gun);
  }

  /** Both walk to the new distance; who moved decides who walks, the camera rides with us. */
  move(distance: number, meMove: number, enemyMove: number): void {
    if (!this.ready) return;
    this.walk = { from: this.depth, to: this.depthOf(distance), t: 0, dur: meMove || enemyMove ? 0.75 : 0.01 };
    if (enemyMove) this.en!.motion.walk(enemyMove >= 3 ? "run" : "walk", enemyMove > 0 ? 1 : -1);
    if (meMove) this.me!.motion.walk(meMove >= 3 ? "run" : "walk", meMove > 0 ? 1 : -1);
  }

  /** A figure attacks so that its shot (the stab, the throw) comes `inSeconds` from now, with the sound. */
  attack(who: Who, kind: Attack, inSeconds: number): void {
    if (!this.ready) return;
    (who === "enemy" ? this.en! : this.me!).motion.attack(kind, inSeconds);
  }

  /** A hit lands on a figure: it flinches, and a spark flies off the enemy. */
  hit(who: Who, zone: Zone, kind: HitKind): void {
    if (!this.ready) return;
    const f = who === "enemy" ? this.en! : this.me!;
    f.kick[zone] = Math.min(1.4, f.kick[zone] + (kind === "blocked" ? 0.45 : 1));
    if (kind === "grenade") { f.kick.body = 1; f.kick.legs = 0.8; }
    if (who !== "enemy") return;
    zoneWorld(f, kind === "grenade" ? "legs" : zone, this.spark.position);
    if (kind === "grenade") this.spark.position.y = 0.35;
    this.spark.userData.size = kind === "grenade" ? 1.6 : 0.32;
    this.spark.material.color.set(kind === "blocked" ? 0x7fe6f2 : 0xffffff);
    this.sparkMax = kind === "grenade" ? 0.4 : 0.18;
    this.sparkLife = this.sparkMax;
    this.spark.visible = true;
  }

  fall(who: Who): void {
    if (!this.ready) return;
    const f = who === "enemy" ? this.en! : this.me!;
    f.fallT = 0;
    f.motion.fall();
  }

  reset(distance: number): void {
    if (!this.ready) return;
    this.walk = null;
    this.depth = this.depthOf(distance);
    resetFigure(this.en!, 0.0);
    resetFigure(this.me!, 0.43);
  }

  /** Where a zone of the enemy is right now, as fractions of the canvas (0..1 from the top left). */
  zoneOnScreen(zone: Zone): [number, number] {
    if (!this.ready) return [0.645, { head: 0.26, body: 0.34, groin: 0.42, legs: 0.49 }[zone]];
    zoneWorld(this.en!, zone, tmpA).project(this.camera);
    return this.lens.toScreen((tmpA.x + 1) / 2, (1 - tmpA.y) / 2);
  }

  private depthOf(d: number): number {
    return this.k!.d7 * Math.pow(STEP_GROWTH, d - 7);
  }

  private makeFigure(gltf: GLTF, look: Look): Figure {
    const model = SkeletonUtils.clone(gltf.scene);
    const body = new THREE.MeshStandardMaterial({ color: look.body, roughness: 0.82, metalness: 0.04, emissive: look.lift ?? 0x000000 });
    const joints = new THREE.MeshStandardMaterial({ color: look.joints, roughness: look.jointsRough ?? 0.42, metalness: look.jointsMetal ?? 0.4 });
    model.traverse((o) => {
      if (!(o instanceof THREE.Mesh)) return;
      o.castShadow = true; o.frustumCulled = false;
      const own = o.material as THREE.MeshStandardMaterial;
      // hair strips are cut out by their texture: the antialiasing smooths the cut edges
      if (own.alphaTest > 0) own.alphaToCoverage = true;
      if (own.map) return; // a textured Persona keeps its own suit; the bare mannequin gets painted
      o.material = /joint/i.test(own.name) ? joints : body;
    });
    model.updateMatrixWorld(true);
    const box = new THREE.Box3().setFromObject(model, true);
    const bones = findBones(model, box);
    // a model made in metres keeps its own height (the female Persona is 1.70 m, 1.75 with her hair);
    // anything else (the stand-in mannequin) is brought to a grown man's height
    const tall = box.max.y - box.min.y, s = tall > 1.4 && tall < 2.2 ? 1 : HEIGHT / tall;
    model.scale.multiplyScalar(s);
    model.position.y = -box.min.y * s;

    const tilt = new THREE.Group(); tilt.add(model); // falls around the feet
    const blob = new THREE.Mesh(new THREE.PlaneGeometry(0.95, 0.7), new THREE.MeshBasicMaterial({
      map: softTexture([[0, "rgba(0,0,0,.8)"], [0.55, "rgba(0,0,0,.35)"], [1, "rgba(0,0,0,0)"]]), transparent: true, depthWrite: false, fog: false,
    }));
    blob.rotation.x = -Math.PI / 2; blob.position.y = 0.004;
    const group = new THREE.Group(); group.add(tilt, blob);
    this.scene.add(group);

    const mixer = new THREE.AnimationMixer(model);
    const motion = new Motion(mixer, gltf.animations);

    // a small cyan light on the sync connector, as in the concept frames
    const port = new THREE.Mesh(new THREE.SphereGeometry(0.016, 12, 8), new THREE.MeshBasicMaterial({ color: 0x7ff3ff, fog: false }));
    this.scene.add(port);

    // right in front of the lens the light is a pinpoint, not a lamp
    if (look.near) { group.traverse((o) => o.layers.set(NEAR)); port.layers.set(NEAR); port.scale.setScalar(0.4); }

    return {
      group, tilt, model, blob, mixer, motion, hand: null, port,
      kick: { head: 0, body: 0, groin: 0, legs: 0 }, fallT: -1, fallSide: look.fallSide, bones,
    };
  }

  /** The wet floor reflects the enemy: a faint upside-down copy in its own lit materials that repeats its pose every frame. */
  private makeMirror(gltf: GLTF, src: Figure): Mirror {
    const model = SkeletonUtils.clone(gltf.scene);
    // the same materials as the body it mirrors (two copies of one model list their meshes in the same order)
    const own: THREE.Mesh[] = [];
    src.model.traverse((o) => { if (o instanceof THREE.Mesh) own.push(o); });
    const copies = new Map<THREE.Material, THREE.MeshStandardMaterial>();
    let i = 0;
    model.traverse((o) => {
      if (!(o instanceof THREE.Mesh)) return;
      const from = (own[i++]?.material ?? o.material) as THREE.MeshStandardMaterial;
      let m = copies.get(from);
      if (!m) {
        // the water darkens it a little
        m = from.clone(); m.color.multiplyScalar(0.62); m.transparent = true; m.opacity = 0.5; m.depthWrite = false;
        copies.set(from, m);
      }
      o.material = m; o.frustumCulled = false; o.castShadow = false;
    });
    model.scale.copy(src.model.scale); model.position.copy(src.model.position);
    const flip = new THREE.Group(); flip.add(model); flip.scale.y = -1;
    const tilt = new THREE.Group(); tilt.add(flip);
    const group = new THREE.Group(); group.add(tilt);
    this.scene.add(group);
    const a: THREE.Object3D[] = [], b: THREE.Object3D[] = [];
    src.model.traverse((o) => { if ((o as THREE.Bone).isBone) a.push(o); });
    model.traverse((o) => { if ((o as THREE.Bone).isBone) b.push(o); });
    return { group, tilt, model, pairs: a.map((bone, i) => [bone, b[i]!]), src, mats: [...copies.values()] };
  }

  /** Stand-in guns in both right hands (and in the enemy's reflection), shown as each figure arms itself. */
  private giveWeapons(mine: GLTF, foe: GLTF): void {
    const steel = new THREE.MeshStandardMaterial({ color: 0x1b1c1d, roughness: 0.45, metalness: 0.55 });
    const myGrip = gripOf(mine.scene), foeGrip = gripOf(foe.scene);
    const me = this.me!, en = this.en!, mirror = this.mirror!;
    if (myGrip) me.hand = new Hand(me.model, myGrip, steel, NEAR);
    if (foeGrip) {
      en.hand = new Hand(en.model, foeGrip, steel);
      // the reflection's copy, darkened by the water like the rest of it
      const wet = steel.clone(); wet.color.multiplyScalar(0.62); wet.transparent = true; wet.opacity = 0.5; wet.depthWrite = false;
      mirror.mats.push(wet);
      const inWater = new Hand(mirror.model, foeGrip, wet);
      en.motion.onHold = (w) => { en.hand!.hold(w); inWater.hold(w); };
    }
    me.motion.onHold = (w) => me.hand?.hold(w);
  }

  private place(): void {
    const k = this.k!, D = this.depth, en = this.en!, me = this.me!;
    const ex = k.kx * D, ez = -D;
    en.group.position.set(ex, 0, ez);
    en.group.rotation.y = Math.atan2(-ex, D); // faces the camera
    const mx = k.me.x, mz = k.me.z;
    me.group.position.set(mx, k.me.y, mz);
    me.group.rotation.y = Math.atan2(ex - mx, ez - mz); // faces the enemy
    this.rim.position.set(ex - 4, 7.5, ez - 14); this.rim.target.position.set(ex, 0.9, ez);
    this.fill.position.set(ex - 2, 3.4, ez + 3.2); this.fill.target.position.set(ex, 1, ez);
    this.meKey.position.set(mx - 1, 3.4, mz + 3); this.meKey.target.position.set(mx, 1.5, mz);
    this.bounce.position.set(ex - 0.2, 0.12, ez + 0.9);
  }

  private tick(time: number): void {
    this.timer.update(time);
    // Slow devices skip time instead of jumping: animations stay smooth, just slower.
    const dt = Math.min(this.timer.getDelta(), 0.05);
    const w = this.walk;
    if (w) {
      w.t += dt;
      const u = Math.min(1, w.t / w.dur);
      this.depth = w.from + (w.to - w.from) * smooth(u);
      if (u >= 1) {
        this.walk = null;
        for (const f of [this.en!, this.me!]) if (f.fallT < 0) f.motion.stand(0.35);
      }
    }
    this.place();
    for (const f of [this.en!, this.me!]) { f.motion.update(dt); f.mixer.update(dt); applyReactions(f, dt); }
    syncMirror(this.mirror!);
    this.scene.updateMatrixWorld();
    placePort(this.en!); placePort(this.me!);
    if (this.sparkLife > 0) {
      this.sparkLife = Math.max(0, this.sparkLife - dt);
      const u = 1 - this.sparkLife / this.sparkMax;
      this.spark.material.opacity = 1 - u;
      this.spark.scale.setScalar((this.spark.userData.size as number) * (0.6 + 0.8 * u));
      this.spark.visible = this.sparkLife > 0;
    }
    if (this.q.dust) {
      this.lens.sunDirection(this.camera, this.sunDir);
      const pxPerMetre = this.renderer.getDrawingBufferSize(tmpSize).y * this.k!.fn;
      this.dust.update(this.timer.getElapsed(), pxPerMetre, this.sunDir, this.lens.raysTexture);
    }
    this.lens.render(this.scene, this.camera, NEAR, { enemyDepth: this.depth, light: this.rim });
    this.onFrame?.();
  }
}

function resetFigure(f: Figure, offset: number): void {
  f.fallT = -1; f.tilt.rotation.set(0, 0, 0);
  f.blob.scale.set(1, 1, 1); f.blob.position.x = 0;
  for (const z of Object.keys(f.kick) as Zone[]) f.kick[z] = 0;
  f.motion.reset(offset);
}

function syncMirror(m: Mirror): void {
  m.group.position.copy(m.src.group.position); m.group.rotation.copy(m.src.group.rotation);
  const r = m.src.tilt.rotation;
  m.tilt.rotation.set(-r.x, r.y, -r.z); // mirrored in the floor
  for (const [a, b] of m.pairs) { b.position.copy(a.position); b.quaternion.copy(a.quaternion); }
}

/** Hit reactions and the fall, layered on top of whatever the animation is doing this frame. */
function applyReactions(f: Figure, dt: number): void {
  const k = f.kick, b = f.bones, decay = Math.exp(-dt * 6.5);
  if (k.head) { pitch(b.head, -0.6 * k.head); pitch(b.neck, -0.25 * k.head); }
  if (k.body) { pitch(b.spine2, -0.32 * k.body); pitch(b.spine1, -0.22 * k.body); }
  if (k.groin) { pitch(b.spine, 0.42 * k.groin); pitch(b.spine1, 0.2 * k.groin); }
  if (k.legs) { pitch(b.rUpLeg, 0.35 * k.legs); pitch(b.rLeg, -0.75 * k.legs); }
  for (const z of Object.keys(k) as Zone[]) k[z] = k[z] * decay < 0.01 ? 0 : k[z] * decay;

  if (f.fallT >= 0) {
    // falls on its side: from this low camera a body lying sideways reads best
    f.fallT = Math.min(f.fallT + dt / 0.9, 1);
    const t = f.fallT, e = smooth(t), g = t * t, side = f.fallSide;
    pitch(b.lUpLeg, 0.35 * e); pitch(b.lLeg, -0.5 * e);
    pitch(b.rUpLeg, 0.2 * e); pitch(b.rLeg, -0.3 * e);
    pitch(b.spine, 0.25 * e); pitch(b.spine2, 0.15 * e); roll(b.head, -side * 0.35 * e);
    f.tilt.rotation.set(-0.25 * g, 0, side * 1.5 * g);
    f.blob.scale.set(1 + 1.9 * g, 1, 1.1); f.blob.position.x = -side * 0.85 * g;
  }
}

/** The sync connector sits on the nape, so it shows on our Persona and hides behind the enemy's head. */
function placePort(f: Figure): void {
  f.bones.head.getWorldPosition(tmpA); f.bones.top.getWorldPosition(tmpB);
  const r = tmpA.distanceTo(tmpB) * PORT.depth, yaw = f.group.rotation.y;
  tmpA.lerp(tmpB, PORT.up);
  f.port.position.set(tmpA.x - Math.sin(yaw) * r, tmpA.y, tmpA.z - Math.cos(yaw) * r);
  f.port.visible = f.fallT < 0;
}

/** A point on the body for each attack zone. */
function zoneWorld(f: Figure, z: Zone, out: THREE.Vector3): THREE.Vector3 {
  const b = f.bones;
  if (z === "head") { b.head.getWorldPosition(out); b.top.getWorldPosition(tmpB); return out.lerp(tmpB, 0.45); }
  if (z === "body") { b.spine1.getWorldPosition(out); b.spine2.getWorldPosition(tmpB); return out.lerp(tmpB, 0.75); }
  if (z === "groin") { b.hips.getWorldPosition(out); out.y -= 0.08; return out; }
  b.rLeg.getWorldPosition(out); b.rUpLeg.getWorldPosition(tmpB);
  return out.lerp(tmpB, 0.15);
}
