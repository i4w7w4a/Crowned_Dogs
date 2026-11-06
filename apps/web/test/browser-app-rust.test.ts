import test from "node:test";
import assert from "node:assert/strict";
import { getEventListeners } from "node:events";
import { RustApplication, mount_application, unmount_application } from "../../../packages/runtime/src/index.ts";

const settle = async () => { for (let i=0; i<16; i++) await Promise.resolve(); };
function deferred<T>() { let resolve!: (v: T) => void; const promise = new Promise<T>(r => { resolve=r; }); return {promise,resolve}; }
async function fixture(run: (f: ReturnType<typeof setup>) => Promise<void>) {
  const keys = ["document","location","history","addEventListener","removeEventListener"];
  const saved = keys.map(key => Object.getOwnPropertyDescriptor(globalThis,key));
  const f = setup();
  for (const [key,value] of Object.entries(f.globals)) Object.defineProperty(globalThis,key,{configurable:true,writable:true,value});
  try { await run(f); } finally {
    unmount_application();
    f.app?.free();
    keys.forEach((key,i) => { if(saved[i]) Object.defineProperty(globalThis,key,saved[i]!); else Reflect.deleteProperty(globalThis,key); });
  }
}
function setup() {
  const window = new EventTarget();
  const createRoot = () => ({ id:"", removed:false, focusCount:0, remove(){this.removed=true;},querySelector(){return {focus:()=>this.focusCount++};} });
  let root = createRoot();
  const document = Object.assign(new EventTarget(), {hidden:false,body:{prepend(){}},createElement:()=>{root=createRoot();return root;}});
  const location = {hash:"#fitting",search:"?muted=1"};
  const history = {replaceState(_a:unknown,_b:unknown,hash:string){location.hash=hash;}};
  const shows: string[] = [], activity: boolean[] = [], settings: boolean[] = [];
  let base: Base, loader: Loader, unlocked=0, disposed=0, starts=0;
  class Base {
    enter: (loadout: object) => Promise<void>;
    constructor(_root:unknown,enter:(loadout:object)=>Promise<void>){this.enter=enter;base=this;}
    show(view:string){shows.push(view);} setActive(on:boolean){activity.push(on);} dispose(){disposed++;}
  }
  class Loader {
    back:()=>void=()=>{}; hidden=0; shown=0; disposed=false;
    constructor(){loader=this;}
    show(back:()=>void){this.shown++;this.back=back;} ready(){return Promise.resolve();}
    failed(){return Promise.resolve("back");} hide(){this.hidden++;} dispose(){this.disposed=true;}
  }
  const imported = deferred<object>();
  const host = { BaseScreen:Base,MawLoader:Loader,selectBattle:undefined as undefined|((signal:AbortSignal)=>Promise<unknown>),
    gameAudio:{unlock(){unlocked++;}}, mountSoundSettings(){},showSoundSettings(on:boolean){settings.push(on);},
    loadBattle:()=>imported.promise,loadLocalLoadout:()=>({}) };
  const f = {
    app: undefined as RustApplication|undefined,
    globals:{document,location,history,addEventListener:window.addEventListener.bind(window),removeEventListener:window.removeEventListener.bind(window)},
    mount(){this.app=new RustApplication(host);return this.app;},
    selectBattle(select:(signal:AbortSignal)=>Promise<unknown>){host.selectBattle=select;},
    mountOwned(){mount_application(host);},
    get base(){return base!;},get loader(){return loader!;}, get unlocked(){return unlocked;},get disposed(){return disposed;},get starts(){return starts;},
    window,document,get root(){return root;},location,shows,activity,settings,
    finishImport(checkRemote?:(remote:unknown)=>void){imported.resolve({async startBattle(_loadout:unknown,_back:unknown,before:()=>Promise<void>,signal:AbortSignal,remote:unknown){signal.throwIfAborted();checkRemote?.(remote);await before();starts++;}});},
  };
  return f;
}
test("online lobby stays interactive until selection finishes, before the modal arena loader opens",async()=>fixture(async f=>{
  const selection=deferred<object>();const remote={id:"selected-server-match"};
  f.selectBattle(()=>selection.promise);f.mount();
  const pending=f.base.enter({});await settle();
  assert.equal(f.loader.shown,0,"a modal arena loader must not make the interactive lobby inert");
  f.finishImport(value=>assert.equal(value,remote));selection.resolve(remote);await pending;
  assert.equal(f.loader.shown,1);assert.equal(f.starts,1);
}));
test("leaving the online lobby returns to base without opening the arena loader",async()=>fixture(async f=>{
  f.selectBattle(async()=>{throw new DOMException("Back to base","AbortError");});f.mount();
  const pending=f.base.enter({});await settle();
  assert.equal(f.loader.shown,0);await pending;
  assert.equal(f.loader.shown,0);assert.equal(f.starts,0);assert.equal(f.location.hash,"#base");
  assert.equal(f.activity.at(-1),true);
}));
test("disposing during lobby selection cannot activate a late selected match",async()=>fixture(async f=>{
  const selection=deferred<object>();f.selectBattle(()=>selection.promise);
  const app=f.mount(),pending=f.base.enter({});await settle();app.dispose();
  selection.resolve({id:"late-server-match"});await pending;
  assert.equal(f.loader.shown,0);assert.equal(f.starts,0);assert.equal(f.location.hash,"#fitting");
}));
test("unexpected lobby failure restores the base and sound settings",async()=>fixture(async f=>{
  f.selectBattle(async()=>{throw Error("selection failed");});f.mount();
  await assert.rejects(f.base.enter({}),/selection failed/);
  assert.equal(f.loader.shown,0);assert.equal(f.location.hash,"#base");
  assert.equal(f.activity.at(-1),true);assert.equal(f.settings.at(-1),true);
}));
test("Rust app routes, visibility and gesture listeners dispose cleanly", async()=>fixture(async f=>{
  const app=f.mount(); assert.deepEqual(f.shows,["fitting"]);
  f.location.hash="#weapon";f.window.dispatchEvent(new Event("hashchange"));assert.deepEqual(f.shows,["fitting","weapon"]);
  for(const kind of ["pointerdown","touchend","click","keydown"]) f.document.dispatchEvent(new Event(kind));
  assert.equal(f.unlocked,4);
  f.document.hidden=true;f.document.dispatchEvent(new Event("visibilitychange"));assert.equal(f.activity.at(-1),false);
  f.window.dispatchEvent(new Event("pageshow"));assert.equal(f.activity.at(-1),true);
  app.dispose();app.dispose();assert.equal(f.disposed,1);assert.equal(f.root.removed,true);assert.equal(f.loader.disposed,true);
  assert.equal(getEventListeners(f.document,"pointerdown").length,0);assert.equal(getEventListeners(f.window,"hashchange").length,0);
}));
test("disposing Rust app cancels a pending import and cannot activate a late battle", async()=>fixture(async f=>{
  const app=f.mount();const pending=f.base.enter({});assert.equal(f.settings.at(-1),false);
  app.dispose();await pending;assert.equal(f.starts,0);assert.equal(f.loader.hidden,1);
  f.finishImport();await settle();assert.equal(f.starts,0);assert.equal(f.location.hash,"#fitting");
}));
test("Rust app Back abandons stalled loading and reactivates base once", async()=>fixture(async f=>{
  f.mount();const pending=f.base.enter({});f.loader.back();await pending;
  assert.equal(f.starts,0);assert.equal(f.location.hash,"#base");assert.equal(f.root.focusCount,1);assert.equal(f.activity.at(-1),true);
  f.finishImport();await settle();assert.equal(f.starts,0);
}));

function listenerCounts(f: ReturnType<typeof setup>) {
  return [
    ...["pointerdown","touchend","click","keydown","visibilitychange"].map(event=>getEventListeners(f.document,event).length),
    ...["hashchange","pagehide","pageshow"].map(event=>getEventListeners(f.window,event).length),
  ];
}
test("module-owned Rust app replaces and unmounts exactly one controller", async()=>fixture(async f=>{
  f.mountOwned();const firstRoot=f.root,firstLoader=f.loader;
  assert.equal(f.app,undefined,"mount never returns or stores a JS controller wrapper");
  assert.deepEqual(listenerCounts(f),Array(8).fill(1));
  f.mountOwned();
  assert.equal(firstRoot.removed,true);assert.equal(firstLoader.disposed,true);assert.equal(f.disposed,1);
  assert.notEqual(f.root,firstRoot);assert.equal(f.root.removed,false);
  assert.deepEqual(listenerCounts(f),Array(8).fill(1),"replacement must not duplicate global listeners");
  f.location.hash="#weapon";f.window.dispatchEvent(new Event("hashchange"));assert.equal(f.shows.at(-1),"weapon");
  unmount_application();unmount_application();
  assert.equal(f.disposed,2);assert.equal(f.root.removed,true);assert.deepEqual(listenerCounts(f),Array(8).fill(0));
  const shows=f.shows.length;f.window.dispatchEvent(new Event("hashchange"));assert.equal(f.shows.length,shows);
  f.document.dispatchEvent(new Event("click"));assert.equal(f.unlocked,0);
  f.mountOwned();assert.deepEqual(listenerCounts(f),Array(8).fill(1));assert.equal(f.root.removed,false);
  f.document.dispatchEvent(new Event("click"));assert.equal(f.unlocked,1);
  unmount_application();assert.equal(f.disposed,3);assert.deepEqual(listenerCounts(f),Array(8).fill(0));
}));

// Targeted command: node --expose-gc --test apps/web/test/browser-app-rust.test.ts
// A full event-loop turn is required for weak references and FinalizationRegistry.
test("Rust-owned application survives forced GC without a JavaScript controller reference", {
  skip: typeof globalThis.gc !== "function" ? "run Node with --expose-gc" : false,
}, async()=>fixture(async f=>{
  f.mountOwned();const root=f.root;
  assert.equal(f.app,undefined);
  let finalized=false;
  const registry=new FinalizationRegistry(()=>{finalized=true;});
  (()=>{registry.register({},"sentinel");})();
  for(let i=0;i<30;i++){
    await new Promise<void>(resolve=>setImmediate(resolve));
    globalThis.gc!();
    await new Promise<void>(resolve=>setImmediate(resolve));
    assert.equal(root.removed,false,"GC must not invoke application teardown");
    assert.deepEqual(listenerCounts(f),Array(8).fill(1));
    if(finalized&&i>=3)break;
  }
  assert.equal(finalized,true,"the regression must actually observe finalizer execution");
  assert.equal(f.disposed,0);
  f.location.hash="#base";f.window.dispatchEvent(new Event("hashchange"));assert.equal(f.shows.at(-1),"base");
  f.document.dispatchEvent(new Event("click"));assert.equal(f.unlocked,1);
  unmount_application();assert.equal(root.removed,true);assert.equal(f.disposed,1);assert.deepEqual(listenerCounts(f),Array(8).fill(0));
}));
