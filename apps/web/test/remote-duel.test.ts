import assert from "node:assert/strict";
import test from "node:test";
import {readFileSync} from "node:fs";
import {RemoteDuel} from "../src/online/remote-duel.ts";
import {OnlineTransport} from "../src/online/transport.ts";
const fixture=JSON.parse(readFileSync(new URL("./fixtures/online-public-v1.json",import.meta.url),"utf8"));
const store={getItem:()=>null,setItem:()=>{},removeItem:()=>{}};
test("late receipt cannot replace a newer snapshot, and server time ignores device clock",()=>{
  const api=new OnlineTransport("/api/online",fetch,store);
  const duel=new RemoteDuel(api,{...fixture.active,serverNow:5000,deadline:35000});
  assert.equal(duel.fighters[0].id,fixture.active.players[1].id);
  assert.ok(duel.seconds>=29 && duel.seconds<=30);
  duel.accept({...fixture.loading,serverNow:5000});
  assert.equal(duel.snapshot.phase,"active");
  duel.accept({...fixture.finished});
  assert.equal(duel.over,true);
  assert.equal(duel.match.result.winner,0);
});
test("remote controls send session commands without local resolution or actor identity",async()=>{
  let sent:any;
  const api=new OnlineTransport("/api/online",async(_url,init)=>{
    sent=JSON.parse(String(init?.body)); return Response.json({battle:{...fixture.active,version:4,own:{...fixture.active.own,submitted:true}}});
  },store);
  const duel=new RemoteDuel(api,fixture.active);
  await duel.submit();
  assert.deepEqual(sent.action,{type:"submit"});
  assert.equal(sent.expectedVersion,3); assert.equal(sent.turn,1);
  assert.equal(sent.playerId,undefined); assert.equal(sent.side,undefined);
  assert.equal(duel.snapshot.own.submitted,true); assert.equal(duel.match.history.length,0);
});
test("polling returns a candidate snapshot so an in-flight animation stays unchanged",async()=>{
  const api=new OnlineTransport("/api/online",async()=>Response.json({battle:fixture.finished}),store);
  const duel=new RemoteDuel(api,fixture.active);
  const candidate=await duel.refresh();
  assert.equal(duel.over,false);
  duel.accept(candidate);
  assert.equal(duel.over,true);
});
