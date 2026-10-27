import assert from "node:assert/strict";
import { test } from "node:test";
import type { Update, UserFromGetMe } from "grammy/types";
import { createBot } from "../src/bot.ts";

const BOT_INFO: UserFromGetMe = {
  id: 1, is_bot: true, first_name: "Crowned Dogs", username: "crowned_dogs_bot",
  can_join_groups: false, can_read_all_group_messages: false, supports_inline_queries: false,
  can_connect_to_business: false, has_main_web_app: false, has_topics_enabled: false,
  allows_users_to_create_topics: false, can_manage_bots: false, supports_join_request_queries: false,
};

// The whole Telegram flow without the network: updates go in, API calls are recorded.
test("from /start to a played turn against the bot", async () => {
  const { bot, duels } = createBot("1:test", { botInfo: BOT_INFO });
  const calls: { method: string; payload: Record<string, unknown> }[] = [];
  let messageId = 10;
  bot.api.config.use(async (_prev, method, payload) => {
    calls.push({ method, payload: payload as Record<string, unknown> });
    const result = method === "sendMessage"
      ? { message_id: messageId++, date: 0, chat: { id: 100, type: "private", first_name: "Аня" }, text: "" }
      : true;
    return { ok: true, result } as never;
  });

  const user = { id: 100, is_bot: false, first_name: "Аня" };
  const chat = { id: 100, type: "private" as const, first_name: "Аня" };
  let updateId = 1;
  const send = (text: string) => bot.handleUpdate({
    update_id: updateId++,
    message: { message_id: 1, date: 0, chat, from: user, text, entities: text.startsWith("/") ? [{ type: "bot_command", offset: 0, length: text.split(" ")[0]!.length }] : [] },
  } as Update);
  const tap = (data: string) => bot.handleUpdate({
    update_id: updateId++,
    callback_query: { id: String(updateId), from: user, chat_instance: "x", data, message: { message_id: messageId - 1, date: 0, chat, text: "" } },
  } as Update);
  const tick = () => new Promise((r) => setTimeout(r, 10));

  try {
    await send("/start");
    assert.match(String(calls.at(-1)?.payload.text), /Контракт оператора/, "a new player sees the contract first");
    await send("/fight");
    assert.match(String(calls.at(-1)?.payload.text), /Контракт оператора/, "no fight before signing");
    assert.match(JSON.stringify(calls.at(-1)?.payload.reply_markup), /"callback_data":"c:bot"/);

    await tap("c");
    const signed = calls.filter((c) => c.method === "sendMessage").at(-2)!;
    assert.match(String(signed.payload.text), /Твой жетон: <b>K-\d{3}-\d{3}<\/b>/);
    assert.match(String(calls.at(-1)?.payload.text), /Crowned Dogs/);

    await send("/me");
    assert.match(String(calls.at(-1)?.payload.text), /Жетон <b>K-\d{3}-\d{3}<\/b>, позывной <b>Аня<\/b>/);

    await tap("m:bot");
    await tap("w:bot:rifle");
    await tick();
    const duel = duels.duelOf("100");
    assert.ok(duel, "the fight has started");
    const screen = calls.filter((c) => c.method === "sendMessage").at(-1)!;
    assert.match(String(screen.payload.text), /Против тебя бот/);
    assert.match(String(screen.payload.text), /Ход 1 из/);

    for (const code of ["zb", "gh", "gb"]) await tap(`t:${duel.id}:1:${code}`);
    const edit = calls.filter((c) => c.method === "editMessageText").at(-1)!;
    assert.match(String(edit.payload.text), /Атака: выстрел в корпус/);

    await tap(`t:${duel.id}:1:go`);
    await tick();
    const report = calls.filter((c) => c.method === "sendMessage").at(-1)!;
    assert.match(String(report.payload.text), /<b>Ход 1<\/b>/);
    assert.ok(duel.match.turn === 2 || duel.match.result, "the turn was played");

    await tap(`t:${duel.id}:1:zh`);
    const stale = calls.filter((c) => c.method === "answerCallbackQuery").at(-1)!;
    // A random bot weapon can end the fight in turn 1, so the old button
    // is refused either as a past turn or as a finished fight.
    assert.equal(stale.payload.text, duel.match.result ? "Этот бой уже закончился" : "Этот ход уже прошёл");
  } finally {
    duels.close();
  }
});

test("an invite link starts a duel between two players", async () => {
  const { bot, duels } = createBot("1:test", { botInfo: BOT_INFO });
  const calls: { method: string; payload: Record<string, unknown> }[] = [];
  bot.api.config.use(async (_prev, method, payload) => {
    calls.push({ method, payload: payload as Record<string, unknown> });
    return { ok: true, result: method === "sendMessage" ? { message_id: calls.length, date: 0, chat: { id: 1, type: "private" } } : true } as never;
  });
  let updateId = 1;
  const as = (id: number, name: string) => {
    const user = { id, is_bot: false, first_name: name };
    const chat = { id, type: "private" as const, first_name: name };
    return {
      send: (text: string) => bot.handleUpdate({ update_id: updateId++, message: { message_id: 1, date: 0, chat, from: user, text,
        entities: [{ type: "bot_command", offset: 0, length: text.split(" ")[0]!.length }] } } as Update),
      tap: (data: string) => bot.handleUpdate({ update_id: updateId++, callback_query: { id: String(updateId), from: user, chat_instance: "x", data,
        message: { message_id: 1, date: 0, chat, text: "" } } } as Update),
    };
  };
  const ann = as(100, "Аня"), bob = as(200, "Боб");
  try {
    await ann.tap("c:duel");
    const tag = /K-\d{3}-\d{3}/.exec(String(calls.find((c) => /Твой жетон/.test(String(c.payload.text)))?.payload.text))?.[0];
    assert.ok(tag, "Аня got a tag");
    assert.match(String(calls.at(-1)?.payload.text), /Дуэль с другом/, "signing carries on with the duel");
    await ann.tap("w:duel:sniper");
    const linkMsg = calls.filter((c) => c.method === "sendMessage").at(-1)!;
    const code = /start=duel_([A-Z0-9]+)/.exec(String(linkMsg.payload.text))?.[1];
    assert.ok(code, "the reply holds an invite link");
    await bob.send(`/start duel_${code}`);
    assert.match(String(calls.at(-1)?.payload.text), /Контракт оператора/, "Боб signs before he can accept");
    await bob.tap(`c:duel_${code}`);
    assert.match(String(calls.at(-1)?.payload.text), new RegExp(`Аня</b> \\(${tag}\\) вызывает тебя на дуэль`));
    await bob.tap(`w:join:${code}:shotgun`);
    await new Promise((r) => setTimeout(r, 10));
    const starts = calls.filter((c) => c.method === "sendMessage" && /Дуэль с .* началась/.test(String(c.payload.text)));
    assert.deepEqual(starts.map((c) => c.payload.chat_id).sort(), [100, 200], "both players get their own screen");
    const bobSees = String(starts.find((c) => c.payload.chat_id === 200)?.payload.text);
    assert.ok(bobSees.includes(`Дуэль с <b>Аня</b> (${tag}) началась`), "the foe is shown with the tag");
  } finally {
    duels.close();
  }
});
