import assert from "node:assert/strict";
import test from "node:test";
import { OnlineSession } from "../src/online/session.ts";

// Native DOM/fetch are the I/O boundary; the actual OnlineSession.login loop runs unchanged.
class Element {
  textContent="";
  children: Element[]=[];
  parent: Element | null=null;
  onclick: (()=>void) | null=null;
  tag: string;
  constructor(tag:string) { this.tag=tag; }
  append(...children:Element[]):void { for(const child of children){child.parent=this;this.children.push(child);} }
  replaceChildren():void { this.children=[]; }
  setAttribute():void {}
  remove():void { if(this.parent)this.parent.children=this.parent.children.filter(c=>c!==this); }
  all():Element[] { return [this,...this.children.flatMap(c=>c.all())]; }
}
async function until(predicate:()=>boolean, ms=7000):Promise<void> {
  const end=Date.now()+ms;
  while(!predicate()) { if(Date.now()>end)assert.fail("Expected login-flow state was not reached"); await new Promise(r=>setTimeout(r,10)); }
}

test("expired Mini App launch offers reopening instead of repeating the same rejected credentials",async(t)=>{
  const world=globalThis as any, keys=["document","window","location","sessionStorage","fetch"];
  const previous=keys.map(key=>[key,Object.getOwnPropertyDescriptor(world,key)] as const);
  t.after(()=>{for(const [key,descriptor] of previous){if(descriptor)Object.defineProperty(world,key,descriptor);else delete world[key];}});
  const body=new Element("body"),values=new Map<string,string>();let requests=0,closed=0;
  world.document={body,createElement:(tag:string)=>new Element(tag)};
  world.window=Object.assign(new EventTarget(),{Telegram:{WebApp:{initData:"expired-launch",close:()=>closed++}}});
  world.location={search:""};
  world.sessionStorage={getItem:(k:string)=>values.get(k)??null,setItem:(k:string,v:string)=>values.set(k,v),removeItem:(k:string)=>values.delete(k)};
  world.fetch=async()=>{requests++;return Response.json({error:"invalid_credentials"},{status:401});};
  const session=new OnlineSession("/api/online");void session.login();
  await until(()=>body.all().some(el=>el.tag==="button"));
  const close=body.all().find(el=>el.tag==="button" && el.textContent==="Закрыть Mini App");
  assert.ok(close,"expired credentials require a fresh Telegram launch");
  assert.ok(!body.all().some(el=>el.textContent==="Повторить"));
  close.onclick!();assert.equal(closed,1);assert.equal(requests,1);
  assert.equal(session.account,undefined);
});

test("confirmed then consume410 restarts real login with the same invitation",{timeout:20000},async(t)=>{
  const world=globalThis as any;
  const keys=["document","window","location","sessionStorage","fetch"];
  const previous=keys.map(key=>[key,Object.getOwnPropertyDescriptor(world,key)] as const);
  t.after(()=>{for(const [key,descriptor] of previous){if(descriptor)Object.defineProperty(world,key,descriptor);else delete world[key];}});
  const body=new Element("body"), values=new Map<string,string>();
  world.document={body,createElement:(tag:string)=>new Element(tag)};
  world.window=new EventTarget(); world.location={search:"?invite=AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"};
  world.sessionStorage={getItem:(k:string)=>values.get(k)??null,setItem:(k:string,v:string)=>values.set(k,v),removeItem:(k:string)=>values.delete(k)};
  const requests:{path:string,body:any}[]=[];
  let consumes=0, signedIn=false;
  const account={id:"synthetic-operator",version:1,appearance:{},loadout:{},profile:{},telegram:{firstName:"Тест"}};
  world.fetch=async(url:string,init?:RequestInit)=>{
    const path=new URL(url,"https://game.test").pathname;
    requests.push({path,body:init?.body?JSON.parse(String(init.body)):null});
    if(path.endsWith("/me"))return Response.json(signedIn?{account,maxLevel:4}:{error:"unauthorized"},{status:signedIn?200:401});
    if(path.endsWith("/start"))return Response.json({challengeId:"BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB",botUrl:"https://t.me/synthetic_bot?start=login_BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB",expiresAt:Date.now()+300000});
    if(path.endsWith("/status"))return Response.json({status:"confirmed",expiresAt:Date.now()+100});
    if(path.endsWith("/consume")) {
      if(++consumes===1)return Response.json({error:"browser_login_expired"},{status:410});
      signedIn=true;return Response.json({account,maxLevel:4,inviteToken:"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"});
    }
    if(path.endsWith("/activity"))return Response.json({invite:null,battle:null});
    throw Error(`Unexpected request ${path}`);
  };
  const session=new OnlineSession("/api/online");
  const running=session.login();
  const button=(label:string)=>body.all().find(el=>el.tag==="button" && el.textContent===label);
  await until(()=>!!button("Войти через Telegram")); button("Войти через Telegram")!.onclick!();
  await until(()=>!!button("Начать вход заново"));
  assert.ok(body.all().some(el=>el.textContent.includes("Время подтверждения истекло")));
  assert.ok(!body.all().some(el=>el.textContent.includes("Сервер недоступен")));
  assert.equal(consumes,1);
  assert.ok(requests.some(r=>r.path.endsWith("/status")),"confirmation was polled before consume");
  button("Начать вход заново")!.onclick!();
  await until(()=>requests.filter(r=>r.path.endsWith("/start")).length===2,1500);
  await running;
  assert.deepEqual(requests.filter(r=>r.path.endsWith("/start")).map(r=>r.body),[{inviteToken:"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"},{inviteToken:"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"}]);
  assert.equal(consumes,2); assert.equal(session.account.id,"synthetic-operator");
  assert.equal(values.get("cd:online:invite"),"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA","login itself does not consume the invitation");
});

test("malformed URL and stored invitation do not poison the browser login challenge",{timeout:15000},async(t)=>{
  const world=globalThis as any,keys=["document","window","location","sessionStorage","fetch"];
  const previous=keys.map(key=>[key,Object.getOwnPropertyDescriptor(world,key)] as const);
  t.after(()=>{for(const [key,descriptor] of previous){if(descriptor)Object.defineProperty(world,key,descriptor);else delete world[key];}});
  const body=new Element("body"),values=new Map([["cd:online:invite","old-invalid-token"]]);
  world.document={body,createElement:(tag:string)=>new Element(tag)};
  world.window=new EventTarget();world.location={search:"?invite=short"};
  world.sessionStorage={getItem:(k:string)=>values.get(k)??null,setItem:(k:string,v:string)=>values.set(k,v),removeItem:(k:string)=>values.delete(k)};
  const starts:unknown[]=[];let signedIn=false;
  const account={id:"synthetic-operator",version:1,appearance:{},loadout:{},profile:{},telegram:{firstName:"Тест"}};
  world.fetch=async(url:string,init?:RequestInit)=>{
    const path=new URL(url,"https://game.test").pathname;
    if(path.endsWith("/me"))return Response.json(signedIn?{account}:{error:"unauthorized"},{status:signedIn?200:401});
    if(path.endsWith("/start")) {
      const input=JSON.parse(String(init?.body));starts.push(input);
      if(input.inviteToken && !/^[A-Za-z0-9_-]{32}$/.test(input.inviteToken))return Response.json({error:"invalid_invite"},{status:400});
      return Response.json({challengeId:"BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB",botUrl:"https://t.me/synthetic_bot?start=login_BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB",expiresAt:Date.now()+300000});
    }
    if(path.endsWith("/status"))return Response.json({status:"confirmed",expiresAt:Date.now()+100});
    if(path.endsWith("/consume")){signedIn=true;return Response.json({account});}
    if(path.endsWith("/activity"))return Response.json({invite:null,battle:null});
    throw Error(`Unexpected request ${path}`);
  };
  const session=new OnlineSession("/api/online"),running=session.login();
  const button=(label:string)=>body.all().find(el=>el.tag==="button" && el.textContent===label);
  await until(()=>!!button("Войти через Telegram"));button("Войти через Telegram")!.onclick!();
  await until(()=>signedIn || !!button("Повторить"));
  assert.deepEqual(starts,[{}],"invalid invitation must not reach browser/start");
  await running;assert.equal(session.account.id,"synthetic-operator");
  assert.equal(values.has("cd:online:invite"),false);
});
