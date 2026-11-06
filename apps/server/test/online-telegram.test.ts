import assert from "node:assert/strict";
import { test } from "node:test";
import { verifyTelegram } from "../src/online/telegram.ts";
import { botToken as token, clock as now, initData as signed } from "./online-fixtures.ts";

test("Telegram verifier authenticates decoded signed fields and enforces trust boundaries", () => {
  const user = verifyTelegram(signed({ signature: "signed-extra", query_id: "a+b & c" }), token, now);
  assert.deepEqual(user, { id: "123", firstName: "Арт + &", lastName: null, username: null });
  assert.equal(verifyTelegram(signed({ auth_date: String(now / 1000 - 300) }), token, now).id, "123");
  assert.equal(verifyTelegram(signed({ auth_date: String(now / 1000 + 30) }), token, now).id, "123");
  for (const raw of [
    signed().replace("123", "456"), signed({}, "wrong-token"),
    signed({ auth_date: String(now / 1000 - 301) }), signed({ auth_date: String(now / 1000 + 31) }),
    signed() + "&auth_date=1", signed() + "&%75ser=%7B%7D", signed() + "&hash=00",
    signed({ user: "{" }), signed({ user: "null" }), signed({ user: "[]" }),
    signed({ user: JSON.stringify({ id: 0, first_name: "x" }) }),
    signed({ user: JSON.stringify({ id: -1, first_name: "x" }) }),
    signed({ user: JSON.stringify({ id: 9007199254740992, first_name: "x" }) }),
    signed({ user: JSON.stringify({ id: "123", first_name: "x" }) }),
    signed({ auth_date: "NaN" }), signed({ auth_date: "1.5" }), "", "user=%ZZ", signed() + "&bad=%FF",
    signed().replace(/hash=[a-f0-9]+/, "hash=xyz"), signed().replace(/auth_date=[^&]+&/, ""),
    signed().replace(/user=[^&]+&/, ""), signed({ signature: "value" }).replace("value", "changed"),
  ]) assert.throws(() => verifyTelegram(raw, token, now), /invalid telegram/i, raw.slice(0, 50));
});
