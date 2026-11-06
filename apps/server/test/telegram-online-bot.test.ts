import assert from "node:assert/strict";
import { test } from "node:test";
import type { Update, UserFromGetMe } from "grammy/types";
import { createOnlineBot } from "../src/online/telegram-bot.ts";
import type { BrowserLoginPort } from "../src/online/browser-login-types.ts";

const challenge = "Abcdefghijklmnopqrstuvwxyz012345";
const botInfo: UserFromGetMe = {
  id: 123, is_bot: true, first_name: "Crowned Dogs", username: "crowned_dogs_bot", can_join_groups: true,
  can_read_all_group_messages: false, supports_inline_queries: false, can_connect_to_business: false,
  has_main_web_app: true, has_topics_enabled: false, allows_users_to_create_topics: false,
  can_manage_bots: false, supports_join_request_queries: false,
};
function setup(port: BrowserLoginPort) {
  const bot = createOnlineBot("123:test", { browserLogin: port, botUsername: "crowned_dogs_bot", webAppUrl: "https://example.test/r2/", botInfo, now: () => 1000 });
  const calls: { method: string; payload: Record<string, any> }[] = [];
  bot.api.config.use(async (_prev, method, payload) => {
    calls.push({ method, payload: payload as Record<string, any> });
    return { ok: true, result: method === "sendMessage" ? { message_id: 91, date: 0, chat: { id: 42, type: "private" }, text: "" } : true } as never;
  });
  let updateId = 0;
  const user = { id: 42, is_bot: false, first_name: "Анна", last_name: "&Имя", username: "anna_test" };
  const send = (text: string, group = false) => bot.handleUpdate({ update_id: ++updateId, message: { message_id: 1, date: 0,
    chat: group ? { id: -1001, type: "supergroup", title: "Group" } : { id: 42, type: "private", first_name: "Анна" }, from: user, text,
    entities: [{ type: "bot_command", offset: 0, length: text.split(" ")[0]!.length }],
  } } as Update);
  const tap = (data: string, group = false) => bot.handleUpdate({ update_id: ++updateId, callback_query: { id: String(updateId), from: user, chat_instance: "x", data,
    message: { message_id: 91, date: 0, chat: group ? { id: -1001, type: "supergroup", title: "Group" } : { id: 42, type: "private", first_name: "Анна" }, text: "" },
  } } as Update);
  return { bot, calls, send, tap };
}
const available: BrowserLoginPort = { async describe() { return { origin: "https://example.test", expiresAt: 301000 }; }, async confirm() { return "confirmed"; } };

test("/start login показывает домен и явное подтверждение без авторизации", async () => {
  let confirms = 0;
  const { send, calls } = setup({ ...available, async confirm() { confirms++; return "confirmed"; } });
  await send(`/start login_${challenge}`);
  assert.equal(confirms, 0); const message = calls.find(c => c.method === "sendMessage")!;
  assert.match(message?.payload.text, /https:\/\/example.test/);
  assert.match(message?.payload.text, /только если.*сам/i);
  assert.equal(message.payload.reply_markup.inline_keyboard[0][0].callback_data, `login:${challenge}`);
});

test("только callback личного чата передаёт Telegram identity доверенному порту", async () => {
  const confirmations: unknown[] = [];
  const { tap, calls } = setup({ ...available, async confirm(...args) { confirmations.push(args); return "confirmed"; } });
  await tap(`login:${challenge}`, true); assert.deepEqual(confirmations, []);
  await tap(`login:${challenge}`);
  assert.deepEqual(confirmations, [[challenge, { id: "42", firstName: "Анна", lastName: "&Имя", username: "anna_test" }, 1000]]);
  assert.match(String(calls.findLast(c => c.method === "editMessageText")?.payload.text), /Вернись в браузер/);
  assert.deepEqual(calls.findLast(c => c.method === "editMessageText")?.payload.reply_markup, { inline_keyboard: [] });
});

test("истёкшая или чужая подтверждённая заявка не обещает успешный вход", async () => {
  for (const result of ["expired", "already_confirmed"] as const) {
    const { tap, calls } = setup({ ...available, async confirm() { return result; } });
    await tap(`login:${challenge}`);
    const text = String(calls.findLast(c => c.method === "editMessageText")?.payload.text);
    assert.doesNotMatch(text, /Вход подтверждён/); assert.match(text, /Начни вход заново/);
  }
  const { send, calls } = setup({ ...available, async describe() { return null; } });
  await send(`/start login_${challenge}`); assert.ok(!JSON.stringify(calls).includes('"callback_data":"login:'));
});

test("групповой или неверный login payload не подтверждает аккаунт", async () => {
  let reads = 0, confirms = 0;
  const { send, tap } = setup({ async describe() { reads++; return null; }, async confirm() { confirms++; return "confirmed"; } });
  await send(`/start login_${challenge}`, true); await send("/start login_bad"); await tap("login:bad");
  assert.equal(reads, 0); assert.equal(confirms, 0);
});

test("дуэль и старые боевые кнопки открывают Mini App без создания второго PvP", async () => {
  const { send, tap, calls } = setup(available);
  await send("/start duel_same_token");
  const invite = calls.findLast(c => c.method === "sendMessage")!;
  assert.equal(invite.payload.reply_markup.inline_keyboard[0][0].url, "https://t.me/crowned_dogs_bot?startapp=duel_same_token");
  for (const command of ["/duel", "/fight", "/me"]) await send(command);
  await tap("w:duel:rifle");
  assert.match(String(calls.findLast(c => c.method === "sendMessage")?.payload.text), /3D/);
  assert.equal(calls.findLast(c => c.method === "sendMessage")?.payload.reply_markup.inline_keyboard[0][0].url, "https://t.me/crowned_dogs_bot?startapp");
});
