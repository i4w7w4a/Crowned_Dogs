import { createHmac } from "node:crypto";
import { mkdtemp, realpath, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve, sep } from "node:path";
import { PGlite } from "@electric-sql/pglite";
import type { PGliteInterface } from "@electric-sql/pglite";
import type { Database, Sql } from "../src/online/database.ts";

export const botToken = "synthetic-test-bot-token";
export const clock = Date.UTC(2026, 10, 6, 12);
// Independent fixture signer; production must only verify.
export function initData(fields: Record<string, string> = {}, token = botToken): string {
  const values = { auth_date: String(clock / 1000), user: JSON.stringify({ id: 123, first_name: "Арт + &" }), ...fields };
  const check = Object.entries(values).sort(([a], [b]) => a < b ? -1 : a > b ? 1 : 0)
    .map(([key, value]) => `${key}=${value}`).join("\n");
  const key = createHmac("sha256", "WebAppData").update(token).digest();
  const hash = createHmac("sha256", key).update(check).digest("hex");
  return new URLSearchParams({ ...values, hash }).toString();
}

export async function embeddedDatabase() {
  const parent = await realpath(tmpdir());
  const path = await mkdtemp(join(parent, "crowned-dogs-online-test-"));
  let engine = new PGlite(path);
  // Like pg, parameterless queries use the simple protocol (migration SQL may contain DDL statements).
  const sql = (connection: Pick<PGliteInterface, "query" | "exec">): Sql => ({
    query: async (text, values) => values ? connection.query<Record<string, any>>(text, values)
      : (await connection.exec(text)).at(-1) as { rows: Record<string, any>[] },
  });
  const database: Database = {
    query: (text, values) => sql(engine).query(text, values),
    transaction: <T>(run: (sql: Sql) => Promise<T>): Promise<T> =>
      engine.transaction(tx => run(sql(tx))),
  };
  return {
    database,
    async reopen() { await engine.close(); engine = new PGlite(path); },
    async close() {
      await engine.close();
      const actual = await realpath(path);
      if (actual !== resolve(path) || !actual.startsWith(parent + sep) || !actual.slice(parent.length + 1).startsWith("crowned-dogs-online-test-")) throw new Error("Unsafe test cleanup");
      await rm(actual, { recursive: true });
    },
  };
}
