// Operators who signed the contract, and the dog tag each one got.
// Kept in a small JSON file so tags survive a restart. Knows nothing about Telegram
// beyond storing the account id next to the tag.

import { randomInt } from "node:crypto";
import { mkdirSync, readFileSync, renameSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";
import { RustOperatorRegistry } from "../../../packages/runtime/src/index.ts";

export interface Operator {
  /** K-XXX-XXX, given once and never changed. */
  tag: string;
  telegramId: string;
  name: string;
  /** ms since 1970. */
  signedAt: number;
}

export interface OperatorRegistryOptions {
  /** Where to keep the list. Without it everything stays in memory. */
  file?: string;
  /** Picks a tag number in [0, TAG_COUNT). Random by default. */
  pick?: () => number;
  now?: () => number;
}

/** Stable public types around the Rust registry and its injected Node filesystem. */
export class OperatorRegistry {
  private runtime: RustOperatorRegistry;
  constructor(opts: OperatorRegistryOptions = {}) {
    this.runtime = new RustOperatorRegistry(opts, { randomInt, mkdirSync, readFileSync, renameSync, writeFileSync, dirname });
  }
  byTelegram(telegramId: string): Operator | undefined { return this.runtime.byTelegram(telegramId); }
  sign(telegramId: string, name: string): Operator { return this.runtime.sign(telegramId, name); }
}
