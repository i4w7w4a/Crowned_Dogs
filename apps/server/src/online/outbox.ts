import { randomUUID } from "node:crypto";
import type { Database, Sql } from "./database.ts";
import { expireInviteRow, gameLock, inviteView, uuid } from "./game-service.ts";
import type { TelegramOutboxPort, TelegramDelivery } from "./telegram-outbox.ts";
export class SqlTelegramOutbox implements TelegramOutboxPort {
    readonly db: Database;
    readonly now: () => number;
    constructor(db: Database, now: () => number = Date.now) { this.db = db; this.now = now; }
    async #delivery(tx: Sql, inviteId: string) {
        const latest = (await tx.query("SELECT status,last_error FROM online_outbox WHERE invite_id=$1 ORDER BY sequence DESC LIMIT 1", [inviteId])).rows[0]!;
        await tx.query("UPDATE online_invites SET delivery_status=$2,delivery_error=$3 WHERE id=$1", [inviteId, latest.status, latest.last_error]);
    }
    async claimDue(now: number, leaseMs: number): Promise<(TelegramDelivery & {
        threadId: number;
    }) | null> {
        if (!Number.isSafeInteger(leaseMs) || leaseMs < 1 || leaseMs > 300000)
            throw new Error("Invalid delivery lease");
        return this.db.transaction(async (tx) => {
            await gameLock(tx);
            now = Math.max(now, this.now());
            const row = (await tx.query(`SELECT o.* FROM online_outbox o WHERE
        ((o.status='pending' AND o.available_at<=$1) OR (o.status='sending' AND o.lease_until<=$1))
        AND NOT EXISTS(SELECT 1 FROM online_outbox earlier WHERE earlier.invite_id=o.invite_id AND earlier.sequence<o.sequence AND earlier.status IN ('pending','sending'))
        ORDER BY o.sequence LIMIT 1 FOR UPDATE`, [new Date(now)])).rows[0];
            if (!row)
                return null;
            const initial = (await tx.query("SELECT * FROM online_invites WHERE id=$1", [row.invite_id])).rows[0]!;
            await expireInviteRow(tx, initial, now);
            const leaseId = randomUUID();
            await tx.query("UPDATE online_outbox SET status='sending',attempts=attempts+1,lease_id=$2,lease_until=$3,updated_at=$4 WHERE id=$1", [row.id, leaseId, new Date(now + leaseMs), new Date(now)]);
            await this.#delivery(tx, row.invite_id);
            const inv = (await tx.query("SELECT * FROM online_invites WHERE id=$1", [row.invite_id])).rows[0]!;
            return { leaseId, kind: row.kind, attempts: row.attempts + 1, chatId: inv.chat_id, threadId: inv.thread_id,
                messageId: inv.message_id === null ? null : Number(inv.message_id), card: inviteView(inv, now) };
        });
    }
    async complete(leaseId: string, messageId: number | null): Promise<boolean> {
        if (!uuid(leaseId) || messageId !== null && (!Number.isSafeInteger(messageId) || messageId < 1))
            return false;
        return this.db.transaction(async (tx) => {
            await gameLock(tx);
            const now = this.now();
            const row = (await tx.query("UPDATE online_outbox SET status='published',lease_id=NULL,lease_until=NULL,last_error=NULL,updated_at=$2 WHERE lease_id=$1 AND status='sending' AND lease_until>$2 RETURNING invite_id", [leaseId, new Date(now)])).rows[0];
            if (!row)
                return false;
            if (messageId !== null)
                await tx.query("UPDATE online_invites SET message_id=$2 WHERE id=$1", [row.invite_id, messageId]);
            await this.#delivery(tx, row.invite_id);
            return true;
        });
    }
    async fail(leaseId: string, failure: {
        code: string;
        retryAt: number | null;
    }): Promise<boolean> {
        if (!uuid(leaseId))
            return false;
        const codes = ["chat_not_allowed", "bot_forbidden", "telegram_rate_limited", "telegram_unavailable", "telegram_bad_request", "telegram_topic_unavailable"];
        const code = codes.includes(failure.code) ? failure.code : "telegram_unavailable";
        return this.db.transaction(async (tx) => {
            await gameLock(tx);
            const now = this.now();
            const retry = failure.retryAt !== null && Number.isSafeInteger(failure.retryAt) ? Math.max(now, failure.retryAt) : null;
            const row = (await tx.query("UPDATE online_outbox SET status=$2,available_at=$3,lease_id=NULL,lease_until=NULL,last_error=$4,updated_at=$5 WHERE lease_id=$1 AND status='sending' AND lease_until>$5 RETURNING invite_id", [leaseId, retry === null ? "failed" : "pending", new Date(retry ?? now), code, new Date(now)])).rows[0];
            if (!row)
                return false;
            await this.#delivery(tx, row.invite_id);
            return true;
        });
    }
}
