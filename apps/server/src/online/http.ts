import { createServer } from "node:http";
import type { IncomingMessage, ServerResponse } from "node:http";
import { maxLevel, OnlineStore, SESSION_MS, TagExhausted } from "./storage.ts";
import { verifyTelegram } from "./telegram.ts";

export interface OnlineOptions {
  store: OnlineStore;
  botToken: string;
  origin: string;
  basePath?: string;
  production?: boolean;
  now?: () => number;
  rateLimit?: { max: number; authMax: number; windowMs: number };
}
const COOKIE = "cd_online_session";
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
  if (body !== undefined) response.setHeader("Content-Type", "application/json; charset=utf-8");
  response.end(body === undefined ? undefined : JSON.stringify(body));
}
async function readBody(request: IncomingMessage, optional = false): Promise<Record<string, unknown>> {
  const length = request.headers["content-length"];
  if (length && (!/^\d+$/.test(length) || Number(length) > BODY_LIMIT)) throw new HttpFailure(413, "body_too_large");
  const chunks: Buffer[] = [];
  let total = 0;
  for await (const chunk of request) {
    total += chunk.length;
    if (total > BODY_LIMIT) throw new HttpFailure(413, "body_too_large");
    chunks.push(chunk);
  }
  if (optional && total === 0) return {};
  if (!/^application\/json(?:\s*;\s*charset=utf-8)?$/i.test(request.headers["content-type"] ?? "")) throw new HttpFailure(415, "json_required");
  try {
    const raw = new TextDecoder("utf-8", { fatal: true }).decode(Buffer.concat(chunks));
    const value: unknown = JSON.parse(raw);
    if (!value || typeof value !== "object" || Array.isArray(value)) throw new Error();
    return value as Record<string, unknown>;
  } catch { throw new HttpFailure(400, "invalid_body"); }
}
function sessionToken(request: IncomingMessage): string {
  const matches = (request.headers.cookie ?? "").split(";").map(part => part.trim())
    .filter(part => part.startsWith(COOKIE + "="));
  return matches.length === 1 ? matches[0]!.slice(COOKIE.length + 1) : "";
}

export function createOnlineServer(options: OnlineOptions) {
  const { store, botToken, origin, production = false } = options;
  const basePath = options.basePath ?? "/api/online";
  if (!/^\/(?:[A-Za-z0-9_-]+\/)*[A-Za-z0-9_-]+$/.test(basePath)) throw new Error("Invalid API base path");
  let originUrl: URL;
  try { originUrl = new URL(origin); } catch { throw new Error("Invalid API origin"); }
  if (!["http:", "https:"].includes(originUrl.protocol) || originUrl.origin !== origin || (production && originUrl.protocol !== "https:")) throw new Error("Invalid API origin");
  if (!botToken.trim()) throw new Error("Bot token required");
  const now = options.now ?? Date.now;
  const limit = options.rateLimit ?? { max: 120, authMax: 20, windowMs: 60_000 };
  if (![limit.max, limit.authMax, limit.windowMs].every(n => Number.isSafeInteger(n) && n > 0)) throw new Error("Invalid rate limit");
  const buckets = new Map<string, { until: number; requests: number; auth: number }>();
  const cookie = (token: string, at: number) => `${COOKIE}=${token}; Path=${basePath}; HttpOnly; SameSite=Lax; Max-Age=${token ? SESSION_MS / 1000 : 0}; Expires=${new Date(token ? at + SESSION_MS : 0).toUTCString()}${production ? "; Secure" : ""}`;

  const server = createServer({ maxHeaderSize: 16_384 }, async (request, response) => {
    try {
      const at = now();
      // Trust the socket, not a client supplied X-Forwarded-For. Proxy setup is documented.
      const key = request.socket.remoteAddress ?? "unknown";
      for (const [ip, bucket] of buckets) if (bucket.until <= at) buckets.delete(ip);
      let bucket = buckets.get(key);
      if (!bucket) {
        if (buckets.size >= 10_000) throw new HttpFailure(429, "rate_limited");
        bucket = { until: at + limit.windowMs, requests: 0, auth: 0 };
        buckets.set(key, bucket);
      }
      bucket.requests++;
      if (request.url === basePath + "/auth/telegram") bucket.auth++;
      if (bucket.requests > limit.max || bucket.auth > limit.authMax) {
        response.setHeader("Retry-After", String(Math.ceil((bucket.until - at) / 1000)));
        throw new HttpFailure(429, "rate_limited");
      }
      const routes: Record<string, string> = {
        [basePath + "/health"]: "GET", [basePath + "/me"]: "GET",
        [basePath + "/auth/telegram"]: "POST", [basePath + "/auth/logout"]: "POST",
      };
      const method = routes[request.url ?? ""];
      if (!method) throw new HttpFailure(404, "not_found");
      if (request.method !== method) { response.setHeader("Allow", method); throw new HttpFailure(405, "method_not_allowed"); }
      if (method === "POST" && request.headers.origin !== origin) throw new HttpFailure(403, "bad_origin");

      if (request.url === basePath + "/health") {
        try {
          if (!(await store.healthy())) throw new Error();
        } catch { throw new HttpFailure(503, "unavailable"); }
        respond(response, 200, { ok: true });
      } else if (request.url === basePath + "/auth/telegram") {
        const body = await readBody(request);
        if (Object.keys(body).length !== 1 || typeof body.initData !== "string") throw new HttpFailure(400, "invalid_body");
        let identity;
        try { identity = verifyTelegram(body.initData, botToken, at); }
        catch { throw new HttpFailure(401, "invalid_credentials"); }
        const result = await store.login(identity, at);
        response.setHeader("Set-Cookie", cookie(result.token, at));
        respond(response, 200, { account: result.account, maxLevel });
      } else {
        if (method === "POST" && Object.keys(await readBody(request, true)).length) throw new HttpFailure(400, "invalid_body");
        const token = sessionToken(request);
        const account = await store.authenticate(token, at);
        if (!account) throw new HttpFailure(401, "unauthorized");
        if (method === "POST") {
          await store.logout(token, at);
          response.setHeader("Set-Cookie", cookie("", at));
          respond(response, 204);
        } else respond(response, 200, { account, maxLevel });
      }
    } catch (error) {
      if (response.destroyed || response.writableEnded) return;
      if (error instanceof HttpFailure) respond(response, error.status, { error: error.code });
      else if (error instanceof TagExhausted) respond(response, 503, { error: "tag_capacity_exhausted" });
      else respond(response, 500, { error: "internal_error" });
    } finally {
      // Drain rejected bounded requests so keep-alive clients receive the safe response.
      if (!request.complete && !request.destroyed) request.resume();
    }
  });
  server.requestTimeout = 10_000;
  server.headersTimeout = 10_000;
  server.keepAliveTimeout = 5000;
  server.setTimeout(15_000, socket => socket.destroy());
  return server;
}
