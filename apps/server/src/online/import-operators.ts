import { randomUUID } from "node:crypto";
import { callRust } from "../../../../packages/runtime/src/index.ts";
import type { Database } from "./database.ts";

export class ImportRejected extends Error {
  readonly categories: Record<string, number>;
  constructor(categories: Record<string, number>) { super("Import rejected"); this.categories = categories; }
}
interface LegacyOperator { telegramId: string; tag: string; name: string; signedAt: number }

function validate(input: unknown): LegacyOperator[] {
  if (!Array.isArray(input)) throw new ImportRejected({ invalid_document: 1 });
  const categories: Record<string, number> = {};
  const error = (key: string) => { categories[key] = (categories[key] ?? 0) + 1; };
  const ids = new Set<string>(), tags = new Set<string>();
  for (const row of input) {
    if (!row || typeof row !== "object" || Array.isArray(row)) { error("invalid_record"); continue; }
    if (Object.keys(row).some(k => !["telegramId", "tag", "name", "signedAt"].includes(k))) error("unknown_fields");
    if (typeof row.telegramId !== "string" || !/^[1-9][0-9]{0,15}$/.test(row.telegramId)
      || BigInt(row.telegramId) > 9007199254740991n) error("invalid_telegram_id");
    else if (ids.has(row.telegramId)) error("duplicate_telegram_id");
    else ids.add(row.telegramId);
    if (typeof row.tag !== "string" || !/^K-(?!000)[0-9]{3}-(?!000)[0-9]{3}$/.test(row.tag)) error("invalid_tag");
    else if (tags.has(row.tag)) error("duplicate_tag");
    else tags.add(row.tag);
    if (typeof row.name !== "string" || !row.name.trim() || row.name.length > 256 || /[\x00-\x1f\x7f]/.test(row.name)) error("invalid_name");
    if (!Number.isSafeInteger(row.signedAt) || row.signedAt < 0 || row.signedAt > 8640000000000000) error("invalid_signed_at");
  }
  if (Object.keys(categories).length) throw new ImportRejected(categories);
  return input as LegacyOperator[];
}

/** Trusted server registry only. The browser has no route to this administrative operation. */
export async function importOperators(db: Database, input: unknown, options: { apply?: boolean } = {}) {
  const operators = validate(input);
  return db.transaction(async tx => {
    // Same row and lock order as OnlineStore.login: imports and registrations cannot allocate together.
    const counter = (await tx.query("SELECT last_tag FROM online_tag_counters WHERE id=1 FOR UPDATE")).rows[0];
    if (!counter) throw new ImportRejected({ allocator_unavailable: 1 });
    const pending: LegacyOperator[] = [];
    let conflicts = 0;
    for (const op of operators) {
      const matches = (await tx.query("SELECT telegram_id, dog_tag FROM online_operators WHERE telegram_id=$1 OR dog_tag=$2", [op.telegramId, op.tag])).rows;
      if (!matches.length) pending.push(op);
      else if (matches.length !== 1 || String(matches[0]!.telegram_id) !== op.telegramId || matches[0]!.dog_tag !== op.tag) conflicts++;
    }
    if (conflicts) throw new ImportRejected({ identity_conflict: conflicts });
    if (options.apply) {
      for (const op of pending) {
        const appearance = callRust<Record<string, unknown>>("client.look.normalize", [{}]);
        appearance.tag = op.tag;
        await tx.query(`INSERT INTO online_operators
          (id, telegram_id, first_name, dog_tag, profile, appearance, loadout, created_at, updated_at)
          VALUES ($1,$2,$3,$4,$5::jsonb,$6::jsonb,$7::jsonb,$8,$8)`,
        [randomUUID(), op.telegramId, op.name, op.tag, JSON.stringify(callRust("core.syncFresh", [])),
          JSON.stringify(appearance), JSON.stringify(callRust("core.starterLoadout", ["pistol"])), new Date(op.signedAt)]);
      }
      // Fixed-width tags sort by call-up and order. Keep the existing high-water mark;
      // occupied older call-ups remain in online_operators, which Rust allocation also reads.
      await tx.query(`UPDATE online_tag_counters SET last_tag=GREATEST(last_tag,
        (SELECT max(dog_tag COLLATE "C") FROM online_operators)) WHERE id=1`);
    }
    return { mode: options.apply ? "apply" : "dry-run", total: operators.length,
      created: pending.length, existing: operators.length - pending.length };
  });
}
