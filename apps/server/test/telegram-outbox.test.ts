import assert from "node:assert/strict";
import { test } from "node:test";
import { TelegramOutboxWorker, type TelegramDelivery, type TelegramOutboxPort } from "../src/online/telegram-outbox.ts";
import { TelegramFailure } from "../src/online/telegram-transport.ts";

const delivery = (): TelegramDelivery => ({
  leaseId: "lease-1", kind: "publish", attempts: 1, chatId: "-1001", threadId: 566, messageId: null,
  card: { token: "same_token", status: "open", expiresAt: 1000000,
    host: { telegram: { firstName: "Имя", lastName: null, username: null }, level: 1, tag: "K-001-001" } },
});
function queue(item: TelegramDelivery | null) {
  const completed: unknown[] = [], failed: unknown[] = [];
  const outbox: TelegramOutboxPort = {
    async claimDue() { const next = item; item = null; return next; },
    async complete(...args) { completed.push(args); return true; },
    async fail(...args) { failed.push(args); return true; },
  };
  return { outbox, completed, failed };
}

test("claim отправляется и только успешный Telegram ответ отмечает нужный lease", async () => {
  const q = queue(delivery()); const sent: unknown[] = [];
  const worker = new TelegramOutboxWorker({ outbox: q.outbox, transport: { async deliver(...args) { sent.push(args); return 91; } }, botUsername: "test_bot", now: () => 1000 });
  assert.equal(await worker.runOnce(), true); assert.equal(await worker.runOnce(), false);
  assert.equal(sent.length, 1); assert.match(JSON.stringify(sent), /duel_same_token/);
  assert.equal((sent[0] as unknown[])[3], 566, "worker keeps the configured topic on every send");
  assert.deepEqual(q.completed, [["lease-1", 91]]); assert.deepEqual(q.failed, []);
});

test("повтор edit сохраняет message id; closed до первой публикации не отправляется", async () => {
  for (const status of ["cancelled", "expired"] as const) {
    const item = delivery(); item.card.status = status;
    const q = queue(item);
    const worker = new TelegramOutboxWorker({ outbox: q.outbox, transport: { async deliver() { assert.fail("closed unpublished invitation sent"); } }, botUsername: "test_bot" });
    assert.equal(await worker.runOnce(), true); assert.deepEqual(q.completed, [["lease-1", null]]);
  }
  const item = delivery(); item.kind = "update"; item.messageId = 41; item.card.status = "accepted";
  const q = queue(item); let edited: number | null = null;
  const worker = new TelegramOutboxWorker({ outbox: q.outbox, transport: { async deliver(_chat, messageId) { edited = messageId; return 41; } }, botUsername: "test_bot" });
  await worker.runOnce(); assert.equal(edited, 41); assert.deepEqual(q.completed, [["lease-1", 41]]);
});

test("принятый до первой публикации вызов завершает outbox без отправки", async () => {
  const item = delivery(); item.card.status = "accepted";
  const q = queue(item); let sends = 0;
  const worker = new TelegramOutboxWorker({ outbox: q.outbox, transport: { async deliver() { sends++; return 91; } }, botUsername: "test_bot" });
  assert.equal(await worker.runOnce(), true);
  assert.equal(sends, 0, "accepted unpublished invitation must not create a Telegram post");
  assert.deepEqual(q.completed, [["lease-1", null]]); assert.deepEqual(q.failed, []);
});

test("временная ошибка и 429 назначают retry; постоянный запрет требует ручного действия", async () => {
  for (const [error, retryAt] of [[new TelegramFailure("telegram_rate_limited", 19000), 20000], [new TelegramFailure("telegram_unavailable", 1000), 2000], [new TelegramFailure("bot_forbidden"), null]] as const) {
    const q = queue(delivery());
    const worker = new TelegramOutboxWorker({ outbox: q.outbox, transport: { async deliver() { throw error; } }, botUsername: "test_bot", now: () => 1000 });
    assert.equal(await worker.runOnce(), true); assert.deepEqual(q.failed, [["lease-1", { code: error.code, retryAt }]]); assert.deepEqual(q.completed, []);
  }
});

test("ошибка записи после send не превращается в новый fail или повторную отправку в том же вызове", async () => {
  const q = queue(delivery()); q.outbox.complete = async () => { throw new Error("db down"); };
  let sends = 0;
  const worker = new TelegramOutboxWorker({ outbox: q.outbox, transport: { async deliver() { sends++; return 91; } }, botUsername: "test_bot" });
  await assert.rejects(worker.runOnce(), /db down/); assert.equal(sends, 1); assert.deepEqual(q.failed, []);
});

test("две одновременные итерации одного worker не отправляют вторую карточку параллельно", async () => {
  let release!: () => void; const held = new Promise<void>(resolve => { release = resolve; });
  let claims = 0;
  const q = queue(null); q.outbox.claimDue = async () => { claims++; return delivery(); };
  const worker = new TelegramOutboxWorker({ outbox: q.outbox, transport: { async deliver() { await held; return 91; } }, botUsername: "test_bot" });
  const first = worker.runOnce();
  assert.equal(await worker.runOnce(), false); assert.equal(claims, 1); release(); assert.equal(await first, true);
});

test("shutdown во время claim запрещает send и следующие итерации", async () => {
  let release!: (item: TelegramDelivery) => void;
  const q = queue(null); q.outbox.claimDue = () => new Promise(resolve => { release = resolve; });
  let sends = 0;
  const worker = new TelegramOutboxWorker({ outbox: q.outbox, transport: { async deliver() { sends++; return 91; } }, botUsername: "test_bot" });
  const running = worker.runOnce(); const stopping = worker.stop();
  release(delivery()); await running; await stopping;
  assert.equal(sends, 0); assert.equal(await worker.runOnce(), false); assert.deepEqual(q.completed, []); assert.deepEqual(q.failed, []);
});

test("shutdown прерывает транспорт и оставляет lease для восстановления", async () => {
  const q = queue(delivery()); let entered!: () => void;
  const ready = new Promise<void>(resolve => { entered = resolve; });
  const worker = new TelegramOutboxWorker({ outbox: q.outbox, transport: { async deliver(_chat, _id, _card, _threadId, signal) {
    entered(); await new Promise<void>((_resolve, reject) => signal!.addEventListener("abort", () => reject(new Error("aborted")), { once: true })); return 91;
  } }, botUsername: "test_bot" });
  const running = worker.runOnce(); await ready; await worker.stop(); await running;
  assert.deepEqual(q.completed, []); assert.deepEqual(q.failed, []);
});
