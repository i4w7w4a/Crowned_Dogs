import test from "node:test";
import assert from "node:assert/strict";
import { getEventListeners } from "node:events";
import { BrowserBase, BrowserScroll, BrowserWeapon, equipment_icon, equipment_item_icon } from "../../../packages/runtime/src/index.ts";
import type { GearSlot } from "@crowned-dogs/core";

test("Rust equipment slots draw each item, fall back per slot and keep unknown items blank", () => {
  const ids = ["", "head-cap", "head-sfera", "head-marevo", "head-filin", "head-sych", "body-light", "body-plita4", "body-bastion", "suit-leshy", "suit-ryvok", "suit-ten", "suit-solonchak", "boots-sprint", "boots-saper", "boots-hodok", "knife-kogot", "knife-shtyk", "knife-zhalo", "knife-iskra", "grenade-emp", "grenade-smoke", "grenade-grad", "rifle-ak", "pistol-pm", "sniper-svd", "shotgun-12", "unknown-item"];
  const slots: GearSlot[] = ["head", "body", "suit", "boots", "knife", "grenade", "weapon1", "weapon2"];
  const art = (svg: string) => svg.replace(/gear-\d+/g, "gear");
  const masks = new Set<string>();
  for (const id of ids) {
    for (const slot of slots) {
      const icon = equipment_icon(slot, id);
      assert.match(icon, /^<svg class="cd-equipment-icon" [^>]*viewBox="[-\d. ]+"[^>]*aria-hidden="true"/, `${slot}: ${id}`);
      for (const [, mask] of icon.matchAll(/mask id="([^"]+)"/g)) {
        assert.ok(!masks.has(mask!), `${slot}: ${id} has its own mask`);
        masks.add(mask!);
      }
      const fits = id.startsWith(`${slot}-`) || (slot.startsWith("weapon") && /^(pistol|rifle|shotgun|sniper)-/.test(id));
      if (!fits) assert.equal(art(icon), art(equipment_icon(slot, "")), `${slot}: ${id} falls back`);
    }
    const prefix = id.split("-")[0]!;
    if (id === "unknown-item" || id === "") assert.equal(equipment_item_icon(id), "", id);
    else assert.equal(art(equipment_item_icon(id)), art(equipment_icon(/^(pistol|rifle|shotgun|sniper)$/.test(prefix) ? "weapon1" : prefix as GearSlot, id)), id);
  }
  assert.notEqual(art(equipment_icon("weapon2", "")), art(equipment_icon("weapon1", "")), "the second slot shows a pistol");
  const worn = ids.filter((id) => /^(head|body|suit|boots|knife|grenade)-/.test(id)).map((id) => art(equipment_item_icon(id)));
  assert.equal(new Set(worn).size, worn.length, "every worn item has its own drawing");
});

class Element extends EventTarget {
  className = "";
  children: Element[] = [];
  parentNode: Element | null = null;
  hidden = false;
  scrollTop = 0;
  scrollLeft = 0;
  clientHeight = 200;
  scrollHeight = 1000;
  attributes = new Map<string, string>();
  styles = new Map<string, string>();
  captures = new Set<number>();
  style = { setProperty: (key: string, value: string) => this.styles.set(key, value) };
  classList = {
    contains: (name: string) => this.className.split(" ").includes(name),
    toggle: (name: string, on: boolean) => {
      const names = new Set(this.className.split(" ").filter(Boolean));
      if (on) names.add(name); else names.delete(name);
      this.className = [...names].join(" ");
    },
  };
  get tabIndex() { return Number(this.attributes.get("tabindex") ?? -1); }
  set tabIndex(value: number) { this.attributes.set("tabindex", String(value)); }
  append(...children: Element[]) { for (const child of children) { child.remove(); child.parentNode = this; this.children.push(child); } }
  insertBefore(child: Element, next: Element) { child.remove(); child.parentNode = this; this.children.splice(this.children.indexOf(next), 0, child); }
  remove() { if (this.parentNode) { this.parentNode.children = this.parentNode.children.filter(child => child !== this); this.parentNode = null; } }
  contains(child: Element | null): boolean { return child === this || this.children.some(candidate => candidate.contains(child)); }
  setAttribute(key: string, value: string) { this.attributes.set(key, value); }
  getAttribute(key: string) { return this.attributes.get(key) ?? null; }
  hasAttribute(key: string) { return this.attributes.has(key); }
  removeAttribute(key: string) { this.attributes.delete(key); }
  focus() { fakeDocument.activeElement = this; }
  getBoundingClientRect() { return { top: 0 }; }
  setPointerCapture(id: number) { this.captures.add(id); }
  hasPointerCapture(id: number) { return this.captures.has(id); }
  releasePointerCapture(id: number) { this.captures.delete(id); pointer(this, "lostpointercapture", id, 0); }
}
/** Selector fixtures exercise controller ownership, without pretending to render a browser layout. */
class SelectorElement extends Element {
  selectors = new Map<string, SelectorElement>();
  get parentElement() { return this.parentNode; }
  querySelector(selector: string) {
    if (!this.selectors.has(selector)) { const element = new SelectorElement(); this.append(element); this.selectors.set(selector, element); }
    return this.selectors.get(selector)!;
  }
  querySelectorAll() { return []; }
}
const fakeDocument = { activeElement: null as Element | null, createElement: () => new SelectorElement() };
class Observer {
  static all: Observer[] = [];
  observed = new Set<Element>();
  disconnected = false;
  callback: () => void;
  constructor(callback: () => void) { this.callback = callback; Observer.all.push(this); }
  observe(element: Element) { this.observed.add(element); }
  unobserve(element: Element) { this.observed.delete(element); }
  disconnect() { this.disconnected = true; this.observed.clear(); }
}
function pointer(element: Element, name: string, id: number, y: number, primary = true) {
  const event = new Event(name, { cancelable: true });
  Object.assign(event, { pointerId: id, clientY: y, button: 0, isPrimary: primary });
  element.dispatchEvent(event);
}
function withDom(run: () => void) {
  const old = ["document", "ResizeObserver", "MutationObserver"].map(key => Object.getOwnPropertyDescriptor(globalThis, key));
  Object.assign(globalThis, { document: fakeDocument, ResizeObserver: Observer, MutationObserver: Observer });
  Observer.all = []; fakeDocument.activeElement = null;
  try { run(); } finally {
    ["document", "ResizeObserver", "MutationObserver"].forEach((key, i) => {
      if (old[i]) Object.defineProperty(globalThis, key, old[i]!); else Reflect.deleteProperty(globalThis, key);
    });
  }
}
test("Rust scrollbar supports drag cancellation, dynamic contents and reversible mounting", () => withDom(() => {
  const parent = new Element(), viewport = new Element(), focused = new Element();
  parent.append(viewport); viewport.append(focused); focused.focus(); viewport.scrollTop = 100;
  const scroll = new BrowserScroll(viewport);
  const frame = parent.children[0]!, track = frame.children[1]!;
  assert.equal(fakeDocument.activeElement, focused);
  assert.equal(viewport.scrollTop, 100);
  assert.equal(track.hidden, false);
  pointer(track, "pointerdown", 1, 50, false);
  assert.equal(track.hasPointerCapture(1), false);
  pointer(track, "pointerdown", 1, 50);
  assert.equal(track.hasPointerCapture(1), true);
  const initial = viewport.scrollTop;
  pointer(track, "pointermove", 2, 200);
  assert.equal(viewport.scrollTop, initial, "other pointers cannot hijack a drag");
  pointer(track, "pointermove", 1, 1000);
  assert.equal(viewport.scrollTop, 800);
  pointer(track, "pointercancel", 1, 1000);
  assert.equal(track.hasPointerCapture(1), false);
  pointer(track, "pointermove", 1, 0);
  assert.equal(viewport.scrollTop, 800);
  const replacement = new Element(); focused.remove(); viewport.append(replacement);
  Observer.all[1]!.callback();
  assert.equal(Observer.all[0]!.observed.has(focused), false);
  assert.equal(Observer.all[0]!.observed.has(replacement), true);
  replacement.focus(); viewport.scrollHeight = viewport.clientHeight; scroll.refresh();
  assert.equal(track.hidden, true);
  scroll.dispose(); scroll.dispose();
  assert.deepEqual(parent.children, [viewport]);
  assert.equal(fakeDocument.activeElement, replacement);
  assert.equal(viewport.classList.contains("cd-scroll-viewport"), false);
  assert.equal(viewport.hasAttribute("tabindex"), false);
  assert.ok(Observer.all.every(observer => observer.disconnected));
}));

test("Rust scrollbar preserves an existing viewport class and keyboard tab order", () => withDom(() => {
  const parent = new Element(), viewport = new Element(); parent.append(viewport);
  viewport.className = "existing cd-scroll-viewport"; viewport.tabIndex = 3;
  const scroll = new BrowserScroll(viewport); scroll.dispose();
  assert.equal(viewport.className, "existing cd-scroll-viewport");
  assert.equal(viewport.tabIndex, 3);
}));

test("free without dispose detaches scrollbar listeners and observers", () => withDom(() => {
  const parent = new Element(), viewport = new Element(); parent.append(viewport);
  const scroll = new BrowserScroll(viewport);
  const track = parent.children[0]!.children[1]!;
  pointer(track, "pointerdown", 3, 50);
  assert.equal(track.hasPointerCapture(3), true);
  assert.equal(getEventListeners(track, "pointermove").length, 1);
  scroll.free();
  assert.equal(track.hasPointerCapture(3), false);
  assert.equal(getEventListeners(track, "pointermove").length, 0);
  assert.equal(getEventListeners(viewport, "scroll").length, 0);
  assert.deepEqual(parent.children, [viewport]);
  assert.ok(Observer.all.every(observer => observer.disconnected));
}));

test("free without dispose removes the weapon workshop and its event handlers", () => withDom(() => {
  const parent = new SelectorElement();
  const workshop = new BrowserWeapon(parent, () => {}, () => true, { baseUrl: "/", backArrow: "", liquidGlass() {} });
  const element = workshop.element as SelectorElement;
  const back = element.querySelector(".cw-back");
  assert.equal(getEventListeners(back, "click").length, 1);
  workshop.free();
  assert.equal(getEventListeners(back, "click").length, 0);
  assert.equal(element.parentNode, null);
  assert.ok(Observer.all.every(observer => observer.disconnected));
}));

test("free without dispose cancels base RAF, global tilt listeners and owned viewers", () => withDom(() => {
  const globals = ["requestAnimationFrame", "cancelAnimationFrame", "matchMedia", "addEventListener", "removeEventListener"];
  const old = globals.map(key => Object.getOwnPropertyDescriptor(globalThis, key));
  const events = new EventTarget();
  const frames = new Map<number, (at: number) => void>();
  let nextFrame = 0, workshopDisposals = 0, depthDisposals = 0;
  Object.assign(globalThis, {
    requestAnimationFrame: (callback: (at: number) => void) => { const id = ++nextFrame; frames.set(id, callback); return id; },
    cancelAnimationFrame: (id: number) => { frames.delete(id); },
    matchMedia: () => ({ matches: true }),
    addEventListener: events.addEventListener.bind(events), removeEventListener: events.removeEventListener.bind(events),
  });
  try {
    const parent = new Element(), root = new SelectorElement(); parent.append(root);
    class Workshop { dispose() { workshopDisposals++; } }
    class Depth { ready = Promise.resolve(); dispose() { depthDisposals++; } }
    const base = new BrowserBase(root, async () => {}, {
      baseUrl: "/", backArrow: "", GEAR_SLOT_NAMES: {}, GEAR_SLOTS: [], ITEM_BY_ID: new Map(),
      loadLocalLoadout: () => ({ weapon1: { item: "rifle-ak" } }),
      compileKit: () => ({ armor: { head: 0, body: 0, legs: 0 } }),
      WeaponScreen: Workshop, DepthBackdrop: Depth, liquidGlass() {},
    });
    const back = root.querySelector(".cd-fitting .cd-back");
    assert.equal(frames.size, 1);
    // Only the fitting room follows device tilt; the base photograph stays fixed.
    assert.equal(getEventListeners(events, "deviceorientation").length, 1);
    assert.equal(getEventListeners(back, "click").length, 1);
    base.free();
    assert.equal(frames.size, 0);
    assert.equal(getEventListeners(events, "deviceorientation").length, 0);
    assert.equal(getEventListeners(back, "click").length, 0);
    assert.equal(root.parentNode, null);
    assert.equal(workshopDisposals, 1);
    assert.equal(depthDisposals, 1);
    assert.ok(Observer.all.every(observer => observer.disconnected));
  } finally {
    globals.forEach((key, i) => { if (old[i]) Object.defineProperty(globalThis, key, old[i]!); else Reflect.deleteProperty(globalThis, key); });
  }
}));

test("base waits for its camera journey, ignores repeated clicks and cancels navigation on disposal", async () => {
  const keys=["document","ResizeObserver","MutationObserver","requestAnimationFrame","cancelAnimationFrame","matchMedia","addEventListener","removeEventListener","history"];
  const old=keys.map(k=>Object.getOwnPropertyDescriptor(globalThis,k));
  const events=new EventTarget(),frames=new Map<number,(time:number)=>void>();
  let next=0,battles=0,disposed=0;const draws:Array<[string,number]>=[];
  Object.assign(globalThis,{
    document:fakeDocument,ResizeObserver:Observer,MutationObserver:Observer,
    requestAnimationFrame:(cb:(t:number)=>void)=>{const id=++next;frames.set(id,cb);return id;},
    cancelAnimationFrame:(id:number)=>frames.delete(id),matchMedia:()=>({matches:false}),
    addEventListener:events.addEventListener.bind(events),removeEventListener:events.removeEventListener.bind(events),
    history:{replaceState(){}},
  });
  const root=new SelectorElement(),home=root.querySelector(".cd-home");
  for(const selector of [".cd-enter",".cd-to-fitting"]) root.selectors.set(selector,home.querySelector(selector));
  class Home {ready=Promise.resolve();draw(target:string,p:number){draws.push([target,p]);}dispose(){disposed++;}}
  class Workshop {dispose(){} hide(){}}
  class Depth {ready=Promise.resolve();dispose(){} draw(){}}
  const step=(time:number)=>{const pending=[...frames.values()];frames.clear();for(const callback of pending)callback(time);};
  let base:BrowserBase|undefined;
  try {
    base=new BrowserBase(root,async()=>{battles++;},{
      baseUrl:"/",backArrow:"",GEAR_SLOT_NAMES:{},GEAR_SLOTS:[],ITEM_BY_ID:new Map(),
      loadLocalLoadout:()=>({weapon1:{item:"rifle-ak"}}),compileKit:()=>({armor:{head:0,body:0,legs:0}}),
      WeaponScreen:Workshop,DepthBackdrop:Depth,HomeStage:Home,liquidGlass(){},gameAudio:{screen(){},voice(){},stopVoice(){}},
    });
    await new Promise(resolve=>setImmediate(resolve));
    const enter=root.querySelector(".cd-enter");const now=performance.now();
    enter.dispatchEvent(new Event("click"));enter.dispatchEvent(new Event("click"));
    assert.equal(battles,0);assert.equal(home.getAttribute("aria-busy"),"true");
    step(now+1000);assert.equal(battles,0);assert.ok(draws.at(-1)![1]>0);
    step(now+2200);assert.equal(battles,1);step(now+2300);assert.equal(battles,1);
    await new Promise(resolve=>setImmediate(resolve));
    base.show("base");enter.dispatchEvent(new Event("click"));base.free();base=undefined;
    step(now+8000);assert.equal(battles,1);assert.equal(disposed,1);assert.equal(frames.size,0);
  } finally {
    base?.free();keys.forEach((k,i)=>{if(old[i])Object.defineProperty(globalThis,k,old[i]!);else Reflect.deleteProperty(globalThis,k);});
  }
});
