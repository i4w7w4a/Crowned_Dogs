// Salt dust hanging in the hall: specks drifting slowly through the air, bright where
// the shafts of light fall and when you look towards the opening, soft and large when
// they float right in front of the lens. Drawn with the enemy, so his body hides the
// specks behind him.

import * as THREE from "three";
import { callRust, motionCompute } from "../../../../packages/runtime/src/index.ts";

/** The air the dust floats in, metres: in front of the camera (z forward is negative), up to the roof trusses. */
const BOX = { min: new THREE.Vector3(-4, 0.1, -16), size: new THREE.Vector3(8, 4.6, 15) };

const { vertex: vertexShader, fragment: fragmentShader } = callRust<{vertex:string; fragment:string}>("motion.dustShaders", []);

export class Dust {
  readonly points: THREE.Points;
  private material: THREE.ShaderMaterial;
  private capacity = 0;

  constructor() {
    this.material = new THREE.ShaderMaterial({
      vertexShader, fragmentShader,
      uniforms: {
        time: { value: 0 }, pxPerMetre: { value: 800 }, boxMin: { value: BOX.min }, boxSize: { value: BOX.size },
        sunDir: { value: new THREE.Vector3(0, 0.5, -1).normalize() }, tRays: { value: null }, hasRays: { value: 0 },
        focusInv: { value: 0 }, dofPx: { value: 0 },
        color: { value: new THREE.Color(0xe9eef2).multiplyScalar(0.55) },
      },
      transparent: true, depthWrite: false, fog: false,
      blending: THREE.CustomBlending, blendSrc: THREE.OneFactor, blendDst: THREE.OneFactor, blendSrcAlpha: THREE.ZeroFactor, blendDstAlpha: THREE.OneFactor,
    });
    this.points = new THREE.Points(new THREE.BufferGeometry(), this.material);
    this.points.frustumCulled = false;
    this.points.visible = false;
  }

  /** How many specks float; 0 hides the dust. */
  setCount(n: number): void {
    this.points.visible = n > 0;
    if (n > this.capacity) {
      const packed = motionCompute("dust", new Float64Array([n, crypto.getRandomValues(new Uint32Array(1))[0] || 1]));
      const pos = new Float32Array(packed.slice(0, n * 3)), seed = new Float32Array(packed.slice(n * 3));
      this.points.geometry.dispose();
      const g = new THREE.BufferGeometry();
      g.setAttribute("position", new THREE.BufferAttribute(pos, 3));
      g.setAttribute("seed", new THREE.BufferAttribute(seed, 4));
      this.points.geometry = g;
      this.capacity = n;
    }
    this.points.geometry.setDrawRange(0, n);
  }

  /**
   * Each frame: the time, how many pixels a metre is at one metre away, where the light comes from, the shafts it
   * lies in, and the focus: 1 / the enemy's distance and the blur of the painting in pixels per 1/m behind him.
   */
  update(time: number, pxPerMetre: number, sunDir: THREE.Vector3, rays: THREE.Texture | null, focusInv: number, dofPx: number): void {
    const u = this.material.uniforms;
    u.time!.value = time;
    u.pxPerMetre!.value = pxPerMetre;
    (u.sunDir!.value as THREE.Vector3).copy(sunDir);
    u.tRays!.value = rays;
    u.hasRays!.value = rays ? 1 : 0;
    u.focusInv!.value = focusInv;
    u.dofPx!.value = dofPx;
  }
}
