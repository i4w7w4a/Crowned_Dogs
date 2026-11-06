import type { TelegramCard } from "./telegram-cards.ts";

export type TelegramFailureCode = "chat_not_allowed" | "bot_forbidden" | "telegram_rate_limited" | "telegram_unavailable" | "telegram_bad_request" | "telegram_topic_unavailable";
export class TelegramFailure extends Error {
  code: TelegramFailureCode;
  retryAfterMs: number | null;
  constructor(code: TelegramFailureCode, retryAfterMs: number | null = null) { super(code); this.code = code; this.retryAfterMs = retryAfterMs; }
}
export interface TelegramTransportOptions { token: string; allowedChatIds: readonly string[]; apiRoot?: string; timeoutMs?: number }
export class TelegramTransport {
  readonly #token: string;
  readonly #botId: number;
  readonly #allowed: ReadonlySet<string>;
  readonly #root: string;
  readonly #timeout: number;
  constructor(options: TelegramTransportOptions) {
    if (!/^\d+:[A-Za-z0-9_-]+$/.test(options.token)) throw new Error("Invalid bot configuration");
    this.#token = options.token;
    this.#botId = Number(options.token.split(":")[0]);
    this.#allowed = new Set(options.allowedChatIds);
    if (!Number.isSafeInteger(this.#botId) || this.#botId <= 0 || [...this.#allowed].some(id => !/^-[1-9]\d*$/.test(id) || !Number.isSafeInteger(Number(id)))) throw new Error("Invalid bot configuration");
    const root = new URL(options.apiRoot ?? "https://api.telegram.org");
    // HTTP is only useful for the controlled loopback API. Do not leak tokens through redirects.
    if (root.username || root.password || root.search || root.hash || root.pathname !== "/" ||
      (root.protocol !== "https:" && !(root.protocol === "http:" && ["127.0.0.1", "[::1]"].includes(root.hostname)))) throw new Error("Invalid Telegram API root");
    this.#root = root.origin;
    this.#timeout = options.timeoutMs ?? 8000;
    if (!Number.isSafeInteger(this.#timeout) || this.#timeout < 1 || this.#timeout > 10_000) throw new Error("Invalid Telegram timeout");
  }

  async #call(method: string, body: Record<string, unknown>, signal?: AbortSignal): Promise<any> {
    try {
      const response = await fetch(`${this.#root}/bot${this.#token}/${method}`, {
        method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify(body),
        signal: signal ? AbortSignal.any([signal, AbortSignal.timeout(this.#timeout)]) : AbortSignal.timeout(this.#timeout), redirect: "error",
      });
      const data = await response.json() as Record<string, any>;
      if (data?.ok === true && response.ok) return data.result;
      const code = Number(data?.error_code ?? response.status);
      if (code === 429) {
        const seconds = Number(data.parameters?.retry_after);
        throw new TelegramFailure("telegram_rate_limited", Number.isSafeInteger(seconds * 1000) && seconds > 0 ? seconds * 1000 : 1000);
      }
      if (code >= 500) throw new TelegramFailure("telegram_unavailable", 1000);
      if (code === 401 || code === 403) throw new TelegramFailure("bot_forbidden");
      if (code === 400 && typeof data.description === "string" && /TOPIC_CLOSED|TOPIC_DELETED|TOPIC_ID_INVALID|message thread not found/i.test(data.description)) throw new TelegramFailure("telegram_topic_unavailable");
      if (code === 400 && method === "editMessageText" && typeof data.description === "string" && data.description.startsWith("Bad Request: message is not modified")) return { message_id: body.message_id };
      throw new TelegramFailure("telegram_bad_request");
    } catch (error) {
      if (error instanceof TelegramFailure) throw error;
      throw new TelegramFailure("telegram_unavailable", 1000);
    }
  }

  async deliver(chatId: string, messageId: number | null, card: TelegramCard, threadId: number, signal?: AbortSignal): Promise<number> {
    if (!this.#allowed.has(chatId)) throw new TelegramFailure("chat_not_allowed");
    if (!Number.isSafeInteger(threadId) || threadId < 1) throw new TelegramFailure("telegram_bad_request");
    if (messageId !== null && (!Number.isSafeInteger(messageId) || messageId < 1)) throw new TelegramFailure("telegram_bad_request");
    const chat = await this.#call("getChat", { chat_id: chatId }, signal);
    if (!chat || String(chat.id) !== chatId || !["group", "supergroup"].includes(chat.type)) throw new TelegramFailure("bot_forbidden");
    if (chat.is_forum !== true) throw new TelegramFailure("telegram_topic_unavailable");
    const member = await this.#call("getChatMember", { chat_id: chatId, user_id: this.#botId }, signal);
    const canSend = member?.status === "creator" || member?.status === "administrator" ||
      (member?.status === "member" && chat.permissions?.can_send_messages === true) ||
      (member?.status === "restricted" && member.is_member === true && member.can_send_messages === true);
    if (!canSend) throw new TelegramFailure("bot_forbidden");
    const result = await this.#call(messageId === null ? "sendMessage" : "editMessageText", {
      chat_id: chatId, ...(messageId === null ? { message_thread_id: threadId } : { message_id: messageId }), ...card,
      link_preview_options: { is_disabled: true },
    }, signal);
    if (!Number.isSafeInteger(result?.message_id) || result.message_id < 1 || (messageId !== null && result.message_id !== messageId)) throw new TelegramFailure("telegram_unavailable", 1000);
    return result.message_id;
  }
}
