import { createHmac, timingSafeEqual } from "node:crypto";

export interface TelegramIdentity {
  id: string;
  firstName: string;
  lastName: string | null;
  username: string | null;
}

/** Authenticate raw Mini App initData, never initDataUnsafe or request identity fields. */
export function verifyTelegram(raw: string, botToken: string, now = Date.now()): TelegramIdentity {
  try {
    if (!botToken || !raw || Buffer.byteLength(raw) > 16_384) throw new Error();
    const fields = new Map<string, string>();
    const decode = (s: string) => decodeURIComponent(s.replaceAll("+", " "));
    for (const entry of raw.split("&")) {
      const equals = entry.indexOf("=");
      if (equals < 1) throw new Error();
      const key = decode(entry.slice(0, equals));
      const value = decode(entry.slice(equals + 1));
      if (fields.has(key)) throw new Error();
      fields.set(key, value);
    }
    const hash = fields.get("hash") ?? "";
    const date = fields.get("auth_date") ?? "";
    if (!/^[a-fA-F0-9]{64}$/.test(hash) || !/^\d+$/.test(date)) throw new Error();
    const seconds = Number(date);
    if (!Number.isSafeInteger(seconds) || seconds <= 0 || seconds < now / 1000 - 300 || seconds > now / 1000 + 30) throw new Error();
    fields.delete("hash");
    const check = [...fields].sort(([a], [b]) => a < b ? -1 : a > b ? 1 : 0)
      .map(([key, value]) => `${key}=${value}`).join("\n");
    const secret = createHmac("sha256", "WebAppData").update(botToken).digest();
    const expected = createHmac("sha256", secret).update(check).digest();
    if (!timingSafeEqual(expected, Buffer.from(hash, "hex"))) throw new Error();
    const user: unknown = JSON.parse(fields.get("user") ?? "");
    if (!user || typeof user !== "object" || Array.isArray(user)) throw new Error();
    const u = user as Record<string, unknown>;
    if (!Number.isSafeInteger(u.id) || Number(u.id) <= 0 || typeof u.first_name !== "string" || !u.first_name || u.first_name.length > 256) throw new Error();
    const optional = (value: unknown): string | null => {
      if (value === undefined) return null;
      if (typeof value !== "string" || value.length > 256) throw new Error();
      return value;
    };
    return { id: String(u.id), firstName: u.first_name, lastName: optional(u.last_name), username: optional(u.username) };
  } catch {
    throw new Error("Invalid Telegram credentials");
  }
}
