import { isAbsolute } from "node:path";
import { parseArgs } from "node:util";
import { dataTarget } from "./data-target.ts";
import { createBackup, verifyRestore } from "./data-backup.ts";

async function main() {
  let values, positionals, target;
  try {
    ({ values, positionals } = parseArgs({ options: { directory: { type: "string" },
      "database-env": { type: "string" }, "expect-database": { type: "string" }, "code-version": { type: "string" } }, allowPositionals: true }));
    if (positionals.length !== 1 || !["backup", "verify-restore"].includes(positionals[0]!)
      || !values.directory || !isAbsolute(values.directory) || !values["database-env"] || !values["expect-database"]
      || !/^[A-Z][A-Z0-9_]*$/.test(values["database-env"])
      || (process.env.PG_BIN && !isAbsolute(process.env.PG_BIN))) throw new Error();
    if (positionals[0] === "backup" && !/^[a-f0-9]{40}$/.test(values["code-version"] ?? "")) throw new Error();
    if (positionals[0] === "verify-restore" && values["code-version"]) throw new Error();
    target = dataTarget(process.env[values["database-env"]] ?? "", values["expect-database"]);
  } catch { console.error(JSON.stringify({ ok: false, category: "invalid_options" })); process.exitCode = 1; return; }
  const operation = positionals[0]!;
  try {
    const result = operation === "backup"
      ? await createBackup({ source: target, directory: values.directory, codeVersion: values["code-version"]!, pgBin: process.env.PG_BIN })
      : await verifyRestore({ target, directory: values.directory, pgBin: process.env.PG_BIN });
    console.log(JSON.stringify({ ok: true, operation, ...result }));
  } catch {
    console.error(JSON.stringify({ ok: false, category: operation === "backup" ? "backup_failed" : "verification_failed" }));
    process.exitCode = 1;
  }
}
await main();
