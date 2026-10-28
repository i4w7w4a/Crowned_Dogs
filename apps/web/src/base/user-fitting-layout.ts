import * as THREE from "three";
import layout from "./user-fitting-layout.json" with { type: "json" };

/** Saved Blender placement, converted to the equipment group's metre/Y-up space.
 * The strap is the evaluated Bezier surface, preserving its handles, tilt and profile. */
export function authoredWeaponFit(mount: THREE.Object3D): THREE.Group {
  new THREE.Matrix4().fromArray(layout.weaponMatrix).decompose(mount.position, mount.quaternion, mount.scale);
  const group = new THREE.Group(); group.name = "Authored_Sling_And_Clips";
  for (const part of layout.accessories) {
    const geometry = new THREE.BufferGeometry();
    geometry.setAttribute("position", new THREE.Float32BufferAttribute(part.positions, 3));
    geometry.setAttribute("normal", new THREE.Float32BufferAttribute(part.normals, 3));
    geometry.setIndex(part.indices);
    const material = new THREE.MeshStandardMaterial({
      color: new THREE.Color().setRGB(part.color[0]!, part.color[1]!, part.color[2]!),
      roughness: part.roughness, metalness: part.metalness, side: THREE.DoubleSide,
    });
    const mesh = new THREE.Mesh(geometry, material); mesh.name = part.name;
    mesh.castShadow = true; mesh.receiveShadow = true; group.add(mesh);
  }
  return group;
}
