export function required(env: NodeJS.ProcessEnv, name: string): string {
  const value = env[name]?.trim();
  if (!value) throw new Error(`${name} required`);
  return value;
}

export function onlineConfig(env: NodeJS.ProcessEnv) {
  const port = env.ONLINE_PORT ?? "3100";
  if (!/^\d+$/.test(port) || Number(port) < 1 || Number(port) > 65535) throw new Error("Invalid online port");
  return { databaseUrl: required(env, "ONLINE_DATABASE_URL"), botToken: required(env, "BOT_TOKEN"),
    origin: required(env, "ONLINE_ORIGIN"), host: env.ONLINE_HOST?.trim() || "127.0.0.1", port: Number(port),
    basePath: env.API_BASE_PATH ?? "/api/online", production: env.NODE_ENV === "production" };
}
