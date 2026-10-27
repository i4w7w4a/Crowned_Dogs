// The defence panel: our Persona projected as a hologram, a live copy of our own
// 3D figure (same pose, same flinches), cut at fixed heights into the four zones
// the enemy can hit. Guarded zones light up and are hatched; a hit flashes red,
// a hit on the Zont white. Leader lines join each zone name to its band.

import type { AttackReport, Zone } from "@crowned-dogs/core";
import * as THREE from "three";
import type { GLTF } from "three/addons/loaders/GLTFLoader.js";
import * as SkeletonUtils from "three/addons/utils/SkeletonUtils.js";

/** The box in CSS px; 72 px per metre; tilted ~9° down. */
export const HOLO = { W: 66, H: 154, feet: 140, ppm: 72, tilt: 0.16 };
/** Metres: legs | groin | body | head. */
const ZCUT = [0.8, 1.06, 1.5] as const;
const ZI: Record<Zone, number> = { head: 0, body: 1, groin: 2, legs: 3 };
/** Where each leader line touches the body, metres up... */
export const ZMID: Record<Zone, number> = { head: 1.64, body: 1.3, groin: 0.94, legs: 0.5 };
/** ...and how far left of the centre line, so the line ends on the body. */
const ZEND: Record<Zone, number> = { head: -0.05, body: -0.09, groin: -0.07, legs: -0.1 };
const HEIGHT = 1.8;

/** A height on the body, in px of the box. */
export const holoY = (y: number) => HOLO.feet - y * Math.cos(HOLO.tilt) * HOLO.ppm;
/** The zone under a point of the box. */
export function zoneAtY(py: number): Zone {
  const y = (HOLO.feet - py) / (Math.cos(HOLO.tilt) * HOLO.ppm);
  return y >= ZCUT[2] ? "head" : y >= ZCUT[1] ? "body" : y >= ZCUT[0] ? "groin" : "legs";
}

const HOLO_FRAG = `
uniform vec4 uSel, uHit, uBlk, uHov, uPulse;
uniform vec3 uCut;
uniform float uSweep, uDpr, uDim, uJoint;
varying vec3 vHN;
varying float vWY;
`;
const HOLO_MAIN = `
  float aa = max(fwidth(vWY), 1e-4);
  float sH = smoothstep(uCut.z - aa, uCut.z + aa, vWY), sC = smoothstep(uCut.y - aa, uCut.y + aa, vWY), sG = smoothstep(uCut.x - aa, uCut.x + aa, vWY);
  vec4 zw = vec4(sH, sC - sH, sG - sC, 1.0 - sG);
  float sel = dot(zw, uSel), hit = dot(zw, uHit), blk = dot(zw, uBlk), hov = dot(zw, uHov), pul = dot(zw, uPulse);
  float rim = pow(1.0 - abs(normalize(vHN).z), 2.0);
  // survey lines round the body every 5 cm, and a sharp line where one zone meets the next
  float cy = vWY / 0.05, fc = fwidth(cy);
  float contour = 1.0 - smoothstep(0.35 * fc, 1.25 * fc, abs(fract(cy + 0.5) - 0.5));
  vec3 dc = abs(vec3(vWY) - uCut);
  float cut = 1.0 - smoothstep(0.6 * aa, 1.8 * aa, min(dc.x, min(dc.y, dc.z)));
  float hatch = 1.0 - smoothstep(0.3, 0.46, fract((gl_FragCoord.x - gl_FragCoord.y) / (3.4 * uDpr)));
  float scan = 0.78 + 0.22 * step(0.5, fract(gl_FragCoord.y / (2.0 * uDpr)));
  float sweep = exp(-pow((vWY - uSweep) * 9.0, 2.0));

  float I = mix(0.035 + 0.5 * rim + 0.09 * contour, 0.13 + 0.95 * rim + 0.24 * hatch + 0.2 * contour, sel);
  I += cut * (0.16 + 0.5 * sel);
  I += hov * (0.06 + 0.3 * rim);
  I += sweep * (0.06 + 0.32 * rim);
  I += pul * (0.2 + 0.7 * rim);
  I *= scan * (1.0 + 0.3 * uJoint) * (1.0 - 0.5 * uDim);
  vec3 col = mix(vec3(0.56, 0.74, 0.78), mix(vec3(0.45, 0.83, 0.89), vec3(0.86, 0.99, 1.0), 0.5 * rim), sel);
  col = mix(col, vec3(0.92, 0.36, 0.26), clamp(hit, 0.0, 1.0));
  col = mix(col, vec3(0.82, 1.0, 1.0), clamp(blk, 0.0, 1.0) * 0.7);
  I += hit * (0.25 + 0.8 * rim) + blk * (0.3 + 0.9 * rim);
  float a = clamp(I, 0.0, 1.0);
  gl_FragColor = vec4(col * a, a);
`;
/** Additive light that composites right on a transparent canvas: colour stays premultiplied, so it never goes out of gamut. */
const ADD = {
  transparent: true, depthWrite: false, depthTest: false, blending: THREE.CustomBlending,
  blendSrc: THREE.OneFactor, blendDst: THREE.OneFactor, blendSrcAlpha: THREE.OneFactor, blendDstAlpha: THREE.OneFactor,
} as const;

type Uniforms = Record<string, THREE.IUniform>;
type Four = [number, number, number, number];
const four = (): Four => [0, 0, 0, 0];

function bodyMaterial(U: Uniforms, joint: number): THREE.MeshBasicMaterial {
  const m = new THREE.MeshBasicMaterial({ ...ADD, fog: false, toneMapped: false });
  m.onBeforeCompile = (sh) => {
    Object.assign(sh.uniforms, U, { uJoint: { value: joint } });
    sh.vertexShader = sh.vertexShader
      .replace("#include <common>", "#include <common>\nuniform float uGlitch, uGy;\nvarying vec3 vHN;\nvarying float vWY;")
      .replace("#include <project_vertex>", `#include <project_vertex>
        vWY = (modelMatrix * vec4(transformed, 1.0)).y;
        #ifdef USE_SKINNING
          vHN = transformedNormal;
        #else
          vHN = normalMatrix * normal;
        #endif
        gl_Position.x += (1.0 - smoothstep(0.025, 0.06, abs(vWY - uGy))) * uGlitch * 0.07 * gl_Position.w;`);
    sh.fragmentShader = sh.fragmentShader
      .replace("#include <common>", "#include <common>\n" + HOLO_FRAG)
      .replace("#include <opaque_fragment>", HOLO_MAIN);
  };
  return m;
}

/** Motes drifting up through the figure, the only trace of the projector. */
function motes(U: Uniforms): THREE.Points {
  const N = 26, pos = new Float32Array(N * 3), seed = new Float32Array(N);
  for (let i = 0; i < N; i++) {
    const a = Math.random() * Math.PI * 2, r = Math.sqrt(Math.random()) * 0.36;
    pos.set([Math.cos(a) * r, Math.random() * 1.9, Math.sin(a) * r], i * 3);
    seed[i] = Math.random();
  }
  const g = new THREE.BufferGeometry();
  g.setAttribute("position", new THREE.BufferAttribute(pos, 3));
  g.setAttribute("seed", new THREE.BufferAttribute(seed, 1));
  return new THREE.Points(g, new THREE.ShaderMaterial({
    ...ADD, uniforms: U,
    vertexShader: `uniform float uTime, uDpr; attribute float seed; varying float vA;
      void main() {
        vec3 p = position; p.y = mod(p.y + uTime * (0.05 + 0.1 * seed), 1.9);
        vA = sin(3.14159 * p.y / 1.9) * (0.25 + 0.45 * seed);
        gl_PointSize = (1.0 + 1.2 * seed) * uDpr;
        gl_Position = projectionMatrix * modelViewMatrix * vec4(p, 1.0);
      }`,
    fragmentShader: `uniform float uDim; varying float vA;
      void main() { float d = length(gl_PointCoord - 0.5); float a = vA * (1.0 - smoothstep(0.2, 0.5, d)) * (1.0 - 0.5 * uDim); gl_FragColor = vec4(vec3(0.6, 0.9, 0.95) * a, a); }`,
  }));
}

export class Hologram {
  /** Zone under the finger or pointer, or -1. */
  hover = -1;
  private want = four();
  private sel = four();
  private pulse = four();
  private hit = four();
  private blk = four();
  private hov = four();
  private lock = false;
  private lost = false;
  private kick = 0;
  private box: HTMLElement;
  private canvas: HTMLCanvasElement;
  private reduceMotion: boolean;

  constructor(box: HTMLElement, canvas: HTMLCanvasElement, reduceMotion: boolean) {
    this.box = box;
    this.canvas = canvas;
    this.reduceMotion = reduceMotion;
  }

  setHover(z: Zone | null): void {
    this.hover = z === null ? -1 : ZI[z];
  }

  /** Picks follow the order being put together. */
  set(defend: readonly Zone[], lock: boolean, lost: boolean): void {
    for (const z of Object.keys(ZI) as Zone[]) {
      const i = ZI[z], on = defend.includes(z) ? 1 : 0;
      if (on && !this.want[i]) this.pulse[i] = 1;
      this.want[i] = on;
    }
    this.lock = lock;
    this.lost = lost;
  }

  /** The enemy's attack lands on (or misses) our projection. */
  flash(a: AttackReport): void {
    if (!a.hit) return;
    if (a.weapon === "grenade") {
      this.hit = [0.8, 0.8, 0.8, 0.8];
      this.kick = 1;
      return;
    }
    const i = ZI[a.zone ?? "body"];
    if (a.blocked) this.blk[i] = 1;
    else { this.hit[i] = 1; this.kick = 1; }
  }

  /** Starts projecting. `source` is our figure in the arena: the projection copies its bones every frame. */
  start(gltf: GLTF, source: THREE.Object3D): void {
    let r: THREE.WebGLRenderer;
    try {
      r = new THREE.WebGLRenderer({ canvas: this.canvas, alpha: true, antialias: true });
    } catch {
      return; // the outline drawing stays
    }
    const dpr = Math.min(window.devicePixelRatio || 1, 3);
    r.setClearColor(0x000000, 0);
    r.outputColorSpace = THREE.LinearSRGBColorSpace;
    r.setPixelRatio(dpr);
    r.setSize(HOLO.W, HOLO.H, false);

    // orthographic and tilted down a little; the feet land on HOLO.feet
    const t = HOLO.tilt, c = 0.9, top = HOLO.feet / HOLO.ppm - c * Math.cos(t), half = HOLO.W / 2 / HOLO.ppm;
    const cam = new THREE.OrthographicCamera(-half, half, top, top - HOLO.H / HOLO.ppm, 0.1, 20);
    cam.position.set(0, c + Math.sin(t) * 6, Math.cos(t) * 6);
    cam.lookAt(0, c, 0);

    const U = {
      uSel: { value: new THREE.Vector4() }, uHit: { value: new THREE.Vector4() }, uBlk: { value: new THREE.Vector4() },
      uHov: { value: new THREE.Vector4() }, uPulse: { value: new THREE.Vector4() }, uCut: { value: new THREE.Vector3(...ZCUT) },
      uTime: { value: 0 }, uSweep: { value: -9 }, uDpr: { value: dpr }, uDim: { value: 0 }, uGlitch: { value: 0 }, uGy: { value: 1 },
    };
    const model = SkeletonUtils.clone(gltf.scene);
    const body = bodyMaterial(U, 0), joints = bodyMaterial(U, 1);
    model.traverse((o) => {
      if (!(o instanceof THREE.Mesh)) return;
      o.material = /joint/i.test((o.material as THREE.Material).name) ? joints : body;
      o.frustumCulled = false;
    });
    model.updateMatrixWorld(true);
    const bb = new THREE.Box3().setFromObject(model, true), s = HEIGHT / (bb.max.y - bb.min.y);
    model.scale.multiplyScalar(s);
    model.position.y = -bb.min.y * s;
    const turn = new THREE.Group();
    turn.add(model);
    const scene = new THREE.Scene();
    scene.add(turn, motes(U));

    // the projection repeats our Persona's pose every frame: breathing, steps, flinches
    const a: THREE.Object3D[] = [], b: THREE.Object3D[] = [];
    source.traverse((o) => { if ((o as THREE.Bone).isBone) a.push(o); });
    model.traverse((o) => { if ((o as THREE.Bone).isBone) b.push(o); });
    const pairs = a.map((bone, i) => [bone, b[i]!] as const);

    let last = performance.now(), dim = 0, glitch = 0, nextGlitch = 4;
    const ease = (arr: Four, to: Four | null, k: number) => { for (let i = 0; i < 4; i++) arr[i] = arr[i]! + ((to ? to[i]! : 0) - arr[i]!) * k; };
    r.setAnimationLoop((now) => {
      const dt = Math.min((now - last) / 1000, 0.05), time = now / 1000;
      last = now;
      for (const [src, dst] of pairs) { dst.position.copy(src.position); dst.quaternion.copy(src.quaternion); }
      turn.rotation.y = this.reduceMotion ? 0.3 : Math.sin(time * 0.4) * 0.42;

      ease(this.sel, this.want, 1 - Math.exp(-dt * 14));
      ease(this.hov, [0, 1, 2, 3].map((i) => (i === this.hover ? 1 : 0)) as Four, 1 - Math.exp(-dt * 16));
      ease(this.pulse, null, 1 - Math.exp(-dt * 4.5));
      ease(this.hit, null, 1 - Math.exp(-dt * 2.6));
      ease(this.blk, null, 1 - Math.exp(-dt * 2.6));
      dim += ((this.lost ? 1.4 : this.lock ? 0.55 : 0) - dim) * (1 - Math.exp(-dt * 6));

      // the image breaks up for a moment now and then, and every time we are hit
      if (this.kick) { glitch = 1; U.uGy.value = 0.3 + Math.random() * 1.4; this.kick = 0; }
      else if (!this.reduceMotion && time > nextGlitch) {
        glitch = this.lost ? 1 : 0.45;
        U.uGy.value = Math.random() * 1.8;
        nextGlitch = time + (this.lost ? 0.5 : 5 + Math.random() * 7);
      }
      glitch *= Math.exp(-dt * 22);

      U.uSel.value.fromArray(this.sel); U.uHov.value.fromArray(this.hov); U.uPulse.value.fromArray(this.pulse);
      U.uHit.value.fromArray(this.hit); U.uBlk.value.fromArray(this.blk);
      U.uTime.value = time; U.uDim.value = dim; U.uGlitch.value = glitch;
      U.uSweep.value = this.reduceMotion ? -9 : (time % 3.6) / 3.6 * 2.8 - 0.4;
      r.render(scene, cam);
    });
    this.box.classList.add("live");
  }
}

/** Leader lines from each zone name to its band on the body, bent like callouts in a manual. */
export function drawLeads(guard: HTMLElement, svg: SVGSVGElement, defend: readonly Zone[]): void {
  const W = guard.clientWidth, cx = W - HOLO.W / 2;
  let out = "";
  for (const b of guard.querySelectorAll<HTMLElement>(".dz")) {
    const z = b.dataset.z as Zone, zl = b.querySelector<HTMLElement>(".zl")!;
    const y0 = b.offsetTop + b.offsetHeight / 2, x0 = zl.offsetLeft + zl.offsetWidth + 6;
    const x2 = cx + ZEND[z] * HOLO.ppm, y2 = holoY(ZMID[z]);
    const x1 = Math.max(x0 + 4, x2 - Math.abs(y2 - y0));
    const on = defend.includes(z) ? " on" : "";
    out += `<g class="lead${on}" data-z="${z}"><path d="M${x0.toFixed(1)} ${y0.toFixed(1)}H${x1.toFixed(1)}L${x2.toFixed(1)} ${y2.toFixed(1)}"/></g>`;
  }
  svg.setAttribute("viewBox", `0 0 ${W} ${HOLO.H}`);
  svg.innerHTML = out;
}
