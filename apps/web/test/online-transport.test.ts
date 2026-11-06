import assert from "node:assert/strict";
import test from "node:test";
import { OnlineTransport, OnlineError } from "../src/online/transport.ts";
const storage = () => {
  const data = new Map<string,string>();
  return { getItem: (k: string) => data.get(k) ?? null, setItem: (k: string,v: string) => { data.set(k,v); }, removeItem: (k: string) => { data.delete(k); } };
};
test("a lost command survives reload and reuses the exact receipt body", async () => {
  const store = storage(), calls: {url:string,init?:RequestInit}[] = [];
  let failed = true;
  const send: typeof fetch = async (url, init) => {
    calls.push({ url:String(url), init });
    if (failed) { failed = false; throw new TypeError("Connection lost after commit"); }
    return Response.json({battle:{version:8}});
  };
  const api = new OnlineTransport("/r2/api/online", send, store);
  await assert.rejects(api.command("/matches/a/commands", { expectedVersion:7, turn:2, action:{type:"submit"} }), /Connection lost/);
  const reopened = new OnlineTransport("/r2/api/online", send, store);
  assert.deepEqual(await reopened.recover(), {battle:{version:8}});
  assert.equal(calls.length, 2);
  assert.equal(calls[0]!.url, "/r2/api/online/matches/a/commands");
  assert.equal(calls[0]!.init!.body, calls[1]!.init!.body);
  assert.ok(JSON.parse(String(calls[0]!.init!.body)).commandId);
  assert.equal(calls[0]!.init!.credentials, "same-origin");
  await reopened.recover();
  assert.equal(calls.length, 2, "receipt is cleared only after a definite response");
});
test("stale command is rejected once and cannot replay as a new intention", async () => {
  let count = 0;
  const api = new OnlineTransport("/api/online", async () => { count++; return Response.json({error:"stale_version"},{status:409}); }, storage());
  await assert.rejects(api.command("/profile/commands", {expectedVersion:1, action:{type:"spend",stat:"power"}}), (e) => e instanceof OnlineError && e.status === 409);
  await api.recover();
  assert.equal(count, 1);
});
test("unknown result blocks a different mutation until recovery", async () => {
  let count=0;
  const api = new OnlineTransport("/api/online", async () => { count++; throw Error("offline"); }, storage());
  await assert.rejects(api.command("/invites", {}));
  await assert.rejects(api.command("/profile/commands", {}), /pending_command/);
  assert.equal(count,1);
});
test("signing into another account cannot replay the previous account's intention",async()=>{
  const store=storage(); let calls=0;
  const api=new OnlineTransport("/api/online",async()=>{calls++;throw Error("offline");},store);
  api.bindAccount("a"); await assert.rejects(api.command("/invites",{}));
  api.bindAccount("b"); await api.recover(); assert.equal(calls,1);
  api.bindAccount("a"); await assert.rejects(api.recover()); assert.equal(calls,2);
});
