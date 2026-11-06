export interface DataTarget { host: string; port: number; database: string; user: string; password: string }

/** Administrative tools run on the database host or through an explicit local tunnel.
 * Never let pg/libpq URL options or inherited defaults silently redirect them. */
export function dataTarget(connectionString: string, expectedDatabase: string): DataTarget {
  try {
    const url = new URL(connectionString);
    if (!/^[a-z][a-z0-9_]{0,62}$/.test(expectedDatabase)
      || !["postgres:", "postgresql:"].includes(url.protocol)
      || !["127.0.0.1", "localhost", "[::1]"].includes(url.hostname)
      || url.search || url.hash || url.pathname !== "/" + expectedDatabase
      || !url.username || !url.password) throw new Error();
    return { host: url.hostname.replace(/^\[(.+)\]$/, "$1"), port: Number(url.port || "5432"),
      database: expectedDatabase, user: decodeURIComponent(url.username), password: decodeURIComponent(url.password) };
  } catch { throw new Error("Invalid explicit local database target"); }
}
