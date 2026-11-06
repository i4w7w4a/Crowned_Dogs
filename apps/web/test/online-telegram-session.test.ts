import assert from "node:assert/strict";
import test from "node:test";
import { OnlineError, OnlineTransport } from "../src/online/transport.ts";
import { telegramSession } from "../src/online/telegram-session.ts";

function storage() {const values=new Map<string,string>();return {values,getItem:(k:string)=>values.get(k)??null,setItem:(k:string,v:string)=>{values.set(k,v);},removeItem:(k:string)=>{values.delete(k);}};}
const key="telegram-binding";
test("expired initData resumes the same live cookie account and reads its current profile",async()=>{
  const store=storage();let fresh=true;const paths:string[]=[];
  const api=new OnlineTransport("/api/online",async(url)=>{
    paths.push(String(url));
    if(String(url).endsWith("/auth/telegram"))return fresh?Response.json({account:{id:"a",profile:{level:1}}}):Response.json({error:"invalid_credentials"},{status:401});
    return Response.json({account:{id:"a",profile:{level:2}}});
  },store);
  await telegramSession(api,"previously-server-verified-init-data",store,key);fresh=false;
  const result=await telegramSession(api,"previously-server-verified-init-data",store,key);
  assert.deepEqual(result.account,{id:"a",profile:{level:2}});
  assert.equal(paths.at(-1),"/api/online/me");
  assert.ok(!store.getItem(key)!.includes("previously-server-verified-init-data"),"raw credentials are not cached");
});
test("changed Telegram launch data cannot reuse the previous player's cookie",async()=>{
  const store=storage();let fresh=true,meReads=0;
  const api=new OnlineTransport("/api/online",async(url)=>{
    if(String(url).endsWith("/me")){meReads++;return Response.json({account:{id:"a"}});}
    return fresh?Response.json({account:{id:"a"}}):Response.json({error:"invalid_credentials"},{status:401});
  },store);
  await telegramSession(api,"actor-A-verified-init-data",store,key);fresh=false;
  await assert.rejects(telegramSession(api,"actor-B-expired-init-data",store,key),e=>e instanceof OnlineError && e.message==="mini_app_reopen");
  assert.equal(meReads,0);
});
test("an expired launch cannot silently take a cookie replaced by another account",async()=>{
  const store=storage();let fresh=true;
  const api=new OnlineTransport("/api/online",async(url)=>String(url).endsWith("/me")?Response.json({account:{id:"b"}}):
    fresh?Response.json({account:{id:"a"}}):Response.json({error:"invalid_credentials"},{status:401}),store);
  await telegramSession(api,"actor-A-verified-init-data",store,key);fresh=false;
  await assert.rejects(telegramSession(api,"actor-A-verified-init-data",store,key),e=>e instanceof OnlineError && e.message==="mini_app_reopen");
});
test("missing or expired cookies never turn a stored routing hint into authentication",async()=>{
  const store=storage();let fresh=true;
  const api=new OnlineTransport("/api/online",async()=>fresh?Response.json({account:{id:"a"}}):Response.json({error:"unauthorized"},{status:401}),store);
  await telegramSession(api,"actor-A-verified-init-data",store,key);fresh=false;
  await assert.rejects(telegramSession(api,"actor-A-verified-init-data",store,key),e=>e instanceof OnlineError && e.message==="mini_app_reopen");
});
