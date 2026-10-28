// Telegram front end. Only wiring: rules live in @crowned-dogs/core,
// fights in DuelService, texts and buttons in view.ts.

import { Bot, GrammyError, InlineKeyboard, type Context } from "grammy";
import type { InlineKeyboardButton, UserFromGetMe } from "grammy/types";
import { VETERAN_KITS, WEAPON_IDS, other, type Side, type WeaponId } from "@crowned-dogs/core";
import { decodeCommand } from "./codes.ts";
import { DuelError, DuelService, type Duel, type DuelEvent } from "./duels.ts";
import { OperatorRegistry, type Operator } from "./operators.ts";
import {
  CONTRACT, RULES, WELCOME, contractButtons, finalScreen, menuButtons, nameWithTag, signedText, tagText,
  turnReport, turnScreen, VETERAN_NOTE, weaponPicker, type AfterContract, type Button, type PickMode,
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

export function createBot(
  token: string,
  opts: { botInfo?: UserFromGetMe; operators?: OperatorRegistry } = {},
): { bot: Bot; duels: DuelService; operators: OperatorRegistry } {
  const bot = new Bot(token, opts.botInfo ? { botInfo: opts.botInfo } : {});
  const operators = opts.operators ?? new OperatorRegistry();
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
            ? `Против тебя бот ${nameWithTag(foe)}.\n\n`
            : `Дуэль с ${nameWithTag(foe)} началась. Приказы тайные.\n\n`;
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

  /** The player behind a private chat, with the tag once they have signed the contract. */
  function who(ctx: Context): { id: string; name: string; tag?: string } | null {
    if (!ctx.from || ctx.chat?.type !== "private") return null;
    const id = String(ctx.from.id);
    return { id, name: ctx.from.first_name || ctx.from.username || "Оператор", tag: operators.byTelegram(id)?.tag };
  }

  /** Nothing happens before the contract is signed. True if the contract was shown instead. */
  async function needsContract(ctx: Context, me: { tag?: string }, next: AfterContract = ""): Promise<boolean> {
    if (me.tag) return false;
    if (ctx.callbackQuery) await ctx.answerCallbackQuery();
    await ctx.reply(CONTRACT, { ...html, reply_markup: keyboard(contractButtons(next)) });
    return true;
  }

  async function askWeapon(ctx: Context, mode: PickMode, lead: string): Promise<void> {
    await ctx.reply(`${lead}\n\nВыбери оружие (в скобках дистанция, где оно сильнее всего):`, { ...html, reply_markup: keyboard(weaponPicker(mode)) });
  }

  const LEADS = {
    bot: "<b>Бой с ботом.</b>",
    duel: "<b>Дуэль с другом.</b> Я дам ссылку, которую надо ему переслать.",
  };

  async function openInvite(ctx: Context, me: { id: string }, code: string): Promise<unknown> {
    const invite = duels.inviteInfo(code);
    if (!invite) return ctx.reply("Приглашение не найдено или устарело. Попроси друга прислать новое.");
    if (invite.host.id === me.id) return ctx.reply("Это твоё приглашение. Перешли ссылку другу.");
    return askWeapon(ctx, `join:${invite.code}`, `${nameWithTag(invite.host)} вызывает тебя на дуэль.`);
  }

  bot.command("start", async (ctx) => {
    const me = who(ctx);
    if (!me) return ctx.reply("Напиши мне в личные сообщения.");
    const payload = ctx.match.trim();
    const invite = /^duel_([A-Z0-9]+)$/.exec(payload)?.[1];
    if (await needsContract(ctx, me, invite ? `duel_${invite}` : "")) return;
    if (invite) return openInvite(ctx, me, invite);
    return ctx.reply(WELCOME, { ...html, reply_markup: keyboard(menuButtons()) });
  });

  bot.callbackQuery(/^c(?::(bot|duel|duel_[A-Z0-9]+))?$/, async (ctx) => {
    const me = who(ctx);
    if (!me) return ctx.answerCallbackQuery();
    const fresh = !me.tag;
    const op: Operator = operators.sign(me.id, me.name);
    await ctx.answerCallbackQuery(fresh ? { text: `Жетон ${op.tag}` } : undefined);
    await ctx.editMessageReplyMarkup({ reply_markup: new InlineKeyboard() }).catch(() => undefined);
    if (fresh) await ctx.reply(signedText(op.tag), html);
    const next = ctx.match[1] ?? "";
    if (next.startsWith("duel_")) return openInvite(ctx, me, next.slice(5));
    if (next === "bot" || next === "duel") return askWeapon(ctx, next, LEADS[next]);
    return ctx.reply(WELCOME, { ...html, reply_markup: keyboard(menuButtons()) });
  });

  bot.command("rules", (ctx) => ctx.reply(RULES, html));
  bot.command("help", (ctx) => ctx.reply(WELCOME, { ...html, reply_markup: keyboard(menuButtons()) }));
  bot.command("me", async (ctx) => {
    const me = who(ctx);
    if (!me) return ctx.reply("Напиши мне в личные сообщения.");
    if (await needsContract(ctx, me)) return;
    return ctx.reply(tagText(me.tag!, operators.byTelegram(me.id)!.name), html);
  });
  for (const mode of ["bot", "duel"] as const) {
    bot.command(mode === "bot" ? "fight" : "duel", async (ctx) => {
      const me = who(ctx);
      if (!me) return ctx.reply("Напиши мне в личные сообщения.");
      if (await needsContract(ctx, me, mode)) return;
      return askWeapon(ctx, mode, LEADS[mode]);
    });
  }

  bot.callbackQuery(/^m:(bot|duel)$/, async (ctx) => {
    const me = who(ctx);
    const mode = ctx.match[1] as "bot" | "duel";
    if (!me) return ctx.answerCallbackQuery();
    if (await needsContract(ctx, me, mode)) return;
    await ctx.answerCallbackQuery();
    await askWeapon(ctx, mode, LEADS[mode]);
  });

  bot.callbackQuery(/^wk:(bot|duel|join:[A-Z0-9]+):(v|s)$/, async (ctx) => {
    const mode = ctx.match[1] as PickMode, veteran = ctx.match[2] === "v";
    await ctx.answerCallbackQuery(veteran ? { text: VETERAN_NOTE, show_alert: true } : undefined);
    await ctx.editMessageReplyMarkup({ reply_markup: keyboard(weaponPicker(mode, veteran)) }).catch(() => undefined);
  });

  bot.callbackQuery(/^w:(bot|duel|join:[A-Z0-9]+):([a-z]+)(:v)?$/, async (ctx) => {
    const me = who(ctx);
    const mode = ctx.match[1]!, weapon = ctx.match[2]!;
    if (!me || !isWeapon(weapon)) return ctx.answerCallbackQuery();
    const kit = ctx.match[3] ? { kit: VETERAN_KITS[weapon] } : {};
    if (await needsContract(ctx, me, mode.startsWith("join:") ? `duel_${mode.slice(5)}` : mode as "bot" | "duel")) return;
    try {
      if (mode === "bot") {
        duels.startVsBot({ ...me, weapon, ...kit });
      } else if (mode === "duel") {
        const inv = duels.createInvite({ ...me, weapon, ...kit });
        const link = `https://t.me/${bot.botInfo.username}?start=duel_${inv.code}`;
        const share = `https://t.me/share/url?url=${encodeURIComponent(link)}&text=${encodeURIComponent("Вызываю тебя на дуэль в Crowned Dogs")}`;
        await ctx.reply(
          `Ссылка на дуэль (действует 30 минут):\n${link}\n\nКак только друг её откроет и выберет оружие, бой начнётся.`,
          { reply_markup: keyboard([[{ text: "Отправить другу", data: "", url: share }]]) },
        );
      } else {
        duels.acceptInvite(mode.slice(5), { ...me, weapon, ...kit });
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
  return { bot, duels, operators };
}
