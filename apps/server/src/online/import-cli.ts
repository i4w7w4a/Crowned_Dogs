import { open } from "node:fs/promises";
import { isAbsolute } from "node:path";
import { parseArgs } from "node:util";
import { Pool } from "pg";
import { PgDatabase } from "./database.ts";
import { dataTarget } from "./data-target.ts";
import { importOperators, ImportRejected } from "./import-operators.ts";

async function main() {
  let values;
  let target;
  try {
    ({ values } = parseArgs({ options: { file: { type: "string" }, "database-env": { type: "string" },
      "expect-database": { type: "string" }, apply: { type: "boolean", default: false } }, allowPositionals: false }));
    if (!values.file || !isAbsolute(values.file) || !values["database-env"] || !values["expect-database"]
      || !/^[A-Z][A-Z0-9_]*$/.test(values["database-env"])) throw new Error();
    target = dataTarget(process.env[values["database-env"]] ?? "", values["expect-database"]);
  } catch { throw new ImportRejected({ invalid_options: 1 }); }
  let input: unknown;
  try {
    const file = await open(values.file, "r");
    try {
      const info = await file.stat();
      if (!info.isFile() || info.size > 10 * 1024 * 1024) throw new Error();
      input = JSON.parse(await file.readFile("utf8"));
    } finally { await file.close(); }
  } catch { throw new ImportRejected({ invalid_file: 1 }); }
  const pool = new Pool({ ...target, max: 1, connectionTimeoutMillis: 5000, statement_timeout: 30_000 });
  pool.on("error", () => { /* no connection details in reports */ });
  try { console.log(JSON.stringify({ ok: true, ...await importOperators(new PgDatabase(pool), input, { apply: values.apply }) })); }
  finally { await pool.end(); }
}

main().catch(error => {
  console.error(JSON.stringify({ ok: false, categories: error instanceof ImportRejected ? error.categories : { database_failure: 1 } }));
  process.exitCode = 1;
});
