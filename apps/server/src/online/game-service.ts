import { createHash, randomBytes, randomUUID } from "node:crypto";
import { callRust } from "../../../../packages/runtime/src/index.ts";
import type { Database, Sql } from "./database.ts";
import { account, maxLevel } from "./storage.ts";
export interface GameResponse {
    status: number;
    body: Record<string, any>;
}
export const uuid = (id: unknown): id is string => typeof id === "string" && /^[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i.test(id);
export const inviteToken = (id: unknown): id is string => typeof id === "string" && /^[A-Za-z0-9_-]{32}$/.test(id);
export class GameFailure extends Error {
    readonly status: number;
    constructor(code: string, status = 400) { super(code); this.status = status; }
}
const statuses: Record<string, number> = { not_found: 404, stale_version: 409, stale_turn: 409, busy: 409, already_submitted: 409, invite_closed: 409, command_id_conflict: 409, deadline_passed: 410, incompatible_ruleset: 503 };
function rust<T = any>(method: string, args: unknown): T {
    try {
        return callRust<T>(`server.online.${method}`, args);
    }
    catch (error) {
        const code = error instanceof Error ? error.message : "invalid_command";
        throw new GameFailure(code in statuses ? code : ["invalid_command", "incomplete_order"].includes(code) ? code : "invalid_command", statuses[code] ?? 400);
    }
}
export async function gameLock(tx: Sql) { await tx.query("SELECT id FROM online_game_gate WHERE id=1 FOR UPDATE"); }
const at = (time: number) => new Date(time);
export const millis = (value: unknown) => new Date(value as string).getTime();
function stable(value: any): string {
    if (Array.isArray(value))
        return `[${value.map(stable).join(",")}]`;
    if (value && typeof value === "object")
        return `{${Object.keys(value).sort().map(k => `${JSON.stringify(k)}:${stable(value[k])}`).join(",")}}`;
    return JSON.stringify(value);
}
function exact(body: any, keys: string[]) { return body && typeof body === "object" && !Array.isArray(body) && Object.keys(body).length === keys.length && keys.every(k => Object.hasOwn(body, k)); }
export async function queueInvite(tx: Sql, id: string, kind: "publish" | "update", now: number) {
    await tx.query("INSERT INTO online_outbox(id,invite_id,kind,available_at,created_at,updated_at) VALUES($1,$2,$3,$4,$4,$4)", [randomUUID(), id, kind, at(now)]);
    await tx.query("UPDATE online_invites SET delivery_status='pending',delivery_error=NULL WHERE id=$1", [id]);
}
export function inviteView(row: Record<string, any>, now: number) {
    return { id: row.id, token: row.token, version: row.version, status: row.status, host: rust("player", [row.host_snapshot]), expiresAt: millis(row.expires_at), matchId: row.match_id,
        delivery: { status: row.delivery_status, error: row.delivery_error }, serverNow: now };
}
export async function expireInviteRow(tx: Sql, row: Record<string, any>, now: number) {
    if (row.status === "open" && millis(row.expires_at) <= now) {
        const status = rust("invite", [{ status: row.status, hostId: row.host_id, version: row.version, expiresAt: millis(row.expires_at) }, "", "expire", row.version, now]);
        await tx.query("UPDATE online_invites SET status=$2,version=version+1,updated_at=$3 WHERE id=$1", [row.id, status, at(now)]);
        await tx.query("DELETE FROM online_player_slots WHERE invite_id=$1", [row.id]);
        await queueInvite(tx, row.id, "update", now);
        return (await tx.query("SELECT * FROM online_invites WHERE id=$1", [row.id])).rows[0]!;
    }
    return row;
}
/** Short serial transactions only. All callers supply a session-authenticated actor. */
export class OnlineGameService {
    readonly db: Database;
    readonly chatId: string | null;
    readonly threadId: number;
    readonly clock: () => number;
    constructor(db: Database, options: {
        chatId: string | null;
        threadId: number;
        clock?: () => number;
    }) {
        if (options.chatId !== null && !/^-\d{1,20}$/.test(options.chatId))
            throw new Error("Invalid game group");
        this.db = db;
        this.chatId = options.chatId;
        this.threadId = options.threadId;
        this.clock = options.clock ?? Date.now;
        if (!Number.isSafeInteger(this.threadId) || this.threadId < 1)
            throw new Error("Invalid game topic");
    }
    async #read(run: (tx: Sql, now: number) => Promise<Record<string, any>>, now: number): Promise<GameResponse> {
        return this.db.transaction(async (tx) => {
            await gameLock(tx);
            now = Math.max(now, this.clock());
            try {
                return { status: 200, body: { ...await run(tx, now), serverNow: now } };
            }
            catch (e) {
                if (e instanceof GameFailure)
                    return { status: e.status, body: { error: e.message, serverNow: now } };
                throw e;
            }
        });
    }
    async #write(actor: string, scope: string, body: any, keys: string[], now: number, run: (tx: Sql, now: number) => Promise<Record<string, any>>): Promise<GameResponse> {
        if (!exact(body, keys) || !uuid(body.commandId) || (keys.includes("expectedVersion") && (!Number.isSafeInteger(body.expectedVersion) || body.expectedVersion < 1)) ||
            (keys.includes("turn") && (!Number.isSafeInteger(body.turn) || body.turn < 1)))
            return { status: 400, body: { error: "invalid_command", serverNow: now } };
        const hash = createHash("sha256").update(stable({ scope, body })).digest("hex");
        return this.db.transaction(async (tx) => {
            await gameLock(tx);
            const receipt = (await tx.query("SELECT * FROM online_command_receipts WHERE operator_id=$1 AND command_id=$2", [actor, body.commandId])).rows[0];
            now = Math.max(now, this.clock());
            if (receipt)
                return receipt.request_hash === hash ? { status: receipt.status, body: receipt.response } : { status: 409, body: { error: "command_id_conflict", serverNow: now } };
            let result: GameResponse;
            try {
                result = { status: 200, body: { ...await run(tx, now), serverNow: now } };
            }
            catch (e) {
                if (!(e instanceof GameFailure))
                    throw e;
                result = { status: e.status, body: { error: e.message, serverNow: now } };
            }
            await tx.query("INSERT INTO online_command_receipts(operator_id,command_id,request_hash,status,response,created_at) VALUES($1,$2,$3,$4,$5::jsonb,$6)", [actor, body.commandId, hash, result.status, JSON.stringify(result.body), at(now)]);
            return result;
        });
    }
    async #account(tx: Sql, id: string) {
        const row = (await tx.query("SELECT * FROM online_operators WHERE id=$1 FOR UPDATE", [id])).rows[0];
        if (!row)
            throw new GameFailure("not_found", 404);
        return account(row);
    }
    async #invite(tx: Sql, token: string, now: number) {
        if (!inviteToken(token))
            throw new GameFailure("not_found", 404);
        let row = (await tx.query("SELECT * FROM online_invites WHERE token=$1 FOR UPDATE", [token])).rows[0];
        if (!row)
            throw new GameFailure("not_found", 404);
        return expireInviteRow(tx, row, now);
    }
    async #free(tx: Sql, actor: string, now: number) {
        const slot = (await tx.query("SELECT s.*,i.token FROM online_player_slots s LEFT JOIN online_invites i ON s.invite_id=i.id WHERE s.operator_id=$1", [actor])).rows[0];
        if (slot?.token)
            await this.#invite(tx, slot.token, now);
        if (slot?.match_id)
            await this.#match(tx, actor, slot.match_id, now);
        if ((await tx.query("SELECT operator_id FROM online_player_slots WHERE operator_id=$1", [actor])).rows.length)
            throw new GameFailure("busy", 409);
    }
    async #save(tx: Sql, s: any, now: number) {
        const deadline = s.phase === "loading" ? s.loadingDeadline : s.deadline;
        await tx.query("UPDATE online_matches SET snapshot=$2::jsonb,phase=$3,deadline=$4,version=$5,updated_at=$6 WHERE id=$1", [s.id, JSON.stringify(s), s.phase, deadline === null ? null : at(deadline), s.version, at(now)]);
        if (s.phase === "finished") {
            const rewards = rust<any[]>("rewards", [s]);
            for (let side = 0; side < 2; side++) {
                const id = s.players[side].id;
                const inserted = await tx.query("INSERT INTO online_reward_ledger(match_id,operator_id,reward,created_at) VALUES($1,$2,$3::jsonb,$4) ON CONFLICT DO NOTHING RETURNING operator_id", [s.id, id, JSON.stringify(rewards[side]), at(now)]);
                if (inserted.rows.length)
                    await tx.query("UPDATE online_operators SET profile=$2::jsonb,version=version+1,updated_at=$3 WHERE id=$1", [id, JSON.stringify(rewards[side].profile), at(now)]);
            }
        }
        if (s.phase === "finished" || s.phase === "cancelled")
            await tx.query("DELETE FROM online_player_slots WHERE match_id=$1", [s.id]);
    }
    async #match(tx: Sql, actor: string, id: string, now: number): Promise<{
        state: any;
        advanced: boolean;
    }> {
        if (!uuid(id))
            throw new GameFailure("not_found", 404);
        const row = (await tx.query("SELECT m.* FROM online_matches m JOIN online_match_players p ON p.match_id=m.id WHERE m.id=$1 AND p.operator_id=$2 FOR UPDATE OF m", [id, actor])).rows[0];
        if (!row)
            throw new GameFailure("not_found", 404);
        let state = row.snapshot;
        let advanced = false;
        if (["loading", "active"].includes(row.phase) && millis(row.deadline) <= now) {
            state = rust("timeout", [state, now]);
            await this.#save(tx, state, now);
            advanced = true;
        }
        return { state, advanced };
    }
    async #view(tx: Sql, state: any, actor: string, now: number) {
        const view = rust("project", [state, actor]);
        const reward = (await tx.query("SELECT reward FROM online_reward_ledger WHERE match_id=$1 AND operator_id=$2", [state.id, actor])).rows[0];
        view.reward = reward?.reward ?? null;
        view.serverNow = now;
        return view;
    }
    createInvite(actor: string, body: any, now: number) {
        return this.#write(actor, "createInvite", body, ["commandId"], now, async (tx, now) => {
            if (!this.chatId)
                throw new GameFailure("chat_not_allowed", 409);
            await this.#free(tx, actor, now);
            const host = await this.#account(tx, actor);
            const id = randomUUID();
            const token = randomBytes(24).toString("base64url");
            await tx.query("INSERT INTO online_invites(id,token,host_id,host_snapshot,status,expires_at,chat_id,thread_id,created_at,updated_at) VALUES($1,$2,$3,$4::jsonb,'open',$5,$6,$7,$8,$8)", [id, token, actor, JSON.stringify(host), at(now + 1800000), this.chatId, this.threadId, at(now)]);
            await tx.query("INSERT INTO online_player_slots(operator_id,invite_id) VALUES($1,$2)", [actor, id]);
            await queueInvite(tx, id, "publish", now);
            return { invite: inviteView(await this.#invite(tx, token, now), now) };
        });
    }
    invite(token: string, now: number) { return this.#read(async (tx, now) => ({ invite: inviteView(await this.#invite(tx, token, now), now) }), now); }
    acceptInvite(actor: string, token: string, body: any, now: number) {
        return this.#write(actor, `accept:${token}`, body, ["commandId", "expectedVersion"], now, async (tx, now) => {
            const inv = await this.#invite(tx, token, now);
            rust("invite", [{ status: inv.status, hostId: inv.host_id, version: inv.version, expiresAt: millis(inv.expires_at) }, actor, "accept", body.expectedVersion, now]);
            await this.#free(tx, actor, now);
            const host = await this.#account(tx, inv.host_id);
            const guest = await this.#account(tx, actor);
            const s = rust("create", [{ id: randomUUID(), players: [host, guest], seed: randomBytes(32).toString("hex"), now }]);
            await tx.query("INSERT INTO online_matches(id,snapshot,phase,deadline,version,created_at,updated_at) VALUES($1,$2::jsonb,$3,$4,$5,$6,$6)", [s.id, JSON.stringify(s), s.phase, at(s.loadingDeadline), s.version, at(now)]);
            for (let side = 0; side < 2; side++)
                await tx.query("INSERT INTO online_match_players(match_id,operator_id,side) VALUES($1,$2,$3)", [s.id, s.players[side].id, side]);
            await tx.query("UPDATE online_player_slots SET invite_id=NULL,match_id=$2 WHERE operator_id=$1", [inv.host_id, s.id]);
            await tx.query("INSERT INTO online_player_slots(operator_id,match_id) VALUES($1,$2)", [actor, s.id]);
            await tx.query("UPDATE online_invites SET status='accepted',match_id=$2,version=version+1,updated_at=$3 WHERE id=$1", [inv.id, s.id, at(now)]);
            await queueInvite(tx, inv.id, "update", now);
            return { invite: inviteView(await this.#invite(tx, token, now), now), battle: await this.#view(tx, s, actor, now) };
        });
    }
    cancelInvite(actor: string, token: string, body: any, now: number) { return this.#inviteAction(actor, token, body, now, "cancel"); }
    retryInvite(actor: string, token: string, body: any, now: number) { return this.#inviteAction(actor, token, body, now, "retry"); }
    #inviteAction(actor: string, token: string, body: any, now: number, action: "cancel" | "retry") {
        return this.#write(actor, `${action}:${token}`, body, ["commandId", "expectedVersion"], now, async (tx, now) => {
            const inv = await this.#invite(tx, token, now);
            const status = rust("invite", [{ status: inv.status, hostId: inv.host_id, version: inv.version, expiresAt: millis(inv.expires_at) }, actor, action, body.expectedVersion, now]);
            if (action === "retry" && inv.delivery_status !== "failed")
                throw new GameFailure("invalid_command");
            await tx.query("UPDATE online_invites SET status=$2,version=version+1,updated_at=$3 WHERE id=$1", [inv.id, status, at(now)]);
            if (action === "cancel")
                await tx.query("DELETE FROM online_player_slots WHERE invite_id=$1", [inv.id]);
            await queueInvite(tx, inv.id, "update", now);
            return { invite: inviteView(await this.#invite(tx, token, now), now) };
        });
    }
    command(actor: string, id: string, body: any, now: number) {
        return this.#write(actor, `command:${id}`, body, ["commandId", "expectedVersion", "turn", "action"], now, async (tx, now) => {
            const current = await this.#match(tx, actor, id, now);
            if (current.advanced)
                throw new GameFailure("deadline_passed", 410);
            const s = rust("transition", [current.state, actor, body, now]);
            await this.#save(tx, s, now);
            return { battle: await this.#view(tx, s, actor, now) };
        });
    }
    match(actor: string, id: string, now: number) { return this.#read(async (tx, now) => ({ battle: await this.#view(tx, (await this.#match(tx, actor, id, now)).state, actor, now) }), now); }
    activity(actor: string, now: number) {
        return this.#read(async (tx, now) => {
            const slot = (await tx.query("SELECT s.*,i.token FROM online_player_slots s LEFT JOIN online_invites i ON i.id=s.invite_id WHERE s.operator_id=$1", [actor])).rows[0];
            if (slot?.token)
                return { invite: inviteView(await this.#invite(tx, slot.token, now), now), battle: null };
            if (slot?.match_id)
                return { invite: null, battle: await this.#view(tx, (await this.#match(tx, actor, slot.match_id, now)).state, actor, now) };
            return { invite: null, battle: null };
        }, now);
    }
    history(actor: string, now: number) { return this.#read(async (tx) => ({ matches: (await tx.query("SELECT m.id,m.phase,m.snapshot,m.updated_at FROM online_matches m JOIN online_match_players p ON p.match_id=m.id WHERE p.operator_id=$1 ORDER BY m.created_at DESC,m.id LIMIT 30", [actor])).rows.map(r => ({ id: r.id, phase: r.phase, result: rust("project", [r.snapshot, actor]).result, updatedAt: millis(r.updated_at) })) }), now); }
    profile(actor: string, body: any, now: number) {
        return this.#write(actor, "profile", body, ["commandId", "expectedVersion", "action"], now, async (tx, now) => {
            await this.#free(tx, actor, now);
            const p = await this.#account(tx, actor);
            if (p.version !== body.expectedVersion)
                throw new GameFailure("stale_version", 409);
            const next = rust("profile", [p, body.action]);
            const row = (await tx.query("UPDATE online_operators SET profile=$2::jsonb,appearance=$3::jsonb,loadout=$4::jsonb,version=version+1,updated_at=$5 WHERE id=$1 RETURNING *", [actor, JSON.stringify(next.profile), JSON.stringify(next.appearance), JSON.stringify(next.loadout), at(now)])).rows[0]!;
            return { account: account(row), maxLevel };
        });
    }
    async tick(now: number): Promise<void> {
        await this.db.transaction(async (tx) => {
            await gameLock(tx);
            now = Math.max(now, this.clock());
            const invites = await tx.query("SELECT token FROM online_invites WHERE status='open' AND expires_at<=$1 ORDER BY expires_at LIMIT 100", [at(now)]);
            for (const inv of invites.rows)
                await this.#invite(tx, inv.token, now);
            const matches = await tx.query("SELECT snapshot FROM online_matches WHERE phase IN ('loading','active') AND deadline<=$1 ORDER BY deadline LIMIT 100 FOR UPDATE", [at(now)]);
            for (const row of matches.rows)
                await this.#save(tx, rust("timeout", [row.snapshot, now]), now);
        });
    }
}
