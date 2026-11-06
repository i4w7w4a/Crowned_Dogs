import { onlineConfig } from "./config.ts";
import { connectDatabase } from "./database.ts";
import { createOnlineServer } from "./http.ts";
import { OnlineStore } from "./storage.ts";

async function main() {
  const config = onlineConfig(process.env);
  const db = connectDatabase(config.databaseUrl);
  db.pool.on("error", () => console.error("Online database connection lost"));
  try {
    const store = new OnlineStore(db);
    const server = createOnlineServer({ store, ...config });
    if (!(await store.healthy())) throw new Error("Database not migrated");
    await new Promise<void>((resolve, reject) => {
      server.once("error", reject);
      server.listen(config.port, config.host, () => { server.removeListener("error", reject); resolve(); });
    });
    console.log("Online API ready");
    let stopping = false;
    const stop = () => {
      if (stopping) return;
      stopping = true;
      const timeout = setTimeout(() => server.closeAllConnections(), 10_000);
      timeout.unref();
      server.close(() => {
        clearTimeout(timeout);
        void db.pool.end().catch(() => { process.exitCode = 1; });
      });
    };
    process.once("SIGINT", stop);
    process.once("SIGTERM", stop);
  } catch (error) {
    await db.pool.end();
    throw error;
  }
}
main().catch(() => { console.error("Online API startup failed; check configuration, database and migrations"); process.exitCode = 1; });
