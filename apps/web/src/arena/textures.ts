// Texture levels of the figures. The models carry 1K textures inside; the low level
// shrinks them to 512 in the browser, high and ultra load 2K and 4K copies of the same
// maps from tex/<model>/ (made by scripts/texture-tiers.py). A model without such
// copies simply keeps its own.

import * as THREE from "three";

interface Manifest { sizes: number[]; materials: Record<string, string> }
type Slot = "map" | "normalMap" | "roughnessMap" | "metalnessMap";
const SLOTS: [Slot, string][] = [["map", "color"], ["normalMap", "normal"], ["roughnessMap", "orm"], ["metalnessMap", "orm"]];

export class TextureTiers {
  private mats: THREE.MeshStandardMaterial[] = [];
  /** The maps each material came with (1K). */
  private own = new Map<THREE.MeshStandardMaterial, Partial<Record<Slot, THREE.Texture>>>();
  private manifest: Promise<Manifest | null>;
  /** Textures made here, freed when the level changes again. */
  private made: THREE.Texture[] = [];
  private size = 1024;
  private ask = 0;

  /** `model` is the model's address, e.g. models/persona-male.json; its copies are listed in tex/persona-male.json. */
  constructor(roots: THREE.Object3D[], model: string) {
    for (const root of roots) root.traverse((o) => {
      const m = (o as THREE.Mesh).material as THREE.MeshStandardMaterial | undefined;
      if (!m?.isMeshStandardMaterial || !m.map || this.own.has(m)) return;
      this.mats.push(m);
      this.own.set(m, Object.fromEntries(SLOTS.map(([s]) => [s, m[s] ?? undefined])));
    });
    const name = /([\w-]+)\.(json|glb)$/.exec(model)?.[1];
    this.manifest = !name ? Promise.resolve(null) : fetch(`tex/${name}.json`)
      .then((r) => (r.ok ? (r.json() as Promise<Manifest>) : null))
      .catch(() => null);
  }

  /** Puts the textures of this size on (falls back to the model's own when there are no copies that size). */
  async set(size: number, anisotropy: number): Promise<void> {
    const ask = ++this.ask;
    const manifest = await this.manifest;
    if (ask !== this.ask) return;
    let next: Map<THREE.MeshStandardMaterial, Partial<Record<Slot, THREE.Texture>>> | null = null;
    if (size < 1024) next = this.shrink(size);
    else if (size > 1024 && manifest?.sizes.includes(size)) next = await this.fetchSize(manifest, size).catch(() => null);
    if (ask !== this.ask) { if (next) for (const set of next.values()) for (const t of Object.values(set)) t?.dispose(); return; }
    const old = this.made;
    this.made = next ? [...new Set([...next.values()].flatMap((set) => Object.values(set)))] as THREE.Texture[] : [];
    for (const m of this.mats) {
      const own = this.own.get(m)!, mine = next?.get(m);
      for (const [slot] of SLOTS) {
        const t = mine?.[slot] ?? own[slot];
        if (!t) continue;
        t.anisotropy = anisotropy;
        if (m[slot] !== t) { m[slot] = t; m.needsUpdate = true; }
        // the model's own copy leaves the graphics memory while a bigger or smaller one is on
        if (mine?.[slot]) own[slot]!.dispose();
      }
    }
    for (const t of old) t.dispose();
    this.size = next ? size : 1024;
  }

  get current(): number { return this.size; }

  /** The model's maps drawn smaller, for weak phones. */
  private shrink(size: number): Map<THREE.MeshStandardMaterial, Partial<Record<Slot, THREE.Texture>>> {
    const out = new Map<THREE.MeshStandardMaterial, Partial<Record<Slot, THREE.Texture>>>();
    const done = new Map<THREE.Texture, THREE.Texture>();
    for (const m of this.mats) {
      const set: Partial<Record<Slot, THREE.Texture>> = {};
      for (const [slot, t] of Object.entries(this.own.get(m)!) as [Slot, THREE.Texture | undefined][]) {
        if (!t) continue;
        let small = done.get(t);
        if (!small) {
          const img = t.image as CanvasImageSource & { width: number; height: number };
          const c = document.createElement("canvas");
          c.width = Math.min(size, img.width); c.height = Math.min(size, img.height);
          c.getContext("2d")!.drawImage(img, 0, 0, c.width, c.height);
          small = copyWith(t, c);
          done.set(t, small);
        }
        set[slot] = small;
      }
      out.set(m, set);
    }
    return out;
  }

  /** Loads every material's copies of one size; all or nothing, so the figures never wear a mix. */
  private async fetchSize(manifest: Manifest, size: number): Promise<Map<THREE.MeshStandardMaterial, Partial<Record<Slot, THREE.Texture>>>> {
    const loader = new THREE.ImageLoader();
    // the enemy and his reflection share one copy of each picture
    const files = new Map<string, Promise<THREE.Texture>>();
    const out = new Map<THREE.MeshStandardMaterial, Partial<Record<Slot, THREE.Texture>>>();
    await Promise.all(this.mats.map(async (m) => {
      const base = manifest.materials[m.name];
      if (!base) return;
      const own = this.own.get(m)!, set: Partial<Record<Slot, THREE.Texture>> = {};
      for (const [slot, kind] of SLOTS) {
        const t = own[slot];
        if (!t) continue;
        const url = `${base}.${kind}.${size}.webp`;
        if (!files.has(url)) files.set(url, loader.loadAsync(url).then((img) => copyWith(t, img)));
        set[slot] = await files.get(url)!;
      }
      out.set(m, set);
    }));
    return out;
  }
}

/** The same texture settings (colour space, wrap, no flip, as the model has them) with another picture. */
function copyWith(t: THREE.Texture, image: TexImageSource | HTMLCanvasElement): THREE.Texture {
  const c = t.clone();
  c.source = new THREE.TextureSource(image);
  c.needsUpdate = true;
  return c;
}
