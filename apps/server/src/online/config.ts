import { proxyPeers } from "./peer-ip.ts";
export function required(env: NodeJS.ProcessEnv, name: string): string {
    const value = env[name]?.trim();
    if (!value)
        throw new Error(`${name} required`);
    return value;
}
export function onlineConfig(env: NodeJS.ProcessEnv) {
    const port = env.ONLINE_PORT ?? "3100";
    if (!/^\d+$/.test(port) || Number(port) < 1 || Number(port) > 65535)
        throw new Error("Invalid online port");
    return { databaseUrl: required(env, "ONLINE_DATABASE_URL"), botToken: required(env, "BOT_TOKEN"),
        origin: required(env, "ONLINE_ORIGIN"), host: env.ONLINE_HOST?.trim() || "127.0.0.1", port: Number(port),
        basePath: env.API_BASE_PATH ?? "/api/online", production: env.NODE_ENV === "production" };
}
export function onlineRuntimeConfig(env: NodeJS.ProcessEnv) {
    const base = onlineConfig(env);
    const botUsername = required(env, "ONLINE_BOT_USERNAME");
    const chatId = required(env, "ONLINE_CHAT_ID");
    const thread = required(env, "ONLINE_THREAD_ID");
    const webAppUrl = required(env, "ONLINE_WEBAPP_URL");
    if (!/^[A-Za-z0-9_]{5,32}$/.test(botUsername))
        throw new Error("Invalid bot username");
    if (!/^-[1-9]\d*$/.test(chatId) || !Number.isSafeInteger(Number(chatId)))
        throw new Error("Invalid game chat");
    if (!/^[1-9]\d*$/.test(thread) || !Number.isSafeInteger(Number(thread)) || Number(thread) > 2147483647)
        throw new Error("Invalid game topic");
    const web = new URL(webAppUrl);
    if (web.origin !== base.origin || web.username || web.password || web.search || web.hash)
        throw new Error("Invalid Web App URL");
    const trustedProxyIps = [...proxyPeers((env.ONLINE_TRUSTED_PROXY_IPS ?? "").split(",").map(ip => ip.trim()).filter(Boolean))];
    return { ...base, botUsername, chatId, threadId: Number(thread), webAppUrl, trustedProxyIps };
}
