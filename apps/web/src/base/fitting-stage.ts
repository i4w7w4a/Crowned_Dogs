import * as THREE from "three";
import type { Loadout } from "@crowned-dogs/core";

/** Metres; the complete rotating assembly sits on a fixed bearing and a real floor. */
export const PLATFORM_TOP = 0.145;

function disc(radius: number, height: number, material: THREE.Material, segments = 80): THREE.Mesh {
  const mesh = new THREE.Mesh(new THREE.CylinderGeometry(radius, radius, height, segments), material);
  mesh.castShadow = true; mesh.receiveShadow = true;
  return mesh;
}

function softMap(): THREE.CanvasTexture {
  const canvas = document.createElement("canvas"); canvas.width = canvas.height = 128;
  const context = canvas.getContext("2d")!;
  const gradient = context.createRadialGradient(64, 64, 8, 64, 64, 64);
  gradient.addColorStop(0, "rgba(255,255,255,1)");
  gradient.addColorStop(.35, "rgba(255,255,255,.65)");
  gradient.addColorStop(1, "rgba(255,255,255,0)");
  context.fillStyle = gradient; context.fillRect(0, 0, 128, 128);
  return new THREE.CanvasTexture(canvas);
}

function contact(map: THREE.Texture, width: number, depth: number, opacity: number): THREE.Mesh {
  const mesh = new THREE.Mesh(new THREE.PlaneGeometry(width, depth), new THREE.MeshBasicMaterial({
    color: 0x030405, map, transparent: true, opacity, depthWrite: false, toneMapped: false,
  }));
  mesh.rotation.x = -Math.PI / 2;
  return mesh;
}

/** Subtle deterministic grit in the steel, filtered by the normal texture mip chain. */
function steelGrain(): THREE.DataTexture {
  const size = 128, pixels = new Uint8Array(size * size * 4);
  for (let y = 0; y < size; y++) for (let x = 0; x < size; x++) {
    const grit = ((x * 17 + y * 29) ^ (x * y * 13)) & 31;
    const tread = Math.abs(((x + y + (Math.floor(y / 16) % 2) * 8) % 16) - 8) < 1.3 ? 22 : 0;
    const value = 185 + grit + tread, offset = (y * size + x) * 4;
    pixels[offset] = pixels[offset + 1] = pixels[offset + 2] = value; pixels[offset + 3] = 255;
  }
  const texture = new THREE.DataTexture(pixels, size, size);
  texture.wrapS = texture.wrapT = THREE.RepeatWrapping; texture.repeat.set(6, 6);
  texture.magFilter = THREE.LinearFilter; texture.minFilter = THREE.LinearMipmapLinearFilter;
  texture.generateMipmaps = true; texture.needsUpdate = true;
  return texture;
}

export interface FittingStage {
  fixed: THREE.Group;
  rotating: THREE.Group;
  feet: THREE.Group;
}

/** Buildable industrial parts, no baked figure or pedestal in the room image. */
export function makeFittingStage(): FittingStage {
  const fixed = new THREE.Group(), rotating = new THREE.Group(), feet = new THREE.Group();
  fixed.name = "Fitting_Bay_Fixed_Floor_And_Bearing";
  rotating.name = "Fitting_Turntable_Persona_And_Sling";
  feet.name = "Foot_Contact_Shadows";

  const soft = softMap();
  // The room already supplies the floor colour; this mesh contributes only real cast shadows.
  const floorMaterial = new THREE.ShadowMaterial({ color: 0x111111, opacity: .42, depthWrite: false });
  const floor = new THREE.Mesh(new THREE.PlaneGeometry(8, 8), floorMaterial);
  floor.rotation.x = -Math.PI / 2; floor.receiveShadow = true; floor.position.y = -.008;
  fixed.add(floor);
  const baseShadow = contact(soft, 2.45, 2.3, .65); baseShadow.position.y = -.004; fixed.add(baseShadow);

  const grain = steelGrain();
  const steel = new THREE.MeshStandardMaterial({ color: 0x242424, roughness: .7, metalness: .82, roughnessMap: grain });
  const edge = new THREE.MeshStandardMaterial({ color: 0x4a4a4a, roughness: .5, metalness: .9, roughnessMap: grain });
  const rubber = new THREE.MeshStandardMaterial({ color: 0x111111, roughness: .97, metalness: 0 });
  const deck = new THREE.MeshStandardMaterial({ color: 0x292929, roughness: .88, metalness: .8, roughnessMap: grain, bumpMap: grain, bumpScale: .003 });
  const base = disc(.79, .078, steel); base.position.y = .039; fixed.add(base);
  const bearing = disc(.715, .032, rubber); bearing.position.y = .086; fixed.add(bearing);
  const bevel = disc(.77, .028, edge); bevel.position.y = .12; rotating.add(bevel);
  const top = disc(.745, .016, deck); top.position.y = PLATFORM_TOP - .008; rotating.add(top);

  // Real circular joints, fixings and radial deck seams provide parallax as the disc turns.
  for (const radius of [.535, .709]) {
    const ring = new THREE.Mesh(new THREE.TorusGeometry(radius, .0028, 5, 96), rubber);
    ring.rotation.x = Math.PI / 2; ring.position.y = PLATFORM_TOP + .001;
    rotating.add(ring);
  }
  for (let i = 0; i < 12; i++) {
    const angle = i / 12 * Math.PI * 2;
    const screw = new THREE.Mesh(new THREE.CylinderGeometry(.008, .008, .003, 6), edge);
    screw.position.set(Math.cos(angle) * .68, PLATFORM_TOP + .001, Math.sin(angle) * .68);
    screw.castShadow = true; rotating.add(screw);
    if (i % 3 !== 0) continue;
    const seam = new THREE.Mesh(new THREE.BoxGeometry(.18, .0015, .003), rubber);
    seam.position.set(Math.cos(angle) * .618, PLATFORM_TOP + .0015, Math.sin(angle) * .618);
    seam.rotation.y = -angle; rotating.add(seam);
  }
  // Two restrained physical index marks, so rotating the deck is visible independently of the body.
  const paint = new THREE.MeshStandardMaterial({ color: 0xa5aaa7, roughness: .8, metalness: .08 });
  for (const x of [-.19, .19]) {
    const mark = new THREE.Mesh(new THREE.BoxGeometry(.055, .001, .012), paint);
    mark.position.set(x, PLATFORM_TOP + .002, .57); rotating.add(mark);
  }
  for (const x of [-.11, .11]) {
    const shadow = contact(soft, .26, .43, .66);
    shadow.position.set(x, PLATFORM_TOP + .003, .045);
    feet.add(shadow);
  }
  rotating.add(feet);
  return { fixed, rotating, feet };
}

/** The source has an A-pose. Lower it a little for a relaxed fitting stance, without aiming clips. */
export function relaxArms(figure: THREE.Object3D): void {
  figure.updateMatrixWorld(true);
  for (const side of ["l", "r"]) {
    const upper = figure.getObjectByName(`upperarm_${side}`), lower = figure.getObjectByName(`lowerarm_${side}`);
    if (!upper || !lower || !upper.parent) continue;
    const shoulder = upper.getWorldPosition(new THREE.Vector3()), elbow = lower.getWorldPosition(new THREE.Vector3());
    const direction = elbow.sub(shoulder).normalize();
    const sign = direction.x < 0 ? -1 : 1;
    const target = new THREE.Vector3(sign * Math.sin(THREE.MathUtils.degToRad(28)), -Math.cos(THREE.MathUtils.degToRad(28)), direction.z * .4).normalize();
    const correction = new THREE.Quaternion().setFromUnitVectors(direction, target);
    const world = upper.getWorldQuaternion(new THREE.Quaternion()).premultiply(correction);
    upper.quaternion.copy(upper.parent.getWorldQuaternion(new THREE.Quaternion()).invert().multiply(world));
    figure.updateMatrixWorld(true);
  }
}

/** A flat woven sling follows the shoulder and the torso, joining both weapon attachment points. */
export function makeSling(mount: THREE.Object3D, pistol = false): THREE.Mesh {
  const at = (point: THREE.Vector3) => point.applyMatrix4(mount.matrix);
  mount.updateMatrix();
  const points = pistol ? [
    at(new THREE.Vector3(-.025, -.03, 0)), new THREE.Vector3(.24, 1.02, .18), new THREE.Vector3(.24, 1.08, -.12),
  ] : [
    at(new THREE.Vector3(-.24, .08, 0)), new THREE.Vector3(-.21, 1.50, .20),
    new THREE.Vector3(-.22, 1.53, -.08), new THREE.Vector3(.13, 1.35, -.205),
    new THREE.Vector3(.27, 1.12, -.10), new THREE.Vector3(.28, 1.06, .19),
    at(new THREE.Vector3(.34, .065, 0)),
  ];
  const curve = new THREE.CatmullRomCurve3(points, false, "centripetal");
  const vertices: number[] = [], uvs: number[] = [], indices: number[] = [];
  const steps = 56, width = pistol ? .018 : .031;
  for (let i = 0; i <= steps; i++) {
    const t = i / steps, point = curve.getPoint(t), tangent = curve.getTangent(t);
    const outward = new THREE.Vector3(point.x, 0, point.z).normalize();
    if (outward.lengthSq() < .1) outward.set(0, 0, 1);
    const across = new THREE.Vector3().crossVectors(tangent, outward).normalize().multiplyScalar(width / 2);
    const a = point.clone().add(across), b = point.clone().sub(across);
    vertices.push(a.x, a.y, a.z, b.x, b.y, b.z); uvs.push(0, t, 1, t);
    if (i < steps) { const k = i * 2; indices.push(k, k + 1, k + 2, k + 1, k + 3, k + 2); }
  }
  const geometry = new THREE.BufferGeometry();
  geometry.setAttribute("position", new THREE.Float32BufferAttribute(vertices, 3));
  geometry.setAttribute("uv", new THREE.Float32BufferAttribute(uvs, 2)); geometry.setIndex(indices); geometry.computeVertexNormals();
  const sling = new THREE.Mesh(geometry, new THREE.MeshStandardMaterial({ color: 0x20251f, roughness: .94, metalness: 0, side: THREE.DoubleSide }));
  sling.name = "Two_Point_Woven_Sling"; sling.castShadow = true; sling.receiveShadow = true;
  return sling;
}

/** Reuse the finished Varyag sockets; its geometry and materials are left unchanged. */
export function prepareVaryag(gun: THREE.Object3D): void {
  for (const child of [...gun.children]) {
    if (child.userData.factory === false) gun.getObjectByName(`socket_${child.userData.slot}`)?.add(child);
  }
}

export function fitVaryag(gun: THREE.Object3D, loadout: Loadout | null): void {
  const names: Record<string, string> = { "optic-sokol": "optic_sokol", "muzzle-tishina": "muzzle_tishina" };
  const replaces = new Set<string>();
  gun.traverse((object) => {
    if (object.userData.factory !== false) return;
    const selected = loadout?.weapon1.mods?.[object.userData.slot as "optic" | "muzzle"];
    object.visible = !!selected && names[selected] === object.name;
    if (object.visible && typeof object.userData.replaces === "string") replaces.add(object.userData.replaces);
  });
  gun.traverse((object) => { if (object.userData.factory === true) object.visible = !replaces.has(object.name); });
}
