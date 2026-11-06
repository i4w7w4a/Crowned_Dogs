import { Bot, GrammyError, type Context } from "grammy";
import type { UserFromGetMe } from "grammy/types";
import type { BrowserLoginPort } from "./browser-login-types.ts";
import { miniAppUrl } from "./telegram-cards.ts";

export interface OnlineBotOptions { browserLogin: BrowserLoginPort; botUsername: string; webAppUrl: string; botInfo?: UserFromGetMe; now?: () => number }
/** Factory only: the single application lifecycle starts/stops polling. No legacy DuelService. */
export function createOnlineBot(token: string, options: OnlineBotOptions): Bot {
  const bot = new Bot(token, options.botInfo ? { botInfo: options.botInfo } : {});
  const base = new URL(options.webAppUrl);
  if (base.username || base.password || (base.protocol !== "https:" && !(base.protocol === "http:" && ["127.0.0.1", "localhost", "[::1]"].includes(base.hostname)))) throw new Error("Invalid Web App URL");
  miniAppUrl(options.botUsername, "validate");
  const now = options.now ?? Date.now;
  const challengePattern = /^[A-Za-z0-9_-]{32}$/;
  const personal = (ctx: Context) => ctx.chat?.type === "private" && ctx.from && !ctx.from.is_bot && ctx.chat.id === ctx.from.id;
  const openGame = (ctx: Context, invite?: string) => ctx.reply("Crowned Dogs: профиль, вызовы и бои доступны в 3D-игре.", {
    reply_markup: { inline_keyboard: [[{ text: invite ? "Открыть вызов" : "Открыть игру", url: invite ? miniAppUrl(options.botUsername, invite) : `https://t.me/${options.botUsername}?startapp` }]] },
  });
  bot.command("start", async ctx => {
    const payload = ctx.match.trim();
    if (payload.startsWith("login_")) {
      const id = payload.slice(6);
      if (!personal(ctx) || !challengePattern.test(id)) { await ctx.reply("Начни вход заново на сайте и открой ссылку в личном чате с ботом."); return; }
      let login;
      try { login = await options.browserLogin.describe(id, now()); }
      catch { await ctx.reply("Сервис входа временно недоступен. Повтори попытку позже."); return; }
      if (!login || login.expiresAt <= now() || login.origin !== base.origin) { await ctx.reply("Срок запроса истёк. Начни вход заново на сайте."); return; }
      await ctx.reply(`Подтвердить вход на ${login.origin}?\n\nПодтверждай только если ты сам начал вход в своём браузере. Не подтверждай чужую просьбу или присланную ссылку.\nСсылка действует 5 минут.`, {
        link_preview_options: { is_disabled: true },
        reply_markup: { inline_keyboard: [[{ text: "Подтвердить вход", callback_data: `login:${id}` }]] },
      });
      return;
    }
    const invite = payload.startsWith("duel_") ? payload.slice(5) : undefined;
    await openGame(ctx, invite && /^[A-Za-z0-9_-]{1,506}$/.test(invite) ? invite : undefined);
  });
  bot.callbackQuery(/^login:/, async ctx => {
    const id = ctx.callbackQuery.data.slice(6);
    if (!personal(ctx) || !challengePattern.test(id)) { await ctx.answerCallbackQuery({ text: "Открой запрос в личном чате с ботом.", show_alert: true }); return; }
    let result;
    try {
      result = await options.browserLogin.confirm(id, {
        id: String(ctx.from.id), firstName: ctx.from.first_name,
        lastName: ctx.from.last_name ?? null, username: ctx.from.username ?? null,
      }, now());
    } catch { await ctx.answerCallbackQuery({ text: "Сервис входа временно недоступен. Повтори попытку позже.", show_alert: true }); return; }
    await ctx.answerCallbackQuery();
    const text = result === "confirmed" ? "Вход подтверждён. Вернись в браузер, где начал вход."
      : result === "expired" ? "Срок запроса истёк. Начни вход заново на сайте."
      : "Этот запрос уже использован. Начни вход заново на сайте.";
    try { await ctx.editMessageText(text, { reply_markup: { inline_keyboard: [] } }); }
    catch (error) {
      if (!(error instanceof GrammyError && error.error_code === 400 && error.description.startsWith("Bad Request: message is not modified"))) throw error;
    }
  });
  bot.command(["duel", "fight", "me", "rules"], async ctx => { await openGame(ctx); });
  // Old in-chat combat buttons cannot reach the retired in-memory progression.
  bot.on("callback_query:data", async ctx => { await ctx.answerCallbackQuery({ text: "Продолжи в 3D-игре." }); await openGame(ctx); });
  return bot;
}
