import { createHash, randomBytes } from "node:crypto";
import { maxLevel, type OnlineStore } from "./storage.ts";
import type { Sql } from "./database.ts";
import type { BrowserLoginIdentity, BrowserLoginPort } from "./browser-login-types.ts";
import { GameFailure, inviteToken, millis } from "./game-service.ts";
export const BROWSER_LOGIN_MS = 300000;
const hash = (nonce: string) => createHash("sha256").update(nonce).digest("hex");
export class BrowserLoginService implements BrowserLoginPort {
    readonly store: OnlineStore;
    readonly origin: string;
    readonly botUsername: string;
    readonly clock: () => number;
    constructor(store: OnlineStore, options: {
        origin: string;
        botUsername: string;
        clock?: () => number;
    }) {
        const url = new URL(options.origin);
        if (url.origin !== options.origin || !["https:", "http:"].includes(url.protocol) || !/^[A-Za-z0-9_]{5,32}$/.test(options.botUsername))
            throw new Error("Invalid browser login configuration");
        this.store = store;
        this.origin = options.origin;
        this.botUsername = options.botUsername;
        this.clock = options.clock ?? Date.now;
    }
    async start(body: Record<string, unknown>, now: number) {
        now = Math.max(now, this.clock());
        if (Object.keys(body).some(k => k !== "inviteToken") || (body.inviteToken !== undefined && !inviteToken(body.inviteToken)))
            throw new GameFailure("invalid_body");
        const id = randomBytes(24).toString("base64url");
        const nonce = randomBytes(32).toString("base64url");
        const expiresAt = now + BROWSER_LOGIN_MS;
        await this.store.db.query("INSERT INTO online_browser_challenges(id,nonce_hash,invite_token,created_at,expires_at) VALUES($1,$2,$3,$4,$5)", [id, hash(nonce), body.inviteToken ?? null, new Date(now), new Date(expiresAt)]);
        return { challengeId: id, botUrl: `https://t.me/${this.botUsername}?start=login_${id}`, expiresAt, cookie: `${id}.${nonce}` };
    }
    async #row(tx: Sql, cookie: string, lock = false) {
        if (!/^[A-Za-z0-9_-]{32}\.[A-Za-z0-9_-]{43}$/.test(cookie))
            return undefined;
        const [id, nonce] = cookie.split(".");
        return (await tx.query(`SELECT * FROM online_browser_challenges WHERE id=$1 AND nonce_hash=$2${lock ? " FOR UPDATE" : ""}`, [id, hash(nonce!)])).rows[0];
    }
    async status(cookie: string, now: number): Promise<{
        status: "pending" | "confirmed" | "expired";
        expiresAt: number | null;
    }> {
        const row = await this.#row(this.store.db, cookie);
        now = Math.max(now, this.clock());
        if (!row || row.consumed_at || millis(row.expires_at) <= now)
            return { status: "expired", expiresAt: row ? millis(row.expires_at) : null };
        return { status: row.identity ? "confirmed" : "pending", expiresAt: millis(row.expires_at) };
    }
    async describe(id: string, now: number) {
        now = Math.max(now, this.clock());
        if (!inviteToken(id))
            return null;
        const row = (await this.store.db.query("SELECT expires_at FROM online_browser_challenges WHERE id=$1 AND consumed_at IS NULL AND expires_at>$2", [id, new Date(now)])).rows[0];
        return row ? { origin: this.origin, expiresAt: millis(row.expires_at) } : null;
    }
    async confirm(id: string, identity: BrowserLoginIdentity, now: number): Promise<"confirmed" | "expired" | "already_confirmed"> {
        if (!inviteToken(id))
            return "expired";
        if (!/^[1-9]\d*$/.test(identity.id) || !Number.isSafeInteger(Number(identity.id)) || !identity.firstName || identity.firstName.length > 256 ||
            [identity.lastName, identity.username].some(v => v !== null && (typeof v !== "string" || v.length > 256)))
            throw new GameFailure("invalid_identity");
        return this.store.db.transaction(async (tx) => {
            const row = (await tx.query("SELECT * FROM online_browser_challenges WHERE id=$1 FOR UPDATE", [id])).rows[0];
            now = Math.max(now, this.clock());
            if (!row || row.consumed_at || millis(row.expires_at) <= now)
                return "expired";
            if (row.identity)
                return row.identity.id === identity.id ? "confirmed" : "already_confirmed";
            await tx.query("UPDATE online_browser_challenges SET identity=$2::jsonb WHERE id=$1", [id, JSON.stringify(identity)]);
            return "confirmed";
        });
    }
    async consume(cookie: string, now: number) {
        return this.store.db.transaction(async (tx) => {
            const row = await this.#row(tx, cookie, true);
            now = Math.max(now, this.clock());
            if (!row || row.consumed_at || millis(row.expires_at) <= now)
                throw new GameFailure("browser_login_expired", 410);
            if (!row.identity)
                throw new GameFailure("browser_login_pending", 409);
            const result = await this.store.loginInTransaction(tx, row.identity, now);
            await tx.query("UPDATE online_browser_challenges SET consumed_at=$2 WHERE id=$1", [row.id, new Date(now)]);
            return { ...result, maxLevel, inviteToken: row.invite_token };
        });
    }
}
