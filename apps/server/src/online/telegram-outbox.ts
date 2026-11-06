import { invitationCard, type InvitationCard } from "./telegram-cards.ts";
import { TelegramFailure, type TelegramTransport } from "./telegram-transport.ts";

/** SQL owns leases, per-invitation ordering, retries and the durable Telegram message ID. */
export interface TelegramDelivery {
  leaseId: string;
  kind: "publish" | "update";
  attempts: number;
  chatId: string;
  messageId: number | null;
  card: InvitationCard;
}
export interface TelegramOutboxPort {
  claimDue(now: number, leaseMs: number): Promise<TelegramDelivery | null>;
  complete(leaseId: string, messageId: number | null): Promise<boolean>;
  fail(leaseId: string, failure: { code: string; retryAt: number | null }): Promise<boolean>;
}
export class TelegramOutboxWorker {
  readonly #options: { outbox: TelegramOutboxPort; transport: Pick<TelegramTransport, "deliver">; botUsername: string; now?: () => number };
  #flight: Promise<boolean> | null = null;
  readonly #abort = new AbortController();
  constructor(options: { outbox: TelegramOutboxPort; transport: Pick<TelegramTransport, "deliver">; botUsername: string; now?: () => number }) { this.#options = options; }

  /** One bounded delivery; the application's existing lifecycle owns the scheduler. */
  runOnce(): Promise<boolean> {
    if (this.#flight || this.#abort.signal.aborted) return Promise.resolve(false);
    this.#flight = this.#deliver().finally(() => { this.#flight = null; });
    return this.#flight;
  }
  async stop(): Promise<void> { this.#abort.abort(); await this.#flight; }

  async #deliver(): Promise<boolean> {
    const { outbox, transport, botUsername, now = Date.now } = this.#options;
    // Three sequential HTTP calls, each <=10s, fit the lease with ample margin.
    const delivery = await outbox.claimDue(now(), 60_000);
    if (!delivery) return false;
    if (this.#abort.signal.aborted) return true;
    const { leaseId, card, chatId, messageId } = delivery;
    if (messageId === null && (card.status === "cancelled" || card.status === "expired")) {
      await outbox.complete(leaseId, null);
      return true;
    }
    let sentId: number;
    try {
      sentId = await transport.deliver(chatId, messageId, invitationCard(card, botUsername), this.#abort.signal);
    } catch (error) {
      if (this.#abort.signal.aborted) return true;
      const failure = error instanceof TelegramFailure ? error : new TelegramFailure("telegram_unavailable", 1000);
      const backoff = Math.min(60_000, 1000 * 2 ** Math.max(0, Math.min(6, delivery.attempts - 1)));
      await outbox.fail(leaseId, { code: failure.code, retryAt: failure.retryAfterMs === null ? null : now() + Math.max(failure.retryAfterMs, backoff) });
      return true;
    }
    // Never convert a failed DB acknowledgement into a transport error. Recovery reclaims
    // the lease and may repeat this same card/token, but cannot create a second invitation.
    await outbox.complete(leaseId, sentId);
    return true;
  }
}
