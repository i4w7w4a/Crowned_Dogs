import assert from "node:assert/strict";
import test from "node:test";
import { RustBattle, callRust } from "../../../packages/runtime/src/index.ts";
import { starterLoadout, VETERAN_KITS, WEAPONS, WEAPON_IDS, type TurnReport } from "@crowned-dogs/core";
import { LocalDuel, type LocalDuelOptions } from "../src/game/local-duel.ts";

type Task = { at: number; interval: number; run: () => void };
class Clock {
  now = 0; next = 1; tasks = new Map<number, Task>(); frames = new Map<number, () => void>();
  schedule = (run: () => void, delay = 0, interval = 0) => { const id = this.next++; this.tasks.set(id, { at: this.now + delay, interval, run }); return id; };
  clear = (id: number) => { this.tasks.delete(id); };
  advance(ms: number) {
    const end = this.now + ms;
    for (;;) {
      const next = [...this.tasks.entries()].filter(([, t]) => t.at <= end).sort((a, b) => a[1].at - b[1].at)[0];
      if (!next) break;
      const [id, task] = next; this.now = task.at;
      if (task.interval) task.at += task.interval; else this.tasks.delete(id);
      task.run();
    }
    this.now = end;
  }
}
class Element extends EventTarget {
  id = ""; className = ""; hidden = false; disabled = false; textContent = ""; value = "";
  offsetTop = 0; offsetWidth = 40; clientWidth = 600; clientHeight = 1000;
  dataset: Record<string, string> = {}; children: Element[] = []; parent: Element | null = null;
  attrs = new Map<string, string>(); selectors = new Map<string, Element>();
  style: Record<string, unknown> = { setProperty: (name: string, value: string) => { this.style[name] = value; } };
  private markup = "";
  get innerHTML() { return this.markup; }
  set innerHTML(value: string) { this.markup = value; this.children = []; }
  classList = {
    contains: (name: string) => this.className.split(" ").includes(name),
    toggle: (name: string, on: boolean) => { const names = new Set(this.className.split(" ").filter(Boolean)); if (on) names.add(name); else names.delete(name); this.className = [...names].join(" "); },
  };
  appendChild(child: Element) { child.parent = this; this.children.push(child); return child; }
  remove() { if (this.parent) this.parent.children = this.parent.children.filter(child => child !== this); this.parent = null; }
  setAttribute(key: string, value: string) { this.attrs.set(key, value); }
  getAttribute(key: string) { return this.attrs.get(key) ?? null; }
  querySelector(key: string) { if (!this.selectors.has(key)) this.selectors.set(key, new Element()); return this.selectors.get(key)!; }
  getBoundingClientRect() { return { top: 0, left: 0, bottom: this.clientHeight, right: this.clientWidth }; }
  contains(child: Element): boolean { return child === this || this.children.some(node => node.contains(child)); }
  closest() { return this; }
  focus() {}
  click() { if (!this.disabled) this.dispatchEvent(new Event("click")); }
  decode() { return Promise.resolve(); }
}
function deferred() {
  let resolve!: () => void, reject!: (error: Error) => void;
  const promise = new Promise<void>((yes, no) => { resolve = yes; reject = no; });
  return { promise, resolve, reject };
}
const settle = async () => { for (let i = 0; i < 20; i++) await Promise.resolve(); };

type HarnessOptions = { search?: string; time?: string; map?: string; now?: number };
async function harness(run: (h: ReturnType<typeof createHarness>) => Promise<void>, options: HarnessOptions = {}) {
  const h = createHarness(options);
  const before = Object.fromEntries(Object.keys(h.globals).map(key => [key, Object.getOwnPropertyDescriptor(globalThis, key)]));
  for (const [key, value] of Object.entries(h.globals)) Object.defineProperty(globalThis, key, { configurable: true, writable: true, value });
  const realNow = Date.now;
  if (options.now !== undefined) Date.now = () => h.now;
  try { h.battle = new RustBattle(h.bindings); await run(h); }
  finally {
    Date.now = realNow;
    h.clock.tasks.clear(); h.clock.frames.clear();
    for (const key of Object.keys(h.globals)) { if (before[key]) Object.defineProperty(globalThis, key, before[key]!); else Reflect.deleteProperty(globalThis, key); }
    Reflect.deleteProperty(globalThis, "__cd");
  }
}
function createHarness(options: HarnessOptions = {}) {
  const clock = new Clock(), elements = new Map<string, Element>(), events = new EventTarget(), doc = new EventTarget();
  const calls: { name: string; at: number; args: unknown[] }[] = [];
  const record = (name: string, args: unknown[] = []) => calls.push({ name, at: clock.now, args });
  const element = (id: string) => { if (!elements.has(id)) { const el = new Element(); el.id = id; elements.set(id, el); } return elements.get(id)!; };
  const guards = ["head", "body", "groin", "legs"].map(z => { const el = new Element(); el.className = "dz"; el.dataset.z = z; return el; });
  const moves = [-2, -1, 0, 1, 2, 3].map(m => { const el = new Element(); el.className = "mv"; el.dataset.m = String(m); return el; });
  const panel = new Element(); panel.querySelector(".ctl").offsetTop = 800;
  const fakeDocument = Object.assign(doc, { hidden: false, getElementById: element, createElement: () => new Element(),
    querySelector: (selector: string) => selector === ".panel" ? panel : element(selector),
    querySelectorAll: (selector: string) => selector === ".dz" ? guards : selector === ".mv" ? moves : [],
  });
  for (const id of ["app", "infoVeil", "endVeil"]) element(id).hidden = true;
  const load = deferred(), textures = deferred(); let loading = () => load.promise; let arena!: FakeArena;
  class FakeArena {
    ready = false; gl = {}; onFrame?: () => void; texturesDone = textures.promise;
    constructor() { arena = this; }
    async load() { record("arena.load"); await loading(); this.ready = true; }
    setFirstPerson(...args: unknown[]) { record("arena.camera", args); }
    setQuality(...args: unknown[]) { record("arena.quality", args); }
    setFrame(...args: unknown[]) { record("arena.frame", args); }
    setConditions(...args: unknown[]) { record("arena.conditions", args); }
    resize() {} zoneOnScreen() { return [0.6, 0.4]; }
    arm(...args: unknown[]) { record("arena.arm", args); }
    reset(...args: unknown[]) { record("arena.reset", args); }
    resume() { record("arena.resume"); } pause() { record("arena.pause"); }
    move(...args: unknown[]) { record("arena.move", args); }
    attack(...args: unknown[]) { record("arena.attack", args); }
    hit(...args: unknown[]) { record("arena.hit", args); }
    fall(...args: unknown[]) { record("arena.fall", args); }
  }
  class FightLog {
    open = false;
    setOpen(value: boolean) { this.open = value; record("log.open", [value]); }
    note(...args: unknown[]) { record("log.note", args); }
    reset(...args: unknown[]) { record("log.reset", args); }
    add(...args: unknown[]) { record("log.add", args); }
  }
  class SeededDuel extends LocalDuel { constructor(options: LocalDuelOptions) { super({ ...options, seed: "battle-controller-review" }); } }
  const audio = { on: false, station: "test", volume: { fx: 0, music: 0, fight: 0 },
    play: (...args: unknown[]) => record("audio.play", args), turn: (...args: unknown[]) => record("audio.turn", args),
    finish: (...args: unknown[]) => record("audio.finish", args), sheet: (...args: unknown[]) => record("audio.sheet", args),
    newFight: (...args: unknown[]) => record("audio.newFight", args), unlock: () => record("audio.unlock"),
    toggle: () => {}, tune: () => {}, setVolume: () => {},
  };
  const data = callRust<{ TIMING: { ourAttack: number; enemyAttack: number; knifeLands: number } }>("client.data", []);
  const stored = new Map<string, string>();
  if (options.time) stored.set("cd-time", options.time);
  if (options.map) stored.set("cd-map", options.map);
  const bindings = { Arena: FakeArena, LocalDuel: SeededDuel, FightLog, gameAudio: audio, liquidGlass: () => {}, icon: () => "<svg/>",
    store: { get: (key: string) => stored.get(key) ?? null, set: (key: string, value: string) => stored.set(key, value) },
    startQuality: () => ({ id: "low", picked: true }),
    frames: [...["day", "morning", "evening", "night"].map(time => ({
      id: time === "day" ? "hall1" : `hall-${time}`, name: "Соляной цех", src: `${time}.png`,
      w: 941, h: 1672, top: 0, mapId: "saltHall", weather: "clear", time,
    })), ...["day", "morning", "evening", "night"].map(time => ({
      id: `blocks-${time}`, name: "Пятиэтажки", src: `blocks-${time}.png`,
      w: 941, h: 1672, top: 0, mapId: "blocks", weather: "clear", time,
    })), ...[["dam", "dam", "Плотина"], ["drybed", "dryBed", "Сухое дно"]].flatMap(([prefix, mapId, name]) => ["day", "morning", "evening", "night"].map(time => ({
      id: `${prefix}-${time}`, name, src: `${prefix}-${time}.png`,
      w: 941, h: 1672, top: 0, mapId, weather: "clear", time,
    })))],
    weaponIds: WEAPON_IDS, weapons: WEAPONS, veteranKits: VETERAN_KITS, playlist: [], tracks: {}, timing: data.TIMING,
    myModel: "me.json", foeModel: "enemy.json",
  };
  const globals = { document: fakeDocument, location: { hash: "", search: options.search ?? "" }, matchMedia: () => ({ matches: false }),
    ResizeObserver: class { observe() {} }, addEventListener: events.addEventListener.bind(events),
    setTimeout: (cb: () => void, delay: number) => clock.schedule(cb, delay), clearTimeout: clock.clear,
    setInterval: (cb: () => void, delay: number) => clock.schedule(cb, delay, delay), clearInterval: clock.clear,
    requestAnimationFrame: (cb: () => void) => { const id = clock.next++; clock.frames.set(id, cb); return id; },
    cancelAnimationFrame: (id: number) => clock.frames.delete(id),
  };
  return { now: options.now ?? 0, bindings, globals, clock, element, guards, panel, calls, load, textures, timing: data.TIMING, stored, doc,
    battle: null as RustBattle | null, arena: () => arena,
    setLoading: (next: () => Promise<void>) => { loading = next; },
    duel: () => (globalThis as typeof globalThis & { __cd: { duel: LocalDuel } }).__cd.duel,
    click: (id: string) => element(id).click(),
    ready: async () => { load.resolve(); textures.resolve(); await settle(); },
  };
}

test("Rust battle never activates after cancellation during arena load or overlay exit", async () => {
  await harness(async h => {
    const controller = new AbortController(); let entered = 0;
    const start = h.battle!.start(starterLoadout("rifle"), () => {}, () => { entered++; }, controller.signal);
    await settle(); controller.abort(); const rejected = assert.rejects(start, { name: "AbortError" });
    await h.ready(); await rejected;
    assert.equal(entered, 0); assert.equal(h.element("app").hidden, true);
    assert.equal(h.calls.some(c => c.name === "arena.resume" || c.name === "audio.newFight"), false);
    const overlay = deferred(), next = new AbortController();
    const second = h.battle!.start(starterLoadout("rifle"), () => {}, () => overlay.promise, next.signal);
    await settle(); assert.equal([...h.clock.tasks.values()].some(t => t.interval), false, "turn clock waits for the loader");
    next.abort(); const rejectedAgain = assert.rejects(second, { name: "AbortError" }); overlay.resolve(); await rejectedAgain;
    assert.equal(h.element("app").hidden, true);
    assert.equal([...h.clock.tasks.values()].some(t => t.interval), false);
  });
});

test("Rust battle waits for texture readiness, pauses the turn clock in menus, and exits cleanly", async () => harness(async h => {
  let exits = 0; const start = h.battle!.start(starterLoadout("rifle"), () => exits++, async () => {}, undefined);
  await settle(); h.load.resolve(); await settle();
  assert.equal(h.element("app").hidden, true, "model readiness alone must not reveal the fight");
  h.textures.resolve(); await start;
  assert.equal(h.element("app").hidden, false);
  const initial = h.element("timer").textContent; h.clock.advance(1000); const elapsed = h.element("timer").textContent;
  assert.notEqual(elapsed, initial);
  h.click("moreBtn"); assert.equal(h.element("infoVeil").hidden, false); h.clock.advance(10000);
  assert.equal(h.element("timer").textContent, elapsed, "menu pauses countdown without resetting it");
  const escape = new Event("keydown"); Object.assign(escape, { key: "Escape" }); h.doc.dispatchEvent(escape);
  assert.equal(h.element("infoVeil").hidden, true); h.clock.advance(1000); assert.notEqual(h.element("timer").textContent, elapsed);
  h.click("moreBtn"); h.click("baseInfoBtn"); assert.equal(exits, 1); assert.equal(h.element("app").hidden, true);
  assert.equal(h.element("infoVeil").hidden, true); assert.equal(h.element("endVeil").hidden, true);
  assert.equal([...h.clock.tasks.values()].some(t => t.interval), false); assert.equal(h.clock.frames.size, 0);
  assert.ok(h.calls.some(c => c.name === "arena.pause"));
}));

test("Rust battle retries failed arena preparation and bounds the optional texture wait", async () => harness(async h => {
  const failed = h.battle!.start(starterLoadout("rifle"), () => {}, async () => {}, undefined);
  const rejection = assert.rejects(failed, /model download failed/);
  await settle(); h.load.reject(new Error("model download failed")); await rejection;
  assert.equal(h.element("app").hidden, true);
  h.setLoading(async () => {});
  const retry = h.battle!.start(starterLoadout("rifle"), () => {}, async () => {}, undefined);
  await settle(); assert.equal(h.calls.filter(c => c.name === "arena.load").length, 2);
  h.clock.advance(24999); await settle(); assert.equal(h.element("app").hidden, true);
  h.clock.advance(1); await retry;
  assert.equal(h.element("app").hidden, false, "embedded textures remain usable after the 25 s optional wait");
}));

test("Rust battle displays health on strike beats and invalidates abandoned turn callbacks", async () => harness(async h => {
  const start = h.battle!.start(starterLoadout("rifle"), () => {}, async () => {}, undefined); await h.ready(); await start;
  const selectOrder = () => { h.element("rings").children[1]!.click(); h.guards[0]!.click(); h.guards[1]!.click(); };
  selectOrder(); const before = [h.element("hpMe").textContent, h.element("hpEn").textContent]; h.click("goBtn");
  const report = h.calls.find(c => c.name === "audio.turn")!.args[0] as TurnReport;
  assert.equal(h.panel.classList.contains("playing"), true);
  assert.deepEqual([h.element("hpMe").textContent, h.element("hpEn").textContent], before);
  h.clock.advance(h.timing.ourAttack * 1000);
  assert.equal(Number(h.element("hpEn").textContent), report.hp[1]);
  assert.equal(h.element("hpMe").textContent, before[0]);
  h.clock.advance((h.timing.enemyAttack - h.timing.ourAttack) * 1000);
  assert.equal(Number(h.element("hpMe").textContent), report.hp[0]);
  assert.equal(h.calls.filter(c => c.name === "log.add").length, 1);
  h.clock.advance(2500 - h.timing.enemyAttack * 1000);
  assert.equal(h.panel.classList.contains("playing"), false); assert.match(h.element("turn").textContent, /^Ход 2 /);
  selectOrder(); h.click("goBtn"); h.click("moreBtn"); h.click("restartBtn");
  const resetHp = [h.element("hpMe").textContent, h.element("hpEn").textContent], logs = h.calls.filter(c => c.name === "log.add").length;
  h.clock.advance(3000);
  assert.deepEqual([h.element("hpMe").textContent, h.element("hpEn").textContent], resetHp);
  assert.equal(h.calls.filter(c => c.name === "log.add").length, logs, "old fight cannot append a late attack");
  assert.match(h.element("turn").textContent, /^Ход 1 /);
  selectOrder(); h.click("goBtn"); h.click("baseInfoBtn"); const callsAtExit = h.calls.length; h.clock.advance(3000);
  assert.equal(h.element("app").hidden, true); assert.equal(h.calls.slice(callsAtExit).some(c => ["arena.hit", "log.add", "audio.newFight"].includes(c.name)), false);
}));

test("time links choose matching art and rules before the first fight, overriding saved time", async () => {
  for (const time of ["morning", "day", "evening", "night"]) {
    await harness(async h => {
      assert.equal(h.duel().match.config.time, time);
      assert.equal((h.calls.find(c => c.name === "arena.frame")!.args[0] as { time: string }).time, time);
      const start = h.battle!.start(starterLoadout("rifle"), () => {}, async () => {}, undefined);
      await h.ready(); await start;
      const config = h.duel().match.config;
      const rendered = h.calls.filter(c => c.name === "arena.conditions").at(-1)!.args[0] as { config: typeof config };
      assert.deepEqual(rendered.config, config, "renderer receives the actual match, including sun and gear");
      assert.equal(config.time, time);
      assert.ok(h.element("condText").textContent);
    }, { search: `?muted=1&time=${time}`, time: "night" });
  }
});

// A UTC hour boundary: the salt hall reads 00:00 (night), the blocks run 12 game hours ahead (day).
const DAY = 3_600_000, MIDNIGHT = 1_800_000_000_000, GAME_HOUR = DAY / 24;
const world = (now: number, map: string) => callRust<{ time: string; weather: string; clock: string; open: boolean }>("client.world.at", [now, map]);
// The first such midnight, a game minute in, when the clock leaves all these maps open.
const openAt = (...maps: string[]) => {
  for (let day = 0; ; day++) {
    const now = MIDNIGHT + day * DAY + GAME_HOUR / 60;
    if (maps.every(map => world(now, map).open)) return now;
  }
};

test("the fight takes its time and weather from the map's game clock", async () => harness(async h => {
  const now = h.now, kit = starterLoadout("rifle");
  assert.equal(h.duel().match.config.time, "night");
  assert.equal(h.duel().match.config.weather, world(now, "saltHall").weather);
  const start = h.battle!.start(kit, () => {}, async () => {}, undefined); await h.ready(); await start;
  h.click("moreBtn");
  assert.match(h.element("clockNote").textContent, /^Сейчас 00:01, ночь/);
  h.click("restartBtn");
  assert.equal(h.duel().match.config.time, "night");
  // The day goes on: an hour of real time later is morning, and a new fight starts there.
  h.now = now + 6 * GAME_HOUR;
  h.click("moreBtn"); h.click("restartBtn");
  assert.equal(h.duel().match.config.time, "morning");
  assert.equal(h.duel().match.config.weather, world(h.now, "saltHall").weather);
  assert.deepEqual(h.duel().fighters[0].kit, kit);
}, { now: MIDNIGHT + GAME_HOUR / 60, time: "evening" }));

test("blocks links choose all four real time variants on the correct map dimensions", async () => {
  for (const time of ["morning", "day", "evening", "night"]) {
    await harness(async h => {
      assert.equal(h.duel().match.config.mapId, "blocks");
      assert.equal(h.duel().match.config.time, time);
      const frame = h.calls.find(c => c.name === "arena.frame")!.args[0] as { id: string; mapId: string; time: string };
      assert.deepEqual([frame.id, frame.mapId, frame.time], [`blocks-${time}`, "blocks", time]);
      assert.equal(h.duel().match.map.maxDistance, 12);
      assert.equal(h.duel().match.distance, 7);
    }, { search: `?muted=1&map=blocks&time=${time}`, time: "night", map: "saltHall" });
  }
});

test("map changes preserve the fitted kit and move to that map's own hour", async () => harness(async h => {
  const kit = { ...starterLoadout("rifle"), head: "head-sych" };
  const start = h.battle!.start(kit, () => {}, async () => {}, undefined); await h.ready(); await start;
  h.click("moreBtn");
  assert.equal(h.element("mapSeg").children.length, 2);
  assert.match(h.element("mapSeg").children[1]!.innerHTML, /12:0\d · день/);
  const first = h.duel();
  h.element("mapSeg").children[1]!.click();
  assert.notEqual(h.duel(), first);
  assert.deepEqual(h.duel().fighters[0].kit, kit);
  assert.equal(h.duel().match.fighters[0].gear.nvg, true);
  assert.equal(h.duel().match.config.mapId, "blocks");
  assert.equal(h.duel().match.config.time, "day");
  assert.equal(h.duel().match.config.weather, world(h.now, "blocks").weather);
  assert.equal(h.duel().match.map.maxDistance, 12);
  assert.equal(h.stored.get("cd-map"), "blocks");
  h.click("moreBtn");
  assert.equal(h.element("mapSeg").children[1]!.getAttribute("aria-pressed"), "true");
  h.element("mapSeg").children[0]!.click();
  assert.equal(h.duel().match.config.mapId, "saltHall");
  assert.equal(h.duel().match.config.time, "night");
  h.click("moreBtn");
  assert.equal(h.element("regionSeg").children.length, 2);
  assert.equal(h.element("regionSeg").children[0]!.getAttribute("aria-pressed"), "true");
  h.element("regionSeg").children[1]!.click();
  assert.equal(h.duel().match.config.mapId, "dam");
  assert.equal(h.duel().match.config.time, "morning", "the dam runs six game hours ahead");
  assert.equal(h.duel().match.map.maxDistance, 14);
  assert.deepEqual(h.duel().fighters[0].kit, kit);
  h.click("moreBtn");
  assert.equal(h.element("mapSeg").children.length, 2, "only the chosen region's maps are listed");
  h.element("mapSeg").children[1]!.click();
  assert.equal(h.duel().match.config.mapId, "dryBed");
  assert.equal(h.duel().match.config.time, "evening", "the dry bed runs eighteen game hours ahead");
  assert.equal(h.duel().match.map.maxDistance, 13);
  assert.equal(h.stored.get("cd-map"), "dryBed");
}, { now: openAt("blocks", "dryBed") }));

test("menu time and weather pickers override the clock until it is chosen again", async () => harness(async h => {
  const kit = { ...starterLoadout("rifle"), head: "head-sych" };
  const start = h.battle!.start(kit, () => {}, async () => {}, undefined); await h.ready(); await start;
  assert.equal(h.duel().match.config.time, "night");
  h.click("moreBtn");
  const names = (seg: string) => [...h.element(seg).children].map(b => b.innerHTML);
  assert.deepEqual(names("timeSeg"), ["По часам", "Утро", "День", "Вечер", "Ночь"]);
  assert.deepEqual(names("weatherSeg"), ["По часам", "Ясно", "Туман", "Пыль", "Гроза"]);
  assert.equal(h.element("timeSeg").children[0]!.getAttribute("aria-pressed"), "true");
  const first = h.duel();
  h.element("timeSeg").children[2]!.click();
  assert.notEqual(h.duel(), first, "a pick starts a new fight");
  assert.deepEqual(h.duel().fighters[0].kit, kit);
  assert.equal(h.duel().match.config.time, "day");
  assert.equal(h.stored.get("cd-test-time"), "day");
  h.click("moreBtn");
  h.element("weatherSeg").children[4]!.click();
  assert.equal(h.duel().match.config.weather, "storm");
  assert.equal(h.duel().match.config.time, "day", "the time pick holds");
  h.click("moreBtn");
  assert.equal(h.element("weatherSeg").children[4]!.getAttribute("aria-pressed"), "true");
  assert.match(h.element("clockNote").textContent, /отключены/);
  h.element("mapSeg").children[1]!.click();
  assert.equal(h.duel().match.config.mapId, "blocks");
  assert.equal(h.duel().match.config.weather, "storm", "picks follow to another map");
  h.click("moreBtn");
  h.element("timeSeg").children[0]!.click();
  h.click("moreBtn");
  h.element("weatherSeg").children[0]!.click();
  assert.equal(h.stored.get("cd-test-time"), "");
  assert.equal(h.duel().match.config.weather, world(h.now, "blocks").weather);
  h.click("moreBtn");
  assert.doesNotMatch(h.element("clockNote").textContent, /отключены/);
}, { now: MIDNIGHT + GAME_HOUR / 60 }));

test("saved blocks map is restored at its own hour", async () => harness(async h => {
  assert.equal(h.duel().match.config.mapId, "blocks");
  assert.equal(h.duel().match.config.time, "day");
}, { map: "blocks", time: "night", now: openAt("blocks") }));
