import * as THREE from "three";
import type { Loadout } from "@crowned-dogs/core";
import { callRust, graphicsTexture } from "../rust/engine.ts";
import { silhouetteDeck, turntableMaterial } from "./turntable-material.ts";
import type { QualityId } from "../arena/quality.ts";

/** Authoring convention shared with the user-edited Blender scene; metres. */
export const PLATFORM_TOP = 0.145;
type Vec3 = [number, number, number];
interface GeometrySpec { kind: string; args?: number[]; profile?: [number, number][]; points?: Vec3[]; segments?: number; radius?: number; height?: number; radialSegments?: number }
interface Part { geometry: GeometrySpec; material: string; position: Vec3; rotation: Vec3; name?: string; opacity?: number; castShadow: boolean; receiveShadow: boolean }
interface StageSpec { top: number; fixed: Part[]; rotating: Part[]; feet: Part[] }
interface SlingSpec { position: number[]; uv: number[]; index: number[]; clips: { name: string; position: Vec3 }[]; clipPath: Vec3[] }

export interface FittingStage {
  fixed: THREE.Group;
  rotating: THREE.Group;
  feet: THREE.Group;
  setQuality: (quality: QualityId) => void;
}

/** Rust constructs the assembly. This adapter translates its primitive descriptions to Three. */
export function makeFittingStage(): FittingStage {
  const spec = callRust<StageSpec>("graphics.stage", {});
  const fixed = new THREE.Group(), rotating = new THREE.Group(), feet = new THREE.Group();
  fixed.name = "Fitting_Bay_Fixed_Floor_And_Bearing";
  rotating.name = "Fitting_Turntable_Persona_And_Sling";
  feet.name = "Foot_Contact_Shadows";
  const soft = new THREE.DataTexture(graphicsTexture("contact", 128), 128, 128, THREE.RGBAFormat);
  soft.magFilter = soft.minFilter = THREE.LinearFilter; soft.needsUpdate = true;
  const deck = turntableMaterial();
  const steel = deck.clone(); steel.color.setHex(0x92989b); steel.metalness = .5;
  const edge = deck.clone(); edge.color.setHex(0xd2d5d5); edge.roughness = .69; edge.metalness = .75;
  const materials: Record<string, THREE.Material> = {
    deck, steel, edge,
    rubber: new THREE.MeshStandardMaterial({ color: 0x111111, roughness: .97, metalness: 0 }),
    paint: new THREE.MeshStandardMaterial({ color: 0xa5aaa7, roughness: .8, metalness: .08 }),
    floor: new THREE.ShadowMaterial({ color: 0x101010, opacity: .32, depthWrite: false }),
  };
  const qualitySetters: ((quality: QualityId) => void)[] = [];
  const build = (part: Part): THREE.Object3D => {
    const d = part.geometry, a = d.args ?? [];
    let result: THREE.Object3D;
    if (d.kind === "silhouetteDeck") {
      const relief = silhouetteDeck(deck, d.radius!, d.height!);
      qualitySetters.push(relief.setQuality); result = relief.group;
    } else {
      let geometry: THREE.BufferGeometry;
      switch (d.kind) {
        case "plane": geometry = new THREE.PlaneGeometry(a[0], a[1]); break;
        case "box": geometry = new THREE.BoxGeometry(a[0], a[1], a[2]); break;
        case "cylinder": geometry = new THREE.CylinderGeometry(a[0], a[1], a[2], a[3]); break;
        case "torus": geometry = new THREE.TorusGeometry(a[0], a[1], a[2], a[3]); break;
        case "lathe": geometry = new THREE.LatheGeometry(d.profile!.map((p) => new THREE.Vector2(...p)), d.segments); break;
        case "tube": geometry = new THREE.TubeGeometry(new THREE.CatmullRomCurve3(d.points!.map((p) => new THREE.Vector3(...p))), d.segments, d.radius, d.radialSegments, false); break;
        default: throw new Error(`Unknown Rust stage primitive: ${d.kind}`);
      }
      const material = part.material === "contact" ? new THREE.MeshBasicMaterial({ color: 0x030405, map: soft, transparent: true, opacity: part.opacity, depthWrite: false, toneMapped: false }) : materials[part.material]!;
      result = new THREE.Mesh(geometry, material);
      result.castShadow = part.castShadow; result.receiveShadow = part.receiveShadow;
    }
    result.position.fromArray(part.position); result.rotation.set(...part.rotation);
    if (part.name) result.name = part.name;
    return result;
  };
  fixed.add(...spec.fixed.map(build)); rotating.add(...spec.rotating.map(build)); feet.add(...spec.feet.map(build)); rotating.add(feet);
  return { fixed, rotating, feet, setQuality: (quality) => qualitySetters.forEach((set) => set(quality)) };
}

/** Fallback sling geometry is evaluated by Rust; the user's authored rifle sling stays exact. */
export function makeSling(mount: THREE.Object3D, pistol = false): THREE.Mesh {
  mount.updateMatrix();
  const spec = callRust<SlingSpec>("graphics.sling", { matrix: mount.matrix.elements, pistol });
  const geometry = new THREE.BufferGeometry();
  geometry.setAttribute("position", new THREE.Float32BufferAttribute(spec.position, 3));
  geometry.setAttribute("uv", new THREE.Float32BufferAttribute(spec.uv, 2));
  geometry.setIndex(spec.index); geometry.computeVertexNormals();
  const sling = new THREE.Mesh(geometry, new THREE.MeshStandardMaterial({ color: 0x20251f, roughness: .94, metalness: 0, side: THREE.DoubleSide }));
  sling.name = "Two_Point_Woven_Sling"; sling.castShadow = true; sling.receiveShadow = true;
  for (const descriptor of spec.clips) {
    const clip = new THREE.Group(); clip.name = descriptor.name; clip.position.fromArray(descriptor.position); clip.rotation.z = mount.rotation.z;
    const metal = new THREE.MeshStandardMaterial({ color: 0x666b6c, metalness: .8, roughness: .48 });
    const path = new THREE.CatmullRomCurve3(spec.clipPath.map((p) => new THREE.Vector3(...p)));
    const frame = new THREE.Mesh(new THREE.TubeGeometry(path, 24, .0024, 6, false), metal); frame.name = `${descriptor.name}_Frame`;
    const gate = new THREE.Mesh(new THREE.CylinderGeometry(.0019, .0019, .031, 8), metal);
    gate.name = `${descriptor.name}_Gate`; gate.position.set(.009, -.001, 0);
    clip.add(frame, gate); sling.add(clip);
  }
  return sling;
}

/** Three's scene hierarchy is an external rendering API; socket attachment remains a bridge. */
export function prepareVaryag(gun: THREE.Object3D): void {
  for (const child of [...gun.children]) {
    if (child.userData.factory === false) gun.getObjectByName(`socket_${child.userData.slot}`)?.add(child);
  }
}

export function fitVaryag(gun: THREE.Object3D, loadout: Loadout | null): void {
  const nodes: { name: string; factory: boolean; slot?: string; replaces?: string }[] = [];
  gun.traverse((object) => {
    if (typeof object.userData.factory === "boolean") nodes.push({ name: object.name, factory: object.userData.factory, slot: object.userData.slot, replaces: object.userData.replaces });
  });
  const visibility = callRust<{ name: string; visible: boolean }[]>("graphics.weaponVisibility", { nodes, loadout });
  for (const state of visibility) { const object = gun.getObjectByName(state.name); if (object) object.visible = state.visible; }
}
