import * as THREE from "three";

const plain = THREE.Object3D.prototype.updateMatrixWorld;

/**
 * Three's updateMatrixWorld, but a hidden branch is left as it is: nothing in it is drawn, and a
 * fighter carries many such (the guns not in hand, the hats and hair not worn, the reflection, the
 * hazard pool), so a frame would otherwise recompute hundreds of matrices nobody reads. The branch
 * is marked stale, so it catches up the frame it is shown again. Reading `getWorldPosition` of a
 * hidden object stays right: that walks up and refreshes on its own.
 */
function lean(this: THREE.Object3D, force?: boolean): void {
  if (this.matrixAutoUpdate) this.updateMatrix();
  if (this.matrixWorldNeedsUpdate || force) {
    if (this.matrixWorldAutoUpdate === true) {
      if (this.parent === null) this.matrixWorld.copy(this.matrix);
      else this.matrixWorld.multiplyMatrices(this.parent.matrixWorld, this.matrix);
    }
    this.matrixWorldNeedsUpdate = false;
    force = true;
  }
  const children = this.children;
  for (let i = 0, l = children.length; i < l; i++) {
    const child = children[i]!;
    if (child.visible === false) child.matrixWorldNeedsUpdate = true;
    else child.updateMatrixWorld(force);
  }
}

/** Gives every plain object under `root` the lean update; cameras and other own versions are kept. */
export function leanMatrices(root: THREE.Object3D): void {
  root.traverse((o) => {
    if (o.updateMatrixWorld === plain) o.updateMatrixWorld = lean;
  });
}
