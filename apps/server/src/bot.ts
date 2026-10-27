// Telegram front end. Only wiring: rules live in @crowned-dogs/core,
// fights in DuelService, texts and buttons in view.ts.

import { Bot, GrammyError, InlineKeyboard, type Context } from "grammy";
import type { InlineKeyboardButton, UserFromGetMe } from "grammy/types";
import { WEAPON_IDS, other, type Side, type WeaponId } from "@crowned-dogs/core";
import { decodeCommand } from "./draft.ts";
import { DuelError, DuelService, type Duel, type DuelEvent } from "./duels.ts";
import {
  RULES, WELCOME, escapeHtml, finalScreen, menuButtons, turnReport, turnScreen, weaponPicker,
  type Button, type PickMode,
} from "./view.ts";

function keyboard(rows: Button[][]): InlineKeyboard {
  return new InlineKeyboard(rows.map((r) => r.map((b): InlineKeyboardButton =>
    b.url ? { text: b.text, url: b.url } : { text: b.text, callback_data: b.data })));
}

const isWeapon = (w: string): w is WeaponId => (WEAPON_IDS as readonly string[]).includes(w);

/** Telegram complains when an edit changes nothing; that is fine for us. */
function notModified(err: unknown): boolean {
  return err instanceof GrammyError && err.description.includes("message is not modified");
}

export function createBot(token: string, opts: { botInfo?: UserFromGetMe } = {}): { bot: Bot; duels: DuelService } {
  const bot = new Bot(token, opts.botInfo ? { botInfo: opts.botInfo } : {});
  /** The message with the live turn buttons for each human side: `${duelId}:${side}`. */
  const screens = new Map<string, number>();

  const html = { parse_mode: "HTML" as const };

  async function showTurn(duel: Duel, side: Side, prefix = ""): Promise<void> {
    const player = duel.players[side];
    const s = turnScreen(duel, side);
    const msg = await bot.api.sendMessage(Number(player.id), prefix + s.text, { ...html, reply_markup: keyboard(s.buttons) });
    screens.set(`${duel.id}:${side}`, msg.message_id);
  }

  /** Takes the buttons off the previous turn's message so old taps cannot confuse anyone. */
  async function retireScreen(duel: Duel, side: Side): Promise<void> {
    const key = `${duel.id}:${side}`;
    const id = screens.get(key);
    screens.delete(key);
    if (id === undefined) return;
    try {
      await bot.api.editMessageReplyMarkup(Number(duel.players[side].id), id, { reply_markup: new InlineKeyboard() });
    } catch (err) {
      if (!notModified(err)) console.warn("could not retire a turn screen", err);
    }
  }

  async function onEvent(e: DuelEvent): Promise<void> {
    const { duel } = e;
    for (const side of [0, 1] as Side[]) {
      const player = duel.players[side];
      if (player.bot) continue;
      try {
        if (e.type === "start") {
          const foe = duel.players[other(side)];
          const intro = foe.bot
            ? `Против тебя бот <b>${escapeHtml(foe.name)}</b>.\n\n`
            : `Дуэль с <b>${escapeHtml(foe.name)}</b> началась. Приказы тайные.\n\n`;
          await showTurn(duel, side, intro);
          continue;
        }
        await retireScreen(duel, side);
        const report = turnReport(duel, side, e.report) + "\n\n";
        if (duel.match.result) {
          const s = finalScreen(duel, side);
          await bot.api.sendMessage(Number(player.id), report + s.text, { ...html, reply_markup: keyboard(s.buttons) });
        } else {
          await showTurn(duel, side, report);
        }
      } catch (err) {
        console.error(`could not message player ${player.id}`, err);
      }
    }
  }

  const duels = new DuelService({ onEvent });

  function who(ctx: Context): { id: string; name: string } | null {
    if (!ctx.from || ctx.chat?.type !== "private") return null;
    return { id: String(ctx.from.id), name: ctx.from.first_name || ctx.from.username || "Оператор" };
  }

  async function askWeapon(ctx: Context, mode: PickMode, lead: string): Promise<void> {
    await ctx.reply(`${lead}\n\nВыбери оружие (в скобках дистанция, где оно сильнее всего):`, { ...html, reply_markup: keyboard(weaponPicker(mode)) });
  }

  bot.command("start", async (ctx) => {
    const me = who(ctx);
    if (!me) return ctx.reply("Напиши мне в личные сообщения.");
    const payload = ctx.match.trim();
    if (payload.startsWith("duel_")) {
      const invite = duels.inviteInfo(payload.slice(5));
      if (!invite) return ctx.reply("Приглашение не найдено или устарело. Попроси друга прислать новое.");
      if (invite.host.id === me.id) return ctx.reply("Это твоё приглашение. Перешли ссылку другу.");
      return askWeapon(ctx, `join:${invite.code}`, `<b>${escapeHtml(invite.host.name)}</b> вызывает тебя на дуэль.`);
    }
    return ctx.reply(WELCOME, { ...html, reply_markup: keyboard(menuButtons()) });
  });

  bot.command("rules", (ctx) => ctx.reply(RULES, html));
  bot.command("help", (ctx) => ctx.reply(WELCOME, { ...html, reply_markup: keyboard(menuButtons()) }));
  bot.command("fight", (ctx) => askWeapon(ctx, "bot", "<b>Бой с ботом.</b>"));
  bot.command("duel", (ctx) => askWeapon(ctx, "duel", "<b>Дуэль с другом.</b> Я дам ссылку, которую надо ему переслать."));

  bot.callbackQuery(/^m:(bot|duel)$/, async (ctx) => {
    await ctx.answerCallbackQuery();
    const mode = ctx.match[1] as "bot" | "duel";
    await askWeapon(ctx, mode, mode === "bot" ? "<b>Бой с ботом.</b>" : "<b>Дуэль с другом.</b> Я дам ссылку, которую надо ему переслать.");
  });

  bot.callbackQuery(/^w:(bot|duel|join:[A-Z0-9]+):([a-z]+)$/, async (ctx) => {
    const me = who(ctx);
    const mode = ctx.match[1]!, weapon = ctx.match[2]!;
    if (!me || !isWeapon(weapon)) return ctx.answerCallbackQuery();
    try {
      if (mode === "bot") {
        duels.startVsBot({ ...me, weapon });
      } else if (mode === "duel") {
        const inv = duels.createInvite({ ...me, weapon });
        const link = `https://t.me/${bot.botInfo.username}?start=duel_${inv.code}`;
        const share = `https://t.me/share/url?url=${encodeURIComponent(link)}&text=${encodeURIComponent("Вызываю тебя на дуэль в Crowned Dogs")}`;
        await ctx.reply(
          `Ссылка на дуэль (действует 30 минут):\n${link}\n\nКак только друг её откроет и выберет оружие, бой начнётся.`,
          { reply_markup: keyboard([[{ text: "Отправить другу", data: "", url: share }]]) },
        );
      } else {
        duels.acceptInvite(mode.slice(5), { ...me, weapon });
      }
      await ctx.answerCallbackQuery();
      await ctx.editMessageReplyMarkup({ reply_markup: new InlineKeyboard() }).catch(() => undefined);
    } catch (err) {
      if (err instanceof DuelError) return ctx.answerCallbackQuery({ text: err.message, show_alert: true });
      throw err;
    }
  });

  bot.callbackQuery(/^t:([a-z0-9]+):(\d+):(.+)$/, async (ctx) => {
    const me = who(ctx);
    if (!me) return ctx.answerCallbackQuery();
    const duelId = ctx.match[1]!, turn = Number(ctx.match[2]), payload = ctx.match[3]!;
    if (payload === "x") {
      return ctx.answerCallbackQuery({ text: "🎯 — куда бить по врагу. 🛡 — какие зоны прикрыть щитом." });
    }
    try {
      const duel = payload === "go"
        ? duels.submit(me.id, duelId, turn)
        : (() => {
            const c = decodeCommand(payload);
            if (!c) throw new DuelError("Неизвестная кнопка");
            return duels.press(me.id, duelId, turn, c);
          })();
      await ctx.answerCallbackQuery(payload === "go" ? { text: "Приказ отдан" } : undefined);
      // If the turn was played right away, the new turn's message is already on its way.
      if (duel.match.turn !== turn || duel.match.result) return;
      const side = duels.sideOf(duel, me.id)!;
      const s = turnScreen(duel, side);
      await ctx.editMessageText(s.text, { ...html, reply_markup: keyboard(s.buttons) }).catch((err) => {
        if (!notModified(err)) throw err;
      });
    } catch (err) {
      if (err instanceof DuelError) return ctx.answerCallbackQuery({ text: err.message });
      throw err;
    }
  });

  bot.on("callback_query:data", (ctx) => ctx.answerCallbackQuery({ text: "Эта кнопка устарела" }));

  bot.catch((err) => console.error("bot error", err.error));
  return { bot, duels };
}
