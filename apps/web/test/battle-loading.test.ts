import assert from "node:assert/strict";
import test from "node:test";
import { activateAfterReady, runBattleLoading, type LoadingPresentation } from "../src/loading/battle-loading.ts";

function deferred() {
  let resolve!: () => void;
  const promise = new Promise<void>((done) => { resolve = done; });
  return { promise, resolve };
}

test("a slow arena cannot start the timer until both the arena and loader exit are ready", async () => {
  const model = deferred(), fade = deferred(), events: string[] = [];
  const started = activateAfterReady(() => model.promise, () => { events.push("ready"); return fade.promise; }, () => { events.push("start timer"); });
  await Promise.resolve();
  assert.deepEqual(events, []);
  model.resolve(); await Promise.resolve();
  assert.deepEqual(events, ["ready"]);
  fade.resolve(); await started;
  assert.deepEqual(events, ["ready", "start timer"]);
});

test("failed arena preparation never runs the exit animation or enables the fight", async () => {
  let entered = false, faded = false;
  await assert.rejects(activateAfterReady(
    () => Promise.reject(new Error("offline")), async () => { faded = true; }, () => { entered = true; },
  ), /offline/);
  assert.equal(entered, false); assert.equal(faded, false);
});

test("leaving during a download prevents a late completed arena from starting", async () => {
  const model = deferred(), controller = new AbortController();
  let entered = false;
  const started = activateAfterReady(() => model.promise, async () => {}, () => { entered = true; }, controller.signal);
  controller.abort(); model.resolve();
  await assert.rejects(started, { name: "AbortError" });
  assert.equal(entered, false);
});

test("leaving during the loader exit prevents battle activation", async () => {
  const fade = deferred(), controller = new AbortController();
  let entered = false;
  const started = activateAfterReady(async () => {}, () => fade.promise, () => { entered = true; }, controller.signal);
  await Promise.resolve(); controller.abort(); fade.resolve();
  await assert.rejects(started, { name: "AbortError" });
  assert.equal(entered, false);
});

test("retry prepares the arena again and dismisses the loader only after success", async () => {
  let attempts = 0;
  const events: string[] = [];
  const view: LoadingPresentation = {
    show: () => { events.push("show"); },
    ready: async () => { events.push("ready"); },
    failed: async () => { events.push("failed"); return "retry"; },
    hide: () => { events.push("hide"); },
  };
  const result = await runBattleLoading(async (ready, signal) => activateAfterReady(async () => {
    attempts += 1; if (attempts === 1) throw new Error("offline");
  }, ready, () => { events.push("entered"); }, signal), view);
  assert.equal(result, true);
  assert.equal(attempts, 2);
  assert.deepEqual(events, ["show", "failed", "show", "ready", "entered", "hide"]);
});

test("back cancels immediately while a stalled model request is still pending", async () => {
  const model = deferred();
  let back!: () => void, hidden = false, entered = false;
  const view: LoadingPresentation = {
    show: (onBack) => { back = onBack; }, ready: async () => {},
    failed: async () => { throw new Error("A cancellation must not show an error"); },
    hide: () => { hidden = true; },
  };
  const result = runBattleLoading((ready, signal) => activateAfterReady(() => model.promise, ready, () => { entered = true; }, signal), view);
  back();
  assert.equal(await result, false);
  assert.equal(hidden, true);
  model.resolve(); await Promise.resolve();
  assert.equal(entered, false);
});

test("back from the error screen leaves the base available without retrying", async () => {
  let attempts = 0, hidden = false;
  const view: LoadingPresentation = {
    show: () => {}, ready: async () => {}, failed: async () => "back", hide: () => { hidden = true; },
  };
  assert.equal(await runBattleLoading(async () => { attempts++; throw new Error("bad model"); }, view), false);
  assert.equal(attempts, 1); assert.equal(hidden, true);
});
