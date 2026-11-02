// What a figure wears over its body: the armour kit fitted on the Personas, as meshes of the model named
// Suit, Vest and Helmet. The body under a garment is not drawn (it would poke through in the poses): its
// vertices carry how much each garment covers them (_suit, _vest), and a triangle with a covered corner
// leaves the index. Hair has a flatter copy for under the helmet (FemHairCards_Helmet for FemHairCards).

import * as THREE from "three";

/** The slots of the kit that are worn (the catalogue's suit, body armour and head slots). */
export interface Wear { suit: boolean; body: boolean; head: boolean }
type Slot = keyof Wear;

/** Kit A: field suit with boots and gloves, «Плита-2», «Каска К-6». */
export const KIT_A: Wear = { suit: true, body: true, head: true };

const GARMENT: Record<string, Slot> = { Suit: "suit", Vest: "body", Helmet: "head" };
const COVER: [string, Slot][] = [["_suit", "suit"], ["_vest", "body"]];
const UNDER_HELMET = "_Helmet";

/** Shows the garments worn and hides what they cover. Models cloned from one file share their geometry,
 *  so dressing a second copy the same way changes nothing. */
export function dress(model: THREE.Object3D, wear: Wear): void {
  model.traverse((o) => {
    if (!(o instanceof THREE.Mesh)) return;
    const slot = GARMENT[o.name];
    if (slot) { o.visible = wear[slot]; return; }
    if (o.name.endsWith(UNDER_HELMET)) { o.visible = wear.head; return; }
    if (model.getObjectByName(o.name + UNDER_HELMET)) { o.visible = !wear.head; return; }
    const g = o.geometry as THREE.BufferGeometry;
    const masks = COVER.filter(([, s]) => wear[s]).map(([a]) => g.getAttribute(a)).filter((a) => a);
    if (!masks.length) return;
    const full = (g.userData.fullIndex ??= g.index) as THREE.BufferAttribute | null;
    const n = full ? full.count : g.getAttribute("position").count, kept: number[] = [];
    const covered = (v: number) => masks.some((m) => m.getX(v) >= 0.5);
    for (let i = 0; i < n; i += 3) {
      const [a, b, c] = full ? [full.getX(i), full.getX(i + 1), full.getX(i + 2)] : [i, i + 1, i + 2];
      if (!covered(a) && !covered(b) && !covered(c)) kept.push(a, b, c);
    }
    g.setIndex(new THREE.BufferAttribute(new Uint32Array(kept), 1));
  });
}
