/** Typed host handles; Rust remembers variants, enforces playlists and consumes randomness. */
import { RustShuffle, RustVariantPicker, audio_pick_other, audio_jitter } from "../../runtime/src/index.ts";
export type Random = () => number;
export const pickOther = (n: number, last: number | undefined, random: Random = Math.random): number =>
  audio_pick_other(n, last, random);
export class VariantPicker {
  private runtime: RustVariantPicker;
  private random: Random;
  constructor(random: Random = Math.random) {
    this.random = random;
    this.runtime = new RustVariantPicker(this.random);
  }
  pick<T>(key: string, items: readonly T[]): T { return this.runtime.pick(key, items); }
}
export class Shuffle<T> {
  private runtime: RustShuffle;
  private random: Random;
  readonly items: readonly T[];
  constructor(items: readonly T[], random: Random = Math.random) {
    this.items = items; this.random = random;
    this.runtime = new RustShuffle(this.items, this.random);
  }
  get now(): T | undefined { return this.runtime.now; }
  next(): T { return this.runtime.next(); }
  set(item: T): void { this.runtime.set(item); }
}
export const jitter = (spread: number | undefined, random: Random = Math.random): number =>
  audio_jitter(spread, random);
