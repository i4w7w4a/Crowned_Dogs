import assert from "node:assert/strict";
import { createServer } from "node:http";
import { test, type TestContext } from "node:test";
import { TelegramTransport, TelegramFailure } from "../src/online/telegram-transport.ts";

const card = { text: "<b>Вызов</b>", parse_mode: "HTML" as const, reply_markup: { inline_keyboard: [[{ text: "Открыть", url: "https://t.me/test_bot?startapp=duel_abc" }]] } };
type Call = { method: string; body: Record<string, any> };
async function api(t: TestContext, override?: (call: Call) => unknown) {
  const calls: Call[] = [];
  const server = createServer(async (req, res) => {
    const chunks = []; for await (const chunk of req) chunks.push(chunk);
    const call = { method: req.url!.split("/").at(-1)!, body: JSON.parse(Buffer.concat(chunks).toString()) };
    calls.push(call);
    const custom = await override?.(call);
    const result = call.method === "getChat" ? { id: -1001, type: "supergroup", is_forum: true, permissions: { can_send_messages: true } }
      : call.method === "getChatMember" ? { status: "member", user: { id: 123, is_bot: true, first_name: "Bot" } }
      : { message_id: 91, chat: { id: -1001, type: "supergroup" }, date: 0, text: card.text };
    res.setHeader("content-type", "application/json");
    res.end(JSON.stringify(custom ?? { ok: true, result }));
  });
  await new Promise<void>(resolve => server.listen(0, "127.0.0.1", resolve));
  t.after(() => { server.closeAllConnections(); return new Promise<void>((resolve, reject) => server.close(e => e ? reject(e) : resolve())); });
  const address = server.address(); assert.ok(address && typeof address === "object");
  return { calls, transport: new TelegramTransport({ token: "123:test-secret", allowedChatIds: ["-1001"], apiRoot: `http://127.0.0.1:${address.port}` }) };
}

test("разрешённая группа проверяется до отправки и получает URL-кнопку с сохранением message id", async t => {
  const { calls, transport } = await api(t);
  assert.equal(await transport.deliver("-1001", null, card, 566), 91);
  assert.deepEqual(calls.map(c => c.method), ["getChat", "getChatMember", "sendMessage"]);
  assert.deepEqual(calls[1]?.body, { chat_id: "-1001", user_id: 123 });
  assert.deepEqual(calls[2]?.body, { chat_id: "-1001", message_thread_id: 566, ...card, link_preview_options: { is_disabled: true } });
});

test("чужая группа не вызывает Telegram API", async t => {
  const { calls, transport } = await api(t);
  await assert.rejects(transport.deliver("-999", null, card, 566), (e: unknown) => e instanceof TelegramFailure && e.code === "chat_not_allowed");
  assert.equal(calls.length, 0);
});

test("удалённый или ограниченный бот и закрытая группа не публикуют карточку", async t => {
  for (const member of [{ status: "left" }, { status: "kicked" }, { status: "restricted", is_member: true, can_send_messages: false }]) {
    const { calls, transport } = await api(t, c => c.method === "getChatMember" ? { ok: true, result: member } : undefined);
    await assert.rejects(transport.deliver("-1001", null, card, 566), (e: unknown) => e instanceof TelegramFailure && e.code === "bot_forbidden");
    assert.ok(!calls.some(c => c.method === "sendMessage"));
  }
  const { calls, transport } = await api(t, c => c.method === "getChat" ? { ok: true, result: { id: -1001, type: "supergroup", is_forum: true, permissions: { can_send_messages: false } } } : undefined);
  await assert.rejects(transport.deliver("-1001", null, card, 566), /bot_forbidden/);
  assert.ok(!calls.some(c => c.method === "sendMessage"));
});

test("update редактирует прежнее сообщение; message is not modified считается успехом", async t => {
  const { calls, transport } = await api(t, c => c.method === "editMessageText" ? { ok: false, error_code: 400, description: "Bad Request: message is not modified" } : undefined);
  assert.equal(await transport.deliver("-1001", 55, { ...card, reply_markup: { inline_keyboard: [] } }, 566), 55);
  assert.deepEqual(calls.at(-1), { method: "editMessageText", body: { chat_id: "-1001", message_id: 55, ...card, reply_markup: { inline_keyboard: [] }, link_preview_options: { is_disabled: true } } });
  assert.ok(!calls.some(c => c.method === "sendMessage"));
});

test("429 сохраняет retry_after, ошибки возвращаются без Telegram description или token", async t => {
  for (const [status, code, retry] of [[429, "telegram_rate_limited", 19000], [500, "telegram_unavailable", 1000], [403, "bot_forbidden", null], [400, "telegram_bad_request", null]] as const) {
    const { transport } = await api(t, c => c.method === "sendMessage" ? { ok: false, error_code: status, description: "123:test-secret PRIVATE", parameters: { retry_after: 19 } } : undefined);
    await assert.rejects(transport.deliver("-1001", null, card, 566), (e: unknown) => {
      assert.ok(e instanceof TelegramFailure); assert.equal(e.code, code); assert.equal(e.retryAfterMs, retry);
      assert.ok(!String(e).includes("test-secret")); return true;
    });
  }
});

test("неполный успешный ответ нельзя отметить доставленным", async t => {
  const { transport } = await api(t, c => c.method === "sendMessage" ? { ok: true, result: true } : undefined);
  await assert.rejects(transport.deliver("-1001", null, card, 566), /telegram_unavailable/);
});

test("долгий retry_after нельзя сократить и повторить раньше срока Telegram", async t => {
  const { transport } = await api(t, c => c.method === "sendMessage" ? { ok: false, error_code: 429, description: "Too Many Requests", parameters: { retry_after: 90000 } } : undefined);
  await assert.rejects(transport.deliver("-1001", null, card, 566), (error: unknown) => error instanceof TelegramFailure && error.retryAfterMs === 90000000);
});

test("отмена во время проверки прав не отправляет карточку", async t => {
  let arrived!: () => void, release!: (value: unknown) => void;
  const seen = new Promise<void>(resolve => { arrived = resolve; });
  const { calls, transport } = await api(t, c => c.method === "getChatMember" ? new Promise(resolve => { release = resolve; arrived(); }) : undefined);
  const abort = new AbortController();
  const sending = transport.deliver("-1001", null, card, 566, abort.signal);
  await seen; abort.abort();
  release({ ok: true, result: { status: "administrator" } });
  await assert.rejects(sending, /telegram_unavailable/);
  assert.ok(!calls.some(c => c.method === "sendMessage"));
});

test("публикация идёт строго в настроенную тему; редактирование использует прежний message id", async t => {
  const { calls, transport } = await api(t);
  assert.equal(await transport.deliver("-1001", null, card, 566), 91);
  assert.equal(calls.find(c => c.method === "sendMessage")?.body.message_thread_id, 566);
  assert.equal(await transport.deliver("-1001", 91, card, 566), 91);
  const edit = calls.find(c => c.method === "editMessageText")!;
  assert.equal(edit.body.message_id, 91); assert.equal(edit.body.message_thread_id, undefined);
});

test("закрытая/удалённая тема даёт постоянную ошибку без fallback в общий чат", async t => {
  for (const description of ["Bad Request: TOPIC_CLOSED", "Bad Request: message thread not found", "Bad Request: TOPIC_DELETED"]) {
    const { calls, transport } = await api(t, c => c.method === "sendMessage" ? { ok: false, error_code: 400, description } : undefined);
    await assert.rejects(transport.deliver("-1001", null, card, 566), (e: unknown) => e instanceof TelegramFailure && e.code === "telegram_topic_unavailable" && e.retryAfterMs === null);
    assert.equal(calls.filter(c => c.method === "sendMessage").length, 1);
    assert.equal(calls.at(-1)?.body.message_thread_id, 566);
  }
});

test("тема в чате без форума и невалидный thread id не отправляются", async t => {
  const { calls, transport } = await api(t, c => c.method === "getChat" ? { ok: true, result: { id: -1001, type: "supergroup", permissions: { can_send_messages: true } } } : undefined);
  await assert.rejects(transport.deliver("-1001", null, card, 566), /telegram_topic_unavailable/);
  for (const threadId of [0, -1, 1.5, Number.NaN]) await assert.rejects(transport.deliver("-1001", null, card, threadId), /telegram_bad_request/);
  assert.ok(!calls.some(c => c.method === "sendMessage"));
});
