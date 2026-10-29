// Random choices that do not repeat: the same gunshot twice in a row sounds
// like a machine, and the same song twice in a row sounds like a bug.

import { callRust } from "../../runtime/src/index.ts";
export type Random = () => number;

/** Index in [0, n) that differs from `last` whenever n > 1. */
export function pickOther(n: number, last: number | undefined, random: Random = Math.random): number {
  return callRust("audio.pickOther", [n, last, n > 1 ? random() : 0]);
}

/** Remembers the last variant of every sound. */
export class VariantPicker {
  private last = new Map<string, number>();
  private random: Random;

  constructor(random: Random = Math.random) {
    this.random = random;
  }

  pick<T>(key: string, items: readonly T[]): T {
    const i = pickOther(items.length, this.last.get(key), this.random);
    this.last.set(key, i);
    return items[i]!;
  }
}

/** A playlist that picks at random and never plays the same track twice in a row. */
export class Shuffle<T> {
  private current: T | undefined;
  readonly items: readonly T[];
  private random: Random;

  constructor(items: readonly T[], random: Random = Math.random) {
    if (items.length === 0) throw new Error("Empty playlist");
    this.items = items;
    this.random = random;
  }

  get now(): T | undefined {
    return this.current;
  }

  next(): T {
    const last = this.current === undefined ? undefined : this.items.indexOf(this.current);
    this.current = this.items[pickOther(this.items.length, last, this.random)]!;
    return this.current;
  }

  /** Jump to a chosen item; the random picks carry on from it. */
  set(item: T): void {
    if (!this.items.includes(item)) throw new Error("Not in the playlist");
    this.current = item;
  }
}

/** Uniform in [-spread, spread]. */
export const jitter = (spread: number | undefined, random: Random = Math.random) =>
  callRust<number>("audio.jitter", [spread, spread ? random() : 0]);
