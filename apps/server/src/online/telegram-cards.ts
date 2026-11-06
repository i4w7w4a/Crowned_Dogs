export interface InvitationCard {
  token: string;
  status: "open" | "accepted" | "cancelled" | "expired";
  expiresAt: number;
  host: { telegram: { firstName: string; lastName: string | null; username: string | null }; level: number; tag: string };
}
export interface TelegramCard {
  text: string;
  parse_mode: "HTML";
  reply_markup: { inline_keyboard: { text: string; url: string }[][] };
}
export function miniAppUrl(botUsername: string, token: string): string {
  if (!/^[A-Za-z][A-Za-z0-9_]{4,31}$/.test(botUsername) || !/^[A-Za-z0-9_-]{1,506}$/.test(token)) throw new Error("Invalid Mini App link");
  return `https://t.me/${botUsername}?startapp=duel_${token}`;
}
const escape = (text: string) => text.replaceAll("&", "&amp;").replaceAll("<", "&lt;").replaceAll(">", "&gt;");
const line = (text: string) => text.replace(/[\u0000-\u001f\u007f\u202a-\u202e\u2066-\u2069]/g, " ").slice(0, 256).trim();

/** Only committed server projections enter this renderer. No gameplay state lives here. */
export function invitationCard(invitation: InvitationCard, botUsername: string): TelegramCard {
  const url = miniAppUrl(botUsername, invitation.token);
  const { telegram, level, tag } = invitation.host;
  const name = [telegram.firstName, telegram.lastName].filter(Boolean).map(s => line(s!)).join(" ") || "Игрок";
  const username = telegram.username ? ` (@${escape(line(telegram.username))})` : "";
  const status = { open: "Вызов на дуэль", accepted: "Вызов принят", cancelled: "Вызов отменён", expired: "Срок вызова истёк" }[invitation.status];
  const text = [
    `<b>${status}</b>`, `<b>${escape(name)}</b>${username}`,
    `Уровень: <b>${escape(String(level))}</b> · Жетон: <b>${escape(tag)}</b>`,
    "Дуэль в 3D. Параметры и результат определяет сервер.",
    ...(invitation.status === "open" ? [
      `Действует до ${new Date(invitation.expiresAt).toISOString().slice(0, 16).replace("T", " ")} UTC.`,
      "Открытие ссылки не принимает вызов. Подтверди участие в игре.",
    ] : []),
  ].join("\n");
  return { text, parse_mode: "HTML", reply_markup: { inline_keyboard:
    invitation.status === "open" || invitation.status === "accepted"
      ? [[{ text: invitation.status === "open" ? "Открыть вызов" : "Открыть бой", url }]] : [],
  } };
}
