// Live 3D figures standing in a painted frame. The three.js canvas lies exactly
// over the picture, an off-centre lens matched to the painting puts the figures'
// feet on the painted floor, and the bodycam lens (lens.ts) lays them over the
// painting and shoots both through the same glass. The fight code only calls arm, move,
// attack, hit, fall and reset; everything else here is about looking right.

import type { Zone } from "@crowned-dogs/core";
import * as THREE from "three";
import type { GLTF } from "three/addons/loaders/GLTFLoader.js";
import * as SkeletonUtils from "three/addons/utils/SkeletonUtils.js";
import { dress, KIT_A } from "./dress.ts";
import { Dust } from "./dust.ts";
import type { Frame } from "./frames.ts";
import { Lens, type View } from "./lens.ts";
import { tune } from "./materials.ts";
import { Motion, Pose, type Attack, type Stance } from "./motion.ts";
import { QUALITY, type Quality } from "./quality.ts";
import { armsOnly, findBones, pitch, type Bones } from "./rig.ts";
import { Collapse, FALL_TIME } from "./fall.ts";
import { Flash, HoldOut } from "./hold.ts";
import { Feet, gaitOf, type Gait } from "./ik.ts";
import { Viewmodel } from "./viewmodel.ts";
import { TextureTiers } from "./textures.ts";
import { loadModel } from "./load-model.ts";
import { Hand, gripOf, loadGuns, type Guns } from "./weapons.ts";

export type Who = "me" | "enemy";
export type HitKind = "hit" | "blocked" | "grenade";

/** Every figure is scaled to this height, metres. */
const HEIGHT = 1.8;
/** Each step changes the distance by 7%, so every step reads the same on screen. */
const STEP_GROWTH = 1.07;
/** Where the sync connector sits on the nape: share of the head height up, share of it back. */
const PORT = { up: 0.08, depth: 0.45 };
/** First person: where the lens sits in our figure's own frame for each stance (metres: x to its left, y up,
 *  z forward; the head is folded away), and how far our figure is tilted back about the lens, radians, so the
 *  arms and the gun come up into the picture. The camera itself never turns: the painted hall must match. */
type Spot = { at: [number, number, number]; tilt: number };
const VIEW: Record<Stance, Spot> & Partial<Record<Attack, Spot>> = {
  // with a gun the lens is a bodycam between the shoulders; the arms reach for the gun (viewmodel.ts)
  rifle: { at: [0, 1.57, 0.04], tilt: 0 },
  shotgun: { at: [0, 1.57, 0.04], tilt: 0 },
  sniper: { at: [0, 1.57, 0.04], tilt: 0 },
  pistol: { at: [0, 1.57, 0.04], tilt: 0 },
  // during a stab or a throw the lens glides to where the hand goes: the knife thrusts low on the right,
  // the grenade leaves the hand low on the left
  knife: { at: [-0.2, 1.36, 0.12], tilt: 0.3 },
  grenade: { at: [0.17, 1.47, 0.12], tilt: 0.53 },
};
{ // ?fpview=x,y,z,tilt tries another lens spot for every stance (tuning)
  const t = new URLSearchParams(location.search).get("fpview")?.split(",").map(Number);
  if (t?.length === 4) { const [x = 0, y = 0, z = 0, tilt = 0] = t; for (const v of Object.values(VIEW)) { v.at = [x, y, z]; v.tilt = tilt; } }
}
/** How big each gun's muzzle flash is, metres across. */
const MUZZLE: Record<Stance, number> = { pistol: 0.16, rifle: 0.24, shotgun: 0.3, sniper: 0.28 };
/** How much wider the first-person lens on our own arms is than the hall's (a squeeze of the picture). */
const WIDE = Number(new URLSearchParams(location.search).get("wide") ?? 0.62);
const FPKEY = Number(new URLSearchParams(location.search).get("fpkey") ?? 2.5);
/** The render layer of our own figure. */
const NEAR = 1;

interface Look {
  body: number;
  joints: number;
  jointsRough?: number;
  jointsMetal?: number;
  /** Faint glow that keeps a backlit figure from turning into a hole. */
  lift?: number;
  /** Which way the figure falls: 1 towards its own left (+x of its group), -1 towards its right. */
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
  /** The mixer's own pose of the bones, before anything is turned on top of it. */
  pose: Pose;
  motion: Motion;
  /** What it holds in the right hand (stand-in guns for now); null if the rig has no fingers to hold with. */
  hand: Hand | null;
  /** Keeps its feet flat on the floor while it stands. */
  feet: Feet;
  /** How its walk and run loops cover the ground, to keep its feet in step with it. */
  gait: Record<"walk" | "run", Gait | null>;
  /** The fall: folds the body down onto the floor where it stood. */
  collapse: Collapse;
  /** Brings a pistol held in at the chest out clear of the body. */
  holdOut: HoldOut;
  /** The flash at its gun's muzzle as it fires. */
  flash: Flash;
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
  back: number;
}

/** The live camera for a painted frame. In first person it sits lower, at our eyes, under the same painted
 *  horizon, and the enemy stands a little closer. */
function calibrate(f: Frame, fp = false): Calibrated {
  const c = f.cal, aspect = f.w / f.h;
  const fn = 0.5 / Math.tan(THREE.MathUtils.degToRad(c.fov) / 2);
  return {
    fn, aspect, d7: HEIGHT * fn / c.height * (fp ? c.fp.near : 1), camH: fp ? c.fp.camH : c.camH,
    hz: c.feet[1] - c.camH * c.height / HEIGHT,
    kx: (c.feet[0] - 0.5) * aspect / fn,
    me: c.me, back: fp ? 0 : c.back,
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

const smooth = (u: number) => u * u * (3 - 2 * u);
const aimAt = new THREE.Vector3(), tmpA = new THREE.Vector3(), tmpB = new THREE.Vector3(), tmpSize = new THREE.Vector2();

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
  private running = false;
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
  /** The muzzle flash's light, for whichever figure fired last. */
  private muzzleLight = new THREE.PointLight(0xffb36b, 0, 4, 2);
  private flashOf: Figure | null = null;
  private k: Calibrated | null = null;
  private frame: Frame | null = null;
  /** The distance in steps the figures stand at (or walk to). */
  private dist = 7;
  private depth = 0;
  /** The walk under way: the enemy's depth from and to, the time and length (s), and how far we go ourselves (m, + ahead). */
  private walk: { from: number; to: number; t: number; dur: number; mine: number } | null = null;
  private en: Figure | null = null;
  private me: Figure | null = null;
  private fp = false;
  /** Our stance, and where the lens sits on us now (it glides to the stance's VIEW). */
  private myGun: Stance = "pistol";
  private view = { at: new THREE.Vector3(), tilt: 0, set: false };
  /** Our own steps, shown as the body rising and swaying (a walk loop played on the spot this close to the lens
   *  swings the head about): how many steps along (x pi), how strong (0..1) and how deep a dip (m). */
  private bob = { phase: 0, k: 0, depth: 0.02 };
  /** Shows only our arms (first person) or the whole of us. */
  private armsOnly: ((on: boolean) => void) | null = null;
  /** First person: our arms on the gun. */
  private viewmodel: Viewmodel | null = null;
  /** The enemy's gun in its reflection. */
  private inWater: Hand | null = null;
  /** First person: our own figure is drawn as through a wider lens than the painted hall's, the way a
   *  bodycam sees the arms (its picture squeezed `k` times towards the lens axis, at NDC height `y`). */
  private wideK = { value: 1 };
  private wideY = { value: 0 };
  private widen = (shader: THREE.WebGLProgramParametersWithUniforms): void => {
    shader.uniforms.wideK = this.wideK; shader.uniforms.wideY = this.wideY;
    shader.vertexShader = "uniform float wideK;\nuniform float wideY;\n" + shader.vertexShader.replace("#include <project_vertex>",
      "#include <project_vertex>\n\tgl_Position.xy = mix(vec2(0.0, wideY) * gl_Position.w, gl_Position.xy, wideK);");
  };
  private mirror: Mirror | null = null;
  private lens: Lens;
  private q: Quality = QUALITY.medium;
  private catcher: THREE.Mesh;
  private dust = new Dust();
  private tiers: TextureTiers[] = [];
  /** The textures of the current quality level, once they are all on the figures. */
  texturesDone: Promise<unknown> = Promise.resolve();
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
    this.muzzleLight.layers.enable(NEAR);
    this.scene.add(this.muzzleLight);
    this.timer.connect(document);
  }

  /** Loads both figures' models (.glb, or .json with the .glb inside as base64) and starts drawing. */
  async load(meUrl: string, enemyUrl: string, distance: number): Promise<void> {
    const [mine, theirs, guns] = await Promise.all([loadModel(meUrl), enemyUrl === meUrl ? null : loadModel(enemyUrl), loadGuns()]);
    const foe = theirs ?? mine;
    this.en = this.makeFigure(foe, { body: 0x77756f, joints: 0x1c1d1d, fallSide: -1 });
    this.me = this.makeFigure(mine, { body: 0x8a8780, joints: 0x1c1d1d, jointsRough: 0.85, jointsMetal: 0, lift: 0x0a0908, fallSide: 1, near: true });
    this.me.blob.visible = !this.fp;
    this.armsOnly = armsOnly(this.me.model);
    this.armsOnly(this.fp);
    this.meKey.intensity = this.fp ? FPKEY : 5;
    this.mirror = this.makeMirror(foe, this.en);
    this.giveWeapons(mine, foe, guns);
    const foeLooks = [this.en.model, this.mirror.model];
    this.tiers = theirs ? [new TextureTiers([this.me.model], meUrl), new TextureTiers(foeLooks, enemyUrl)] : [new TextureTiers([this.me.model, ...foeLooks], meUrl)];
    this.ready = true;
    this.setQuality(this.q);
    this.reset(distance);
    if (this.running) this.resume();
  }

  /** May be called before loading finishes. A hidden base never keeps a render loop alive. */
  resume(): void {
    this.running = true;
    if (!this.ready) return;
    this.timer.reset();
    this.renderer.setAnimationLoop((time) => this.tick(time));
  }

  pause(): void {
    this.running = false;
    this.renderer.setAnimationLoop(null);
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
    const mats = new Set<THREE.Material>([this.catcher.material as THREE.Material]), mine = new Set<THREE.Material>();
    this.en?.model.traverse((o) => { if (o instanceof THREE.Mesh) mats.add(o.material as THREE.Material); });
    this.me?.model.traverse((o) => { if (o instanceof THREE.Mesh) mine.add(o.material as THREE.Material); });
    for (const m of mats) tune(m, q);
    for (const m of mine) tune(m, q, this.widen, "wide");
    if (this.mirror) {
      this.mirror.group.visible = q.mirror;
      this.mirror.mats.forEach((m) => tune(m, q, mirrorShader));
    }
    this.texturesDone = Promise.all(this.tiers.map((t) => t.set(q.textures, Math.min(q.anisotropy, r.capabilities.getMaxAnisotropy())).catch(() => {})));
    this.resize(this.size.w, this.size.h, this.size.view);
  }

  setFrame(f: Frame): void {
    this.frame = f;
    const k = this.k = calibrate(f, this.fp);
    this.camera.position.set(0, k.camH, 0);
    // Off-centre lens: the painted horizon is not in the middle of the picture, verticals stay vertical.
    const n = this.camera.near;
    const top = n * k.hz / k.fn, bottom = -n * (1 - k.hz) / k.fn, half = n * k.aspect / 2 / k.fn;
    this.camera.projectionMatrix.makePerspective(-half, half, top, bottom, n, this.camera.far);
    this.camera.projectionMatrixInverse.copy(this.camera.projectionMatrix).invert();
    this.renderer.toneMappingExposure = f.look.exposure;
    this.lens.setHorizon(k.hz);
    this.lens.setBackdrop(f.src, f.src2x, f.depth);
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

  /** The gun a figure has in hand: its stance and the gun it holds, with the kit's mods (slot → item id). */
  arm(who: Who, gun: Stance, mods: Partial<Record<string, string>> = {}): void {
    if (!this.ready) return;
    const f = who === "enemy" ? this.en! : this.me!;
    f.motion.arm(gun);
    f.hand?.fit(mods);
    if (who === "enemy") this.inWater?.fit(mods);
    if (who === "me") this.myGun = gun;
  }

  /** Both walk to the new distance. The camera rides with us and the painted floor cannot move, so the enemy
   *  covers on foot all the floor between its old spot and its new one, whoever stepped: its feet keep pace
   *  with the ground and never slide. Our own steps show as the body bobbing (our feet are out of the picture). */
  move(distance: number, meMove: number, enemyMove: number): void {
    if (!this.ready) return;
    const from = this.depth, to = this.depthOf(distance), ground = Math.abs(to - from);
    const mine = meMove ? this.depthOf(this.dist) - this.depthOf(this.dist - meMove) : 0;
    this.dist = distance;
    if (ground < 0.01 && !meMove) { this.walk = null; this.depth = to; return; }
    // a step is a walk at an easy pace, two steps ahead a run, a dash a sprint; backing off is a walk, never quick
    const ahead = to < from, far = Math.max(ground, Math.abs(mine));
    const cruise = !ahead ? Math.min(1.5, far / 0.45) : far < 0.8 ? far / 0.47 : far < 1.4 ? 2.5 : 3;
    const dur = 0.2 + far / Math.max(cruise, 0.1);
    this.walk = { from, to, t: 0, dur, mine };
    if (ground >= 0.01) this.stride(this.en!, ahead ? ground : -ground, dur);
    if (meMove) this.bob.depth = Math.abs(mine) / (dur - ramp(dur)) > 2 ? 0.035 : 0.02;
  }

  /** Starts a figure's walk (or run) over `ground` metres (negative: backwards) in `dur` seconds, from the
   *  moment of the loop that brings it to a stop just as the free foot passes the planted one. */
  private stride(f: Figure, ground: number, dur: number): void {
    const cruise = Math.abs(ground) / (dur - ramp(dur)), w = f.gait.walk, r = f.gait.run, s = f.model.scale.x;
    const run = !!r && ground > 0 && (!w || cruise > Math.sqrt(w.speed * r.speed) * s);
    const g = run ? r : w;
    if (g) f.motion.walk(run ? "run" : "walk", g.passing - ground / (g.speed * s));
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

  /** A figure dies: its knees give and it folds down on its side where it stands. */
  fall(who: Who): void {
    if (!this.ready) return;
    const f = who === "enemy" ? this.en! : this.me!;
    if (f.fallT >= 0) return;
    f.fallT = 0;
    f.motion.fall();
  }

  /** First person (Art's bodycam): the camera is our eyes, lower than the painted camera, the enemy a little
   *  closer, and of our own figure only the arms with the weapon show at the bottom of the picture. */
  setFirstPerson(on: boolean): void {
    if (on === this.fp) return;
    this.fp = on;
    if (this.frame) { this.k = calibrate(this.frame, on); this.camera.position.set(0, this.k.camH, 0); this.camera.rotation.set(0, 0, 0); }
    if (!this.ready) return;
    this.walk = null;
    this.depth = this.depthOf(this.dist);
    const me = this.me!;
    me.blob.visible = !on;
    this.armsOnly?.(on);
    this.meKey.intensity = on ? FPKEY : 5;
    this.view.set = false;
    if (!on) { me.group.visible = true; me.group.rotation.x = 0; }
  }

  reset(distance: number): void {
    if (this.k) { this.camera.position.set(0, this.k.camH, 0); this.camera.rotation.set(0, 0, 0); }
    if (!this.ready) return;
    this.walk = null; this.dist = distance;
    this.sparkLife = 0;
    this.spark.visible = false;
    for (const f of [this.en!, this.me!]) f.flash.stop();
    this.muzzleLight.intensity = 0;
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
    return this.k!.d7 * Math.pow(STEP_GROWTH, d - 7) + this.k!.back;
  }

  private makeFigure(gltf: GLTF, look: Look): Figure {
    const model = SkeletonUtils.clone(gltf.scene);
    dress(model, KIT_A); // for now both fighters wear the one kit there is
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
    const feet = new Feet(model, gltf.animations, motion.stances);
    const gait = { walk: gaitIn(model, gltf, "walk"), run: gaitIn(model, gltf, "run") };

    // a small cyan light on the sync connector, as in the concept frames
    const port = new THREE.Mesh(new THREE.SphereGeometry(0.016, 12, 8), new THREE.MeshBasicMaterial({ color: 0x7ff3ff, fog: false }));
    this.scene.add(port);

    // right in front of the lens the light is a pinpoint, not a lamp
    if (look.near) { group.traverse((o) => o.layers.set(NEAR)); port.layers.set(NEAR); port.scale.setScalar(0.4); }

    const flash = new Flash(softTexture([[0, "rgba(255,250,235,1)"], [0.18, "rgba(255,214,150,.9)"], [0.5, "rgba(255,150,60,.35)"], [1, "rgba(255,110,30,0)"]]), look.near ? NEAR : 0);
    this.scene.add(flash.sprite);

    return {
      group, tilt, model, blob, mixer, pose: new Pose(model), motion, hand: null, feet, gait, collapse: new Collapse(model), holdOut: new HoldOut(model), flash, port,
      kick: { head: 0, body: 0, groin: 0, legs: 0 }, fallT: -1, fallSide: look.fallSide, bones,
    };
  }

  /** The wet floor reflects the enemy: a faint upside-down copy in its own lit materials that repeats its pose every frame. */
  private makeMirror(gltf: GLTF, src: Figure): Mirror {
    const model = SkeletonUtils.clone(gltf.scene);
    dress(model, KIT_A);
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

  /** Guns in both right hands (and in the enemy's reflection), shown as each figure arms itself: the real ones in
   *  `guns`, stand-ins for the rest. */
  private giveWeapons(mine: GLTF, foe: GLTF, guns: Guns): void {
    const steel = new THREE.MeshStandardMaterial({ color: 0x1b1c1d, roughness: 0.45, metalness: 0.55 });
    const myGrip = gripOf(mine.scene), foeGrip = gripOf(foe.scene);
    const me = this.me!, en = this.en!, mirror = this.mirror!;
    // our gun has its own copy: it is drawn through the first-person wide lens
    if (myGrip) { me.hand = new Hand(me.model, myGrip, steel.clone(), { layer: NEAR, guns, look: (m) => m.clone() }); me.collapse.gun = me.hand.mount; }
    if (foeGrip) {
      en.hand = new Hand(en.model, foeGrip, steel, { guns });
      en.collapse.gun = en.hand.mount;
      // the reflection's copy, darkened by the water like the rest of it
      const wetten = (m: THREE.Material) => {
        const w = m.clone() as THREE.MeshStandardMaterial;
        w.color.multiplyScalar(0.62); w.transparent = true; w.opacity = 0.5; w.depthWrite = false;
        mirror.mats.push(w);
        return w;
      };
      const inWater = this.inWater = new Hand(mirror.model, foeGrip, wetten(steel), { guns, look: wetten });
      en.motion.onHold = (w) => { en.hand!.hold(w); inWater.hold(w); };
    }
    me.motion.onHold = (w) => me.hand?.hold(w);
    const vm = this.viewmodel = me.hand ? new Viewmodel(me.model, me.hand) : null;
    for (const f of [me, en]) {
      f.motion.onMark = (kind) => {
        if (kind === "knife" || kind === "grenade" || !f.hand) return;
        f.flash.fire(f.hand, MUZZLE[kind]);
        this.flashOf = f;
        if (f === me) vm?.shot();
      };
    }
  }

  private place(): void {
    const k = this.k!, D = this.depth, en = this.en!, me = this.me!;
    const ex = k.kx * D, ez = -D;
    en.group.position.set(ex, 0, ez);
    en.group.rotation.y = Math.atan2(-ex, D); // faces the camera
    if (this.fp) return this.placeView(ex, ez);
    // stepped back, our Persona stays in the same corner of the picture, only smaller
    const mz = k.me.z - k.back, mx = k.me.x * mz / k.me.z;
    me.group.position.set(mx, k.me.y, mz);
    me.group.rotation.y = Math.atan2(ex - mx, ez - mz); // faces the enemy
    this.rim.position.set(ex - 4, 7.5, ez - 14); this.rim.target.position.set(ex, 0.9, ez);
    this.fill.position.set(ex - 2, 3.4, ez + 3.2); this.fill.target.position.set(ex, 1, ez);
    this.meKey.position.set(mx - 1, 3.4, mz + 3); this.meKey.target.position.set(mx, 1.5, mz);
    this.bounce.position.set(ex - 0.2, 0.12, ez + 0.9);
  }

  /** First person: our figure faces the enemy from the camera; holdView() then puts its eyes on the lens. */
  private placeView(ex: number, ez: number): void {
    const me = this.me!, c = this.camera.position;
    me.group.rotation.y = Math.atan2(ex - c.x, ez - c.z);
    me.group.position.set(c.x, 0, c.z);
    this.rim.position.set(ex - 4, 7.5, ez - 14); this.rim.target.position.set(ex, 0.9, ez);
    this.fill.position.set(ex - 2, 3.4, ez + 3.2); this.fill.target.position.set(ex, 1, ez);
    // the arms catch the light of the windows ahead, on their edges
    this.meKey.position.set(c.x + 1.0, c.y + 1.5, c.z - 2.5); this.meKey.target.position.set(c.x + 0.1, c.y - 0.3, c.z - 0.5);
    this.bounce.position.set(ex - 0.2, 0.12, ez + 0.9);
  }

  /** First person: puts our figure around the lens, as VIEW says for our stance (tilted back about it, so the
   *  arms and the gun show low in the picture). When we fall the camera stays where it is: it is matched to
   *  the painting, and turning it alone would swing the enemy off the painted floor. The page tips the whole
   *  picture instead (main.ts, .stage.down). */
  private holdView(dt: number): void {
    const me = this.me!, g = me.group, v = this.view;
    if (me.fallT >= 0) { g.visible = false; return; }
    g.visible = true;
    const act = me.motion.attackKind, want = (act && VIEW[act]) || VIEW[this.myGun], k = v.set ? 1 - Math.exp(-dt * 6) : 1;
    v.at.lerp(tmpB.fromArray(want.at), k); v.tilt += (want.tilt - v.tilt) * k; v.set = true;
    g.rotation.set(-v.tilt, g.rotation.y, 0, "YXZ");
    g.position.copy(this.camera.position).sub(tmpA.copy(v.at).applyEuler(g.rotation));
  }

  /** Our body dips once a step (lowest as the foot lands) and sways a little from foot to foot, as fast as
   *  we cover the ground, `speed` m/s; it settles back when we stop. */
  private step(dt: number, speed: number): void {
    const b = this.bob, me = this.me!;
    b.phase += (speed * dt * Math.PI) / 0.7;
    b.k += ((speed > 0.05 && me.fallT < 0 ? 1 : 0) - b.k) * (1 - Math.exp(-dt * 8));
    if (b.k < 1e-3) { b.k = 0; b.phase = 0; }
    const s = Math.sin(b.phase);
    me.tilt.position.set(0.012 * b.k * s, -b.depth * b.k * s * s, 0);
  }

  private tick(time: number): void {
    this.timer.update(time);
    // Slow devices skip time instead of jumping: animations stay smooth, just slower.
    const dt = Math.min(this.timer.getDelta(), 0.05);
    const w = this.walk;
    if (w) {
      w.t = Math.min(w.t + dt, w.dur);
      const [s, v] = travel(w.t, w.dur);
      this.depth = w.from + (w.to - w.from) * s;
      // each walk loop plays exactly as fast as its figure covers the ground (the enemy faces us: nearer is ahead)
      pace(this.en!, (w.from - w.to) * v);
      if (w.t >= w.dur) {
        this.walk = null;
        for (const f of [this.en!, this.me!]) if (f.fallT < 0) f.motion.stand(0.35);
      }
    }
    this.place();
    this.step(dt, w ? Math.abs(w.mine * travel(w.t, w.dur)[1]) : 0);
    for (const f of [this.en!, this.me!]) {
      f.pose.restore(); f.motion.update(dt); f.mixer.update(dt); f.pose.save(); f.motion.settle();
      if (!(this.fp && f === this.me)) f.holdOut.apply(f.group, f.motion.pistolReady);
      if (!(this.fp && f === this.me)) f.feet.update(dt, f.motion.planted);
      applyReactions(f, dt);
    }
    this.wideK.value = this.fp ? WIDE : 1; this.wideY.value = 1 - 2 * this.k!.hz;
    if (this.fp) {
      this.holdView(dt);
      const me = this.me!, act = me.motion.attackKind;
      const gun = me.fallT < 0 && act !== "knife" && act !== "grenade";
      this.viewmodel?.update(dt, gun, gun && !!act, this.myGun, this.camera, zoneWorld(this.en!, "body", aimAt));
    } else this.viewmodel?.off();
    syncMirror(this.mirror!);
    this.scene.updateMatrixWorld();
    placePort(this.en!); placePort(this.me!);
    if (this.fp) this.me!.port.visible = false;
    // the muzzle flashes after everything has moved, so it sits on the muzzle where it is drawn
    const lit = Math.max(this.en!.flash.update(dt), this.me!.flash.update(dt));
    if (this.flashOf && lit > 0) this.muzzleLight.position.copy(this.flashOf.flash.sprite.position);
    this.muzzleLight.intensity = 6 * lit;
    if (this.sparkLife > 0) {
      this.sparkLife = Math.max(0, this.sparkLife - dt);
      const u = 1 - this.sparkLife / this.sparkMax;
      this.spark.material.opacity = 1 - u;
      this.spark.scale.setScalar((this.spark.userData.size as number) * (0.6 + 0.8 * u));
      this.spark.visible = this.sparkLife > 0;
    }
    if (this.q.dust) {
      this.lens.sunDirection(this.camera, this.sunDir);
      const height = this.renderer.getDrawingBufferSize(tmpSize).y;
      this.dust.update(this.timer.getElapsed(), height * this.k!.fn, this.sunDir, this.lens.raysTexture, 1 / Math.max(0.5, this.depth), height * this.lens.dof);
    }
    this.lens.render(this.scene, this.camera, NEAR, { enemyDepth: this.depth, light: this.rim });
    this.onFrame?.();
  }
}

/** How long a walk takes to get up to pace, and to stop, seconds. */
const ramp = (dur: number) => Math.min(0.2, dur / 3);

/** A walk of `dur` seconds, `t` seconds in: the share of the way covered and the speed (shares per second).
 *  It gets up to pace and stops in a fifth of a second, as feet do, and keeps an even pace in between. */
function travel(t: number, dur: number): [number, number] {
  const r = ramp(dur), v = 1 / (dur - r);
  if (t < r) return [(v * t * t) / (2 * r), (v * t) / r];
  if (t > dur - r) { const e = dur - t; return [1 - (v * e * e) / (2 * r), (v * e) / r]; }
  return [v * (t - r / 2), v];
}

/** Keeps a figure's walk loop in step with the ground it covers, `speed` metres per second (+ ahead). */
function pace(f: Figure, speed: number): void {
  const name = f.motion.gait, g = name && f.gait[name];
  if (g) f.motion.pace(speed / (g.speed * f.model.scale.x));
}

/** How a figure's walk (or run) loop covers the ground, if the model has one. */
function gaitIn(model: THREE.Object3D, gltf: GLTF, name: "walk" | "run"): Gait | null {
  const clip = gltf.animations.find((c) => c.name === name);
  return clip ? gaitOf(model, clip) : null;
}

function resetFigure(f: Figure, offset: number): void {
  f.fallT = -1; f.tilt.rotation.set(0, 0, 0); f.tilt.position.set(0, 0, 0);
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
    // falls on its side where it stands: from this low camera a body lying sideways reads best
    if (f.fallT === 0) f.collapse.begin(f.group, f.fallSide);
    f.fallT = Math.min(f.fallT + dt / FALL_TIME, 1);
    f.collapse.apply(f.fallT * FALL_TIME);
    // the shadow spreads under the body as it comes down beside the feet
    const e = smooth(Math.min(1, f.fallT * 1.4));
    f.blob.scale.set(1 + 0.8 * e, 1, 1.15); f.blob.position.x = f.fallSide * 0.3 * e;
  }
}

/** The sync connector sits on the nape, so it shows on our Persona and hides behind the enemy's head. */
function placePort(f: Figure): void {
  f.bones.head.getWorldPosition(tmpA); f.bones.top.getWorldPosition(tmpB);
  const r = tmpA.distanceTo(tmpB) * PORT.depth, yaw = f.group.rotation.y;
  tmpA.lerp(tmpB, PORT.up);
  f.port.position.set(tmpA.x - Math.sin(yaw) * r, tmpA.y, tmpA.z - Math.cos(yaw) * r);
  f.port.visible = f.fallT < 0 && f.group.visible;
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
