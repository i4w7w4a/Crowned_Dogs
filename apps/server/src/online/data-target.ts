export interface DataTarget {
  host: string; port: number; database: string; user: string; password: string;
  options: "-c search_path=public";
}

/** Administrative tools run on the database host or through an explicit local tunnel.
 * Never let pg/libpq URL options or inherited defaults silently redirect them. */
export function dataTarget(connectionString: string, expectedDatabase: string): DataTarget {
  try {
    const url = new URL(connectionString);
    const port = Number(url.port || "5432");
    if (!/^[a-z][a-z0-9_]{0,62}$/.test(expectedDatabase)
      || !["postgres:", "postgresql:"].includes(url.protocol)
      || !["127.0.0.1", "localhost", "[::1]"].includes(url.hostname)
      || url.search || url.hash || url.pathname !== "/" + expectedDatabase
      || !url.username || !url.password || !Number.isInteger(port) || port < 1 || port > 65535) throw new Error();
    return { host: url.hostname.replace(/^\[(.+)\]$/, "$1"), port,
      database: expectedDatabase, user: decodeURIComponent(url.username), password: decodeURIComponent(url.password),
      // pg treats an empty options string as absent and falls back to PGOPTIONS.
      // Administrative imports always target the migrated public registry.
      options: "-c search_path=public" };
  } catch { throw new Error("Invalid explicit local database target"); }
}
