import assert from "node:assert/strict";
import test from "node:test";
import { OnlineError, OnlineTransport } from "../src/online/transport.ts";
import { isLoginExpired, onlineErrorMessage } from "../src/online/errors.ts";

test("confirmed login expiring during consume offers a new login instead of a server outage", async () => {
  const api=new OnlineTransport("/api/online",async()=>Response.json({error:"browser_login_expired"},{status:410}),
    {getItem:()=>null,setItem:()=>{},removeItem:()=>{}});
  let failure: unknown;
  try { await api.post("/auth/browser/consume",{}); } catch(error) { failure=error; }
  assert.match(onlineErrorMessage(failure),/Время подтверждения истекло/);
  assert.match(onlineErrorMessage(failure),/Начните вход заново/);
  assert.equal(isLoginExpired(failure),true,"the UI offers the restart-login action");
  assert.equal(onlineErrorMessage(failure),onlineErrorMessage(new OnlineError(410,"login_expired")));
});
