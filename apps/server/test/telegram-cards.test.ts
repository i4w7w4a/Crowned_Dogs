import assert from "node:assert/strict";
import { test } from "node:test";
import { invitationCard, miniAppUrl, type InvitationCard } from "../src/online/telegram-cards.ts";

const invitation: InvitationCard = {
  token: "abcdef_0123456789-XYZ", status: "open", expiresAt: 1794054600000,
  host: { telegram: { firstName: '<a href="https://evil.test">Имя</a>', lastName: "& Фамилия", username: null }, level: 3, tag: "K-001-007" },
};

test("карточка экранирует имя без username и содержит серверный уровень и жетон", () => {
  const card = invitationCard(invitation, "crowned_dogs_bot");
  assert.ok(!card.text.includes('<a href="https://evil.test">'));
  assert.match(card.text, /&lt;a href="https:\/\/evil.test"&gt;Имя&lt;\/a&gt; &amp; Фамилия/);
  assert.match(card.text, /Уровень: <b>3<\/b>/);
  assert.match(card.text, /K-001-007/);
  assert.match(card.text, /не принимает вызов/);
  assert.equal(card.parse_mode, "HTML");
  assert.deepEqual(card.reply_markup.inline_keyboard, [[{ text: "Открыть вызов", url: "https://t.me/crowned_dogs_bot?startapp=duel_abcdef_0123456789-XYZ" }]]);
});

test("username показывается рядом с именем; отменённая и истёкшая карточки теряют кнопку", () => {
  const named = { ...invitation, host: { ...invitation.host, telegram: { firstName: "Анна", lastName: null, username: "anna_test" } } };
  assert.match(invitationCard(named, "crowned_dogs_bot").text, /Анна.*@anna_test/);
  for (const state of ["cancelled", "expired"] as const) {
    const card = invitationCard({ ...named, status: state }, "crowned_dogs_bot");
    assert.deepEqual(card.reply_markup.inline_keyboard, []);
    assert.match(card.text, state === "cancelled" ? /Вызов отменён/ : /Срок вызова истёк/);
  }
  const accepted = invitationCard({ ...named, status: "accepted" }, "crowned_dogs_bot");
  assert.match(accepted.text, /Вызов принят/);
  assert.equal(accepted.reply_markup.inline_keyboard[0]?.[0]?.text, "Открыть бой");
});

test("deep link не принимает URL или управляющие символы вместо token и имени бота", () => {
  for (const token of ["", "x&startapp=evil", "../../evil", "x\ny", "x".repeat(508)]) {
    assert.throws(() => miniAppUrl("crowned_dogs_bot", token), /Invalid/);
  }
  for (const name of ["x.test/path", "@bot", "abc?start", "bot\nx"]) assert.throws(() => miniAppUrl(name, "abc"), /Invalid/);
});
