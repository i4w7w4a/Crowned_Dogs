import { createServer } from "node:http";
import type { IncomingMessage, ServerResponse } from "node:http";
import { maxLevel, OnlineStore, SESSION_MS, TagExhausted } from "./storage.ts";
import { verifyTelegram } from "./telegram.ts";
import { GameFailure, type OnlineGameService } from "./game-service.ts";
import { BROWSER_LOGIN_MS, type BrowserLoginService } from "./browser-login.ts";
import { normalizeIp, proxyPeers } from "./peer-ip.ts";
export interface OnlineOptions {
    store: OnlineStore;
    game?: OnlineGameService;
    browserLogin?: BrowserLoginService;
    botToken: string;
    origin: string;
    basePath?: string;
    production?: boolean;
    now?: () => number;
    trustedProxyIps?: readonly string[];
    rateLimit?: {
        max: number;
        authMax: number;
        windowMs: number;
        playerMax?: number;
    };
}
const COOKIE = "cd_online_session";
const BROWSER_COOKIE = "cd_online_browser";
const BODY_LIMIT = 16 * 1024;
class HttpFailure extends Error {
    status: number;
    code: string;
    constructor(status: number, code: string) { super(code); this.status = status; this.code = code; }
}
function respond(response: ServerResponse, status: number, body?: unknown) {
    response.statusCode = status;
    response.setHeader("Cache-Control", "no-store");
    response.setHeader("X-Content-Type-Options", "nosniff");
    if (body !== undefined)
        response.setHeader("Content-Type", "application/json; charset=utf-8");
    response.end(body === undefined ? undefined : JSON.stringify(body));
}
async function readBody(request: IncomingMessage, optional = false): Promise<Record<string, unknown>> {
    const length = request.headers["content-length"];
    if (length && (!/^\d+$/.test(length) || Number(length) > BODY_LIMIT))
        throw new HttpFailure(413, "body_too_large");
    const chunks: Buffer[] = [];
    let total = 0;
    for await (const chunk of request) {
        total += chunk.length;
        if (total > BODY_LIMIT)
            throw new HttpFailure(413, "body_too_large");
        chunks.push(chunk);
    }
    if (optional && total === 0)
        return {};
    if (!/^application\/json(?:\s*;\s*charset=utf-8)?$/i.test(request.headers["content-type"] ?? ""))
        throw new HttpFailure(415, "json_required");
    try {
        const raw = new TextDecoder("utf-8", { fatal: true }).decode(Buffer.concat(chunks));
        const value: unknown = JSON.parse(raw);
        if (!value || typeof value !== "object" || Array.isArray(value))
            throw new Error();
        return value as Record<string, unknown>;
    }
    catch {
        throw new HttpFailure(400, "invalid_body");
    }
}
function cookieValue(request: IncomingMessage, name = COOKIE): string {
    const matches = (request.headers.cookie ?? "").split(";").map(part => part.trim())
        .filter(part => part.startsWith(name + "="));
    return matches.length === 1 ? matches[0]!.slice(name.length + 1) : "";
}
function gameMethod(path: string): string | undefined {
    if (["/activity", "/matches"].includes(path) || /^\/invites\/[A-Za-z0-9_-]+$/.test(path) || /^\/matches\/[A-Za-z0-9-]+$/.test(path))
        return "GET";
    if (["/invites", "/profile/commands"].includes(path) || /^\/invites\/[A-Za-z0-9_-]+\/(accept|cancel|retry)$/.test(path) || /^\/matches\/[A-Za-z0-9-]+\/commands$/.test(path))
        return "POST";
}
async function gameRequest(game: OnlineGameService, path: string, actor: string, body: Record<string, unknown>, now: number) {
    if (path === "/activity")
        return game.activity(actor, now);
    if (path === "/matches")
        return game.history(actor, now);
    if (path === "/invites")
        return game.createInvite(actor, body, now);
    if (path === "/profile/commands")
        return game.profile(actor, body, now);
    const parts = path.split("/");
    if (parts[1] === "matches")
        return parts[3] === "commands" ? game.command(actor, parts[2]!, body, now) : game.match(actor, parts[2]!, now);
    if (parts[3] === "accept")
        return game.acceptInvite(actor, parts[2]!, body, now);
    if (parts[3] === "cancel")
        return game.cancelInvite(actor, parts[2]!, body, now);
    if (parts[3] === "retry")
        return game.retryInvite(actor, parts[2]!, body, now);
    return game.invite(parts[2]!, now);
}
export function createOnlineServer(options: OnlineOptions) {
    const { store, botToken, origin, production = false } = options;
    const basePath = options.basePath ?? "/api/online";
    if (!/^\/(?:[A-Za-z0-9_-]+\/)*[A-Za-z0-9_-]+$/.test(basePath))
        throw new Error("Invalid API base path");
    let originUrl: URL;
    try {
        originUrl = new URL(origin);
    }
    catch {
        throw new Error("Invalid API origin");
    }
    if (!["http:", "https:"].includes(originUrl.protocol) || originUrl.origin !== origin || (production && originUrl.protocol !== "https:"))
        throw new Error("Invalid API origin");
    if (!botToken.trim())
        throw new Error("Bot token required");
    const now = options.now ?? Date.now;
    const limit = options.rateLimit ?? { max: 1000, authMax: 20, windowMs: 60000 };
    const playerMax = limit.playerMax ?? 240;
    const trustedPeers = proxyPeers(options.trustedProxyIps ?? []);
    if (![limit.max, limit.authMax, limit.windowMs, playerMax].every(n => Number.isSafeInteger(n) && n > 0))
        throw new Error("Invalid rate limit");
    const buckets = new Map<string, {
        until: number;
        requests: number;
        auth: number;
    }>();
    const players = new Map<string, {
        until: number;
        requests: number;
    }>();
    const cookie = (token: string, at: number) => `${COOKIE}=${token}; Path=${basePath}; HttpOnly; SameSite=Lax; Max-Age=${token ? SESSION_MS / 1000 : 0}; Expires=${new Date(token ? at + SESSION_MS : 0).toUTCString()}${production ? "; Secure" : ""}`;
    const browserCookie = (value: string) => `${BROWSER_COOKIE}=${value}; Path=${basePath}; HttpOnly; SameSite=Lax; Max-Age=${value ? BROWSER_LOGIN_MS / 1000 : 0}${production ? "; Secure" : ""}`;
    const server = createServer({ maxHeaderSize: 16384 }, async (request, response) => {
        try {
            const at = now();
            const path = (request.url ?? "").startsWith(basePath + "/") ? (request.url ?? "").slice(basePath.length) : "";
            const peer = normalizeIp(request.socket.remoteAddress ?? "") ?? "unknown";
            const supplied = request.headers["x-crowned-client-ip"];
            // Only an exact trusted peer may replace the transport address. XFF is never read.
            const clientIp = trustedPeers.has(peer) && typeof supplied === "string" ? normalizeIp(supplied) : null;
            const key = clientIp ?? peer;
            const consumeIp = () => {
                for (const [ip, bucket] of buckets)
                    if (bucket.until <= at)
                        buckets.delete(ip);
                let bucket = buckets.get(key);
                if (!bucket) {
                    if (buckets.size >= 10000)
                        throw new HttpFailure(429, "rate_limited");
                    bucket = { until: at + limit.windowMs, requests: 0, auth: 0 };
                    buckets.set(key, bucket);
                }
                bucket.requests++;
                if (["/auth/telegram", "/auth/browser/start", "/auth/browser/consume"].includes(path))
                    bucket.auth++;
                if (bucket.requests > limit.max || bucket.auth > limit.authMax) {
                    response.setHeader("Retry-After", String(Math.ceil((bucket.until - at) / 1000)));
                    throw new HttpFailure(429, "rate_limited");
                }
            };
            const accountRoute = path === "/me" || path === "/auth/logout" || Boolean(options.game && gameMethod(path));
            // Successful authenticated polling has no shared IP quota.
            if (!accountRoute)
                consumeIp();
            const routes: Record<string, string> = {
                [basePath + "/health"]: "GET", [basePath + "/me"]: "GET",
                [basePath + "/auth/telegram"]: "POST", [basePath + "/auth/logout"]: "POST",
            };
            if (options.browserLogin) {
                routes[basePath + "/auth/browser/start"] = "POST";
                routes[basePath + "/auth/browser/status"] = "GET";
                routes[basePath + "/auth/browser/consume"] = "POST";
            }
            const method = routes[request.url ?? ""] ?? (options.game ? gameMethod(path) : undefined);
            if (!method)
                throw new HttpFailure(404, "not_found");
            if (request.method !== method) {
                response.setHeader("Allow", method);
                throw new HttpFailure(405, "method_not_allowed");
            }
            if (method === "POST" && request.headers.origin !== origin)
                throw new HttpFailure(403, "bad_origin");
            if (request.url === basePath + "/health") {
                try {
                    if (!(await store.healthy()))
                        throw new Error();
                }
                catch {
                    throw new HttpFailure(503, "unavailable");
                }
                respond(response, 200, { ok: true });
            }
            else if (request.url === basePath + "/auth/telegram") {
                const body = await readBody(request);
                if (Object.keys(body).length !== 1 || typeof body.initData !== "string")
                    throw new HttpFailure(400, "invalid_body");
                let identity;
                try {
                    identity = verifyTelegram(body.initData, botToken, at);
                }
                catch {
                    throw new HttpFailure(401, "invalid_credentials");
                }
                const result = await store.login(identity, at);
                response.setHeader("Set-Cookie", cookie(result.token, at));
                respond(response, 200, { account: result.account, maxLevel });
            }
            else if (options.browserLogin && path.startsWith("/auth/browser/")) {
                const login = options.browserLogin;
                if (path === "/auth/browser/start") {
                    const result = await login.start(await readBody(request), at);
                    response.setHeader("Set-Cookie", browserCookie(result.cookie));
                    respond(response, 200, { challengeId: result.challengeId, botUrl: result.botUrl, expiresAt: result.expiresAt, serverNow: at });
                }
                else if (path === "/auth/browser/status") {
                    respond(response, 200, { ...await login.status(cookieValue(request, BROWSER_COOKIE), at), serverNow: at });
                }
                else {
                    if (Object.keys(await readBody(request, true)).length)
                        throw new HttpFailure(400, "invalid_body");
                    const result = await login.consume(cookieValue(request, BROWSER_COOKIE), at);
                    response.setHeader("Set-Cookie", [cookie(result.token, at), browserCookie("")]);
                    respond(response, 200, { account: result.account, maxLevel, inviteToken: result.inviteToken, serverNow: at });
                }
            }
            else {
                const token = cookieValue(request);
                const account = await store.authenticate(token, at);
                if (!account) {
                    consumeIp();
                    throw new HttpFailure(401, "unauthorized");
                }
                for (const [id, b] of players)
                    if (b.until <= at)
                        players.delete(id);
                let player = players.get(account.id);
                if (!player) {
                    if (players.size >= 10000)
                        throw new HttpFailure(429, "rate_limited");
                    player = { until: at + limit.windowMs, requests: 0 };
                    players.set(account.id, player);
                }
                if (++player.requests > playerMax) {
                    response.setHeader("Retry-After", String(Math.ceil((player.until - at) / 1000)));
                    throw new HttpFailure(429, "rate_limited");
                }
                const body = method === "POST" ? await readBody(request, true) : {};
                if (path === "/auth/logout") {
                    if (Object.keys(body).length)
                        throw new HttpFailure(400, "invalid_body");
                    await store.logout(token, at);
                    response.setHeader("Set-Cookie", cookie("", at));
                    respond(response, 204);
                }
                else if (path === "/me")
                    respond(response, 200, { account, maxLevel });
                else if (options.game) {
                    const result = await gameRequest(options.game, path, account.id, body, at);
                    respond(response, result.status, result.body);
                }
                else
                    throw new HttpFailure(404, "not_found");
            }
        }
        catch (error) {
            if (response.destroyed || response.writableEnded)
                return;
            if (error instanceof HttpFailure)
                respond(response, error.status, { error: error.code });
            else if (error instanceof GameFailure)
                respond(response, error.status, { error: error.message });
            else if (error instanceof TagExhausted)
                respond(response, 503, { error: "tag_capacity_exhausted" });
            else
                respond(response, 500, { error: "internal_error" });
        }
        finally {
            // Drain rejected bounded requests so keep-alive clients receive the safe response.
            if (!request.complete && !request.destroyed)
                request.resume();
        }
    });
    server.requestTimeout = 10000;
    server.headersTimeout = 10000;
    server.keepAliveTimeout = 5000;
    server.setTimeout(15000, socket => socket.destroy());
    return server;
}
