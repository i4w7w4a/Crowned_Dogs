import { createHash } from "node:crypto";
import { spawn } from "node:child_process";
import { createReadStream } from "node:fs";
import { mkdir, open, readFile, writeFile } from "node:fs/promises";
import { isAbsolute, join } from "node:path";
import { Pool } from "pg";
import type { Sql } from "./database.ts";
import { dataTarget } from "./data-target.ts";
import type { DataTarget } from "./data-target.ts";

function checked(target: DataTarget): DataTarget {
  const host = target.host.includes(":") ? `[${target.host}]` : target.host;
  return dataTarget(`postgresql://${encodeURIComponent(target.user)}:${encodeURIComponent(target.password)}@${host}:${target.port}/${target.database}`, target.database);
}
function poolFor(target: DataTarget) {
  const pool = new Pool({ ...checked(target), max: 1, connectionTimeoutMillis: 5000 });
  pool.on("error", () => { /* credentials and SQL payloads never go to operator reports */ });
  return pool;
}
async function requirePg18(sql: Sql) {
  const version = Number((await sql.query("SHOW server_version_num")).rows[0]?.server_version_num);
  if (version < 180000 || version >= 190000) throw new Error("PostgreSQL 18 required");
}
async function hashFile(path: string) {
  const hash = createHash("sha256");
  for await (const chunk of createReadStream(path)) hash.update(chunk);
  return hash.digest("hex");
}
async function pgTool(name: string, args: string[], target: DataTarget, pgBin?: string, input?: number, output?: number) {
  const env = Object.fromEntries(Object.entries(process.env).filter(([key]) => !key.toUpperCase().startsWith("PG")));
  Object.assign(env, { PGHOST: target.host, PGPORT: String(target.port), PGDATABASE: target.database,
    PGUSER: target.user, PGPASSWORD: target.password, PGCONNECT_TIMEOUT: "5", PGAPPNAME: "crowned-dogs-data" });
  const binary = name + (process.platform === "win32" ? ".exe" : "");
  if (pgBin && !isAbsolute(pgBin)) throw new Error("Absolute PG_BIN required");
  await new Promise<void>((resolve, reject) => {
    const child = spawn(pgBin ? join(pgBin, binary) : binary, args,
      { env, shell: false, windowsHide: true, stdio: [input ?? "ignore", output ?? "ignore", "pipe"] });
    // PostgreSQL errors may include paths, credentials or private row values.
    child.stderr?.resume();
    const timeout = setTimeout(() => { child.kill(); reject(new Error(`${name} timeout`)); }, 600_000);
    child.on("error", () => { clearTimeout(timeout); reject(new Error(`${name} unavailable`)); });
    child.on("close", code => { clearTimeout(timeout); code === 0 ? resolve() : reject(new Error(`${name} failed`)); });
  });
}
const report = (snapshot: DataSnapshot) => ({ tables: snapshot.tables.length,
  rows: snapshot.tables.reduce((sum, table) => sum + table.rows, 0), versions: snapshot.versions });

export async function createBackup(options: { source: DataTarget; directory: string; codeVersion: string; pgBin?: string }) {
  const source = checked(options.source);
  if (!isAbsolute(options.directory) || !/^[a-f0-9]{40}$/.test(options.codeVersion)) throw new Error("Explicit backup path and code commit required");
  // Refuse to overwrite any prior archive. A failure leaves an incomplete directory for inspection.
  await mkdir(options.directory, { mode: 0o700 });
  const pool = poolFor(source);
  const archive = join(options.directory, "archive.dump");
  let client;
  try {
    client = await pool.connect();
    await requirePg18(client);
    await client.query("BEGIN ISOLATION LEVEL REPEATABLE READ READ ONLY");
    const snapshotId = (await client.query("SELECT pg_export_snapshot() AS id")).rows[0]!.id;
    const data = await snapshotData(client);
    const output = await open(archive, "wx", 0o600);
    try {
      await pgTool("pg_dump", ["--format=custom", "--no-owner", "--no-privileges", "--no-password",
        `--dbname=${source.database}`, `--snapshot=${snapshotId}`], source, options.pgBin, undefined, output.fd);
      await output.sync();
    } finally { await output.close(); }
    await client.query("COMMIT");
    const manifest = { format: 1, codeVersion: options.codeVersion, createdAt: new Date().toISOString(),
      archiveSha256: await hashFile(archive), data };
    await writeFile(join(options.directory, "manifest.json"), JSON.stringify(manifest, null, 2) + "\n", { flag: "wx", mode: 0o600 });
    return report(data);
  } catch (error) { if (client) await client.query("ROLLBACK").catch(() => {}); throw error; }
  finally { client?.release(); await pool.end(); }
}

export async function verifyRestore(options: { target: DataTarget; directory: string; pgBin?: string }) {
  const target = checked(options.target);
  // This is deliberately a drill-only operation. No flag can turn it into production restore.
  if (!/^crowned_dogs_test_restore_[a-z0-9_]+$/.test(target.database)) throw new Error("Invalid restore target");
  if (!isAbsolute(options.directory)) throw new Error("Absolute backup directory required");
  const manifest = JSON.parse(await readFile(join(options.directory, "manifest.json"), "utf8"));
  const archive = join(options.directory, "archive.dump");
  if (manifest.format !== 1 || !/^[a-f0-9]{40}$/.test(manifest.codeVersion) || !manifest.data?.tables?.length
    || await hashFile(archive) !== manifest.archiveSha256) throw new Error("Invalid backup integrity");
  const pool = poolFor(target);
  try {
    await requirePg18(pool);
    const occupied = (await pool.query(`SELECT
      EXISTS(SELECT 1 FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace
        WHERE n.nspname NOT LIKE 'pg_%' AND n.nspname <> 'information_schema')
      OR EXISTS(SELECT 1 FROM pg_proc p JOIN pg_namespace n ON n.oid=p.pronamespace
        WHERE n.nspname NOT LIKE 'pg_%' AND n.nspname <> 'information_schema')
      OR EXISTS(SELECT 1 FROM pg_namespace WHERE nspname NOT LIKE 'pg_%' AND nspname NOT IN ('public','information_schema'))
      OR EXISTS(SELECT 1 FROM pg_extension WHERE extname <> 'plpgsql') AS occupied`)).rows[0]?.occupied;
    if (occupied) throw new Error("Restore target must be empty");
    const input = await open(archive, "r");
    try {
      await pgTool("pg_restore", ["--single-transaction", "--exit-on-error", "--no-owner", "--no-privileges",
        "--no-password", `--dbname=${target.database}`], target, options.pgBin, input.fd);
    } finally { await input.close(); }
    const client = await pool.connect();
    try {
      await client.query("BEGIN ISOLATION LEVEL REPEATABLE READ READ ONLY");
      const actual = await snapshotData(client);
      await client.query("COMMIT");
      if (JSON.stringify(actual) !== JSON.stringify(manifest.data)) throw new Error("Restored data mismatch");
      return report(actual);
    } finally { client.release(); }
  } finally { await pool.end(); }
}

const identifier = (name: string) => '"' + name.replaceAll('"', '""') + '"';
export interface DataSnapshot {
  versions: number[];
  tables: { schema: string; name: string; columns: unknown[]; rows: number; sha256: string }[];
}

/** Called within the same repeatable-read transaction exported to pg_dump. No row values leave it. */
export async function snapshotData(sql: Sql): Promise<DataSnapshot> {
  await sql.query("SET LOCAL TIME ZONE 'UTC'");
  const versions = (await sql.query("SELECT version FROM public.online_schema_migrations ORDER BY version")).rows.map(r => Number(r.version));
  if (!versions.length) throw new Error("Missing migrations");
  const tables: DataSnapshot["tables"] = [];
  const names = (await sql.query(`SELECT n.nspname AS schema, c.relname AS name FROM pg_class c
    JOIN pg_namespace n ON n.oid=c.relnamespace
    WHERE c.relkind IN ('r','p') AND n.nspname NOT LIKE 'pg_%' AND n.nspname <> 'information_schema'
    ORDER BY n.nspname COLLATE "C", c.relname COLLATE "C"`)).rows;
  for (const { schema, name } of names) {
    const columns = (await sql.query(`SELECT column_name, data_type, udt_schema, udt_name, is_nullable,
      column_default, is_identity, identity_generation, is_generated, generation_expression
      FROM information_schema.columns WHERE table_schema=$1 AND table_name=$2 ORDER BY ordinal_position`, [schema, name])).rows;
    const hash = createHash("sha256");
    let rows = 0;
    await sql.query(`DECLARE backup_rows NO SCROLL CURSOR FOR SELECT to_jsonb(t)::text AS value
      FROM ONLY ${identifier(schema)}.${identifier(name)} t ORDER BY to_jsonb(t)::text COLLATE "C"`);
    try {
      while (true) {
        const batch = (await sql.query("FETCH FORWARD 500 FROM backup_rows")).rows;
        if (!batch.length) break;
        for (const row of batch) { hash.update(row.value).update("\n"); rows++; }
      }
    } finally { await sql.query("CLOSE backup_rows"); }
    tables.push({ schema, name, columns, rows, sha256: hash.digest("hex") });
  }
  return { versions, tables };
}
