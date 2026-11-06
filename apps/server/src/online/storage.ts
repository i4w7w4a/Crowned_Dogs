import { createHash, randomBytes, randomUUID } from "node:crypto";
import { readFile } from "node:fs/promises";
import { callRust } from "../../../../packages/runtime/src/index.ts";
import type { Database } from "./database.ts";
import type { TelegramIdentity } from "./telegram.ts";

export const SESSION_MS = 8 * 60 * 60 * 1000;
export const maxLevel = callRust<{ SYNC: { maxLevel: number } }>("core.constants", []).SYNC.maxLevel;

export class TagExhausted extends Error {
  constructor() { super("Tag capacity exhausted"); }
}
export interface OnlineAccount {
  id: string;
  tag: string;
  telegram: TelegramIdentity;
  profile: Record<string, any>;
  appearance: Record<string, any>;
  loadout: Record<string, any>;
  version: number;
}
function account(row: Record<string, any>): OnlineAccount {
  return { id: row.id, tag: row.dog_tag,
    telegram: { id: String(row.telegram_id), firstName: row.first_name, lastName: row.last_name, username: row.username },
    profile: row.profile, appearance: row.appearance, loadout: row.loadout, version: row.version };
}
const tokenHash = (token: string) => createHash("sha256").update(token).digest("hex");

/** Explicit versioned migrations; run by the migration command, never silently at startup. */
export async function migrate(db: Database): Promise<void> {
  const sql = await readFile(new URL("./migrations/001_accounts.sql", import.meta.url), "utf8");
  await db.transaction(async tx => {
    await tx.query("CREATE TABLE IF NOT EXISTS online_schema_migrations (version integer PRIMARY KEY, applied_at timestamptz NOT NULL DEFAULT now())");
    await tx.query("LOCK TABLE online_schema_migrations IN EXCLUSIVE MODE");
    const applied = await tx.query("SELECT version FROM online_schema_migrations WHERE version = $1", [1]);
    if (!applied.rows.length) {
      await tx.query(sql);
      await tx.query("INSERT INTO online_schema_migrations (version) VALUES ($1)", [1]);
    }
  });
}

export class OnlineStore {
  readonly db: Database;
  constructor(db: Database) { this.db = db; }

  async login(identity: TelegramIdentity, now: number): Promise<{ account: OnlineAccount; token: string }> {
    return this.db.transaction(async tx => {
      const lock = (await tx.query("SELECT last_tag FROM online_tag_counters WHERE id = 1 FOR UPDATE")).rows[0];
      if (!lock) throw new Error("Allocator unavailable");
      let row = (await tx.query("SELECT * FROM online_operators WHERE telegram_id = $1", [identity.id])).rows[0];
      const at = new Date(now);
      if (!row) {
        // Rust owns month numbering, capacity and the permanent tag format.
        const prefix = callRust<string>("core.nextTag", [now, []]).slice(0, 6);
        const taken = (await tx.query("SELECT dog_tag FROM online_operators WHERE dog_tag LIKE $1", [prefix + "%"]))
          .rows.map(r => r.dog_tag as string);
        if (lock.last_tag) taken.push(lock.last_tag);
        let tag: string;
        try { tag = callRust<string>("core.nextTag", [now, taken]); }
        catch { throw new TagExhausted(); }
        const profile = callRust("core.syncFresh", []);
        const appearance = callRust<Record<string, unknown>>("client.look.normalize", [{}]);
        appearance.tag = tag;
        const loadout = callRust("core.starterLoadout", ["pistol"]);
        row = (await tx.query(`INSERT INTO online_operators
          (id, telegram_id, first_name, last_name, username, dog_tag, profile, appearance, loadout, created_at, updated_at)
          VALUES ($1,$2,$3,$4,$5,$6,$7::jsonb,$8::jsonb,$9::jsonb,$10,$10) RETURNING *`,
        [randomUUID(), identity.id, identity.firstName, identity.lastName, identity.username, tag,
          JSON.stringify(profile), JSON.stringify(appearance), JSON.stringify(loadout), at])).rows[0]!;
        await tx.query("UPDATE online_tag_counters SET last_tag = $1 WHERE id = 1", [tag]);
      } else {
        row = (await tx.query(`UPDATE online_operators SET first_name=$2, last_name=$3, username=$4,
          updated_at=$5 WHERE id=$1 RETURNING *`, [row.id, identity.firstName, identity.lastName, identity.username, at])).rows[0]!;
      }
      const token = randomBytes(32).toString("base64url");
      await tx.query("INSERT INTO online_sessions (token_hash, operator_id, created_at, expires_at) VALUES ($1,$2,$3,$4)",
        [tokenHash(token), row.id, at, new Date(now + SESSION_MS)]);
      return { account: account(row), token };
    });
  }

  async authenticate(token: string, now: number): Promise<OnlineAccount | null> {
    if (!/^[A-Za-z0-9_-]{43}$/.test(token)) return null;
    const row = (await this.db.query(`SELECT o.* FROM online_operators o JOIN online_sessions s ON s.operator_id=o.id
      WHERE s.token_hash=$1 AND s.revoked_at IS NULL AND s.expires_at>$2`, [tokenHash(token), new Date(now)])).rows[0];
    return row ? account(row) : null;
  }

  async logout(token: string, now: number): Promise<void> {
    await this.db.query("UPDATE online_sessions SET revoked_at=$2 WHERE token_hash=$1 AND revoked_at IS NULL", [tokenHash(token), new Date(now)]);
  }

  async healthy(): Promise<boolean> {
    // Check the migrated schema as well as connectivity; an empty database is not ready.
    const result = await this.db.query("SELECT version FROM online_schema_migrations WHERE version = $1", [1]);
    return result.rows.length === 1;
  }
}
