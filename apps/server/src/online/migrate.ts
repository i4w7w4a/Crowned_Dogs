import { required } from "./config.ts";
import { connectDatabase } from "./database.ts";
import { migrate } from "./storage.ts";

async function main() {
  const db = connectDatabase(required(process.env, "ONLINE_DATABASE_URL"));
  db.pool.on("error", () => console.error("Migration database connection lost"));
  try { await migrate(db); console.log("Online migrations applied"); }
  finally { await db.pool.end(); }
}
main().catch(() => { console.error("Online migration failed; check isolated database configuration"); process.exitCode = 1; });
