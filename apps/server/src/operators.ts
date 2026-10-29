// Operators who signed the contract, and the dog tag each one got.
// Kept in a small JSON file so tags survive a restart. Knows nothing about Telegram
// beyond storing the account id next to the tag.

import { randomInt } from "node:crypto";
import { mkdirSync, readFileSync, renameSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";
import { TAG_COUNT } from "@crowned-dogs/core";
import { callRust } from "../../../packages/runtime/src/index.ts";

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

export class OperatorRegistry {
  private byTg = new Map<string, Operator>();
  private opts: OperatorRegistryOptions;

  constructor(opts: OperatorRegistryOptions = {}) {
    this.opts = opts;
    if (opts.file) {
      let saved: Operator[] = [];
      try {
        saved = JSON.parse(readFileSync(opts.file, "utf8")) as Operator[];
      } catch (err) {
        if ((err as NodeJS.ErrnoException).code !== "ENOENT") throw err;
      }
      for (const op of saved) this.add(op);
    }
  }

  byTelegram(telegramId: string): Operator | undefined {
    return this.byTg.get(telegramId);
  }

  /** Signs the contract and hands out a tag. Signing again returns the same operator. */
  sign(telegramId: string, name: string): Operator {
    const known = this.byTg.get(telegramId);
    if (known) return known;
    const pick = this.opts.pick ?? (() => randomInt(TAG_COUNT));
    const picks: number[] = [];
    const now = (this.opts.now ?? Date.now)();
    let result: { needsPick?: boolean; operator: Operator; operators: Operator[] };
    for (;;) {
      result = callRust("server.operators.sign", [[...this.byTg.values()], telegramId, name, now, picks]);
      if (!result.needsPick) break;
      picks.push(pick());
    }
    const op = result.operator;
    this.add(op);
    this.save();
    return op;
  }

  private add(op: Operator): void {
    this.byTg.set(op.telegramId, op);
  }

  /** Writes a temp file and renames it, so a crash never leaves half a list. */
  private save(): void {
    const file = this.opts.file;
    if (!file) return;
    mkdirSync(dirname(file), { recursive: true });
    writeFileSync(`${file}.tmp`, JSON.stringify([...this.byTg.values()], null, 2));
    renameSync(`${file}.tmp`, file);
  }
}
