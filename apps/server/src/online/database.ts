import { Pool } from "pg";

export interface Sql {
  query(text: string, values?: unknown[]): Promise<{ rows: Record<string, any>[] }>;
}
export interface Database extends Sql {
  transaction<T>(run: (sql: Sql) => Promise<T>): Promise<T>;
}

/** Every transaction owns one connection, including rollback and release. */
export class PgDatabase implements Database {
  readonly pool: Pool;
  constructor(pool: Pool) { this.pool = pool; }
  query(text: string, values?: unknown[]) { return this.pool.query(text, values); }
  async transaction<T>(run: (sql: Sql) => Promise<T>): Promise<T> {
    const client = await this.pool.connect();
    let broken: Error | undefined;
    try {
      await client.query("BEGIN");
      const result = await run(client);
      await client.query("COMMIT");
      return result;
    } catch (error) {
      try { await client.query("ROLLBACK"); }
      catch { broken = new Error("Transaction rollback failed"); }
      throw error;
    } finally { client.release(broken); }
  }
}

export function connectDatabase(connectionString: string): PgDatabase {
  return new PgDatabase(new Pool({ connectionString, max: 5, connectionTimeoutMillis: 5000,
    idleTimeoutMillis: 30_000, statement_timeout: 5000, query_timeout: 6000 }));
}
