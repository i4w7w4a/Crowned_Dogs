/** Dev-only GPU review with real renderer, assets, catalogue and deterministic rules. No audio is imported. */
import { createMatch, kitFighter, starterLoadout, validateLoadout, weaponSpec, hitChance, resolveTurn, guardShare, type Match, type TimeOfDay, type Weather, type Loadout, type Order } from "@crowned-dogs/core";
import { Arena } from "./arena.ts";
import { callRust } from "../../../../packages/runtime/src/index.ts";
import { FRAMES, type Frame } from "./frames.ts";
import { QUALITY, type QualityId } from "./quality.ts";

const $ = <T extends HTMLElement>(id: string) => document.getElementById(id) as T;
const qa = $("qa"), status = $("status"), report = $("report"), errors = $("errors");
const canvas = $<HTMLCanvasElement>("gl"), plate = $<HTMLImageElement>("plate");
const map = $<HTMLSelectElement>("map");
const kit = $<HTMLSelectElement>("kit"), quality = $<HTMLSelectElement>("quality"), fp = $<HTMLInputElement>("fp");
const controls = [...document.querySelectorAll<HTMLInputElement | HTMLSelectElement | HTMLButtonElement>("button,input,select")];
const query = new URLSearchParams(location.search);
const reviewSeed = query.get("seed") ?? "battle-times-gpu-review";
const WEATHERS: Weather[] = ["clear", "dust", "storm", "fog"];
const reviewWeather: Weather = WEATHERS.find(w => w === query.get("weather")) ?? "clear";
// ?foe=pistol|shotgun|sniper arms the enemy with another gun; the rifle is the default.
const foeGun = (["pistol", "shotgun", "sniper"] as const).find(g => g === query.get("foe")) ?? "rifle";
const reviewURL = new URL(location.href);
reviewURL.searchParams.set("muted", "1"); reviewURL.searchParams.set("motionDebug", "1");
history.replaceState(null, "", reviewURL);
const maps = callRust<{ id: string; name: string }[]>("client.conditions.maps", [FRAMES]);
for (const choice of maps) map.add(new Option(choice.name, choice.id));
let chosen = callRust<Frame>("client.conditions.initial", [FRAMES, query.get("time"), query.get("frame"), null, false, query.get("map"), null]);
let time: TimeOfDay = chosen.time;
map.value = chosen.mapId;
if ([...kit.options].some(option => option.value === query.get("kit"))) kit.value = query.get("kit")!;
if (query.get("q") && query.get("q")! in QUALITY) quality.value = query.get("q")!;
let closed = false;
const events = new AbortController();
const eventOptions = { signal: events.signal };

function fail(reason: unknown) {
  if (closed) return;
  const message = reason instanceof Error ? `${reason.message}\n${reason.stack ?? ""}` : String(reason);
  errors.hidden = false; errors.textContent += `${message.slice(0, 6000)}\n`;
  qa.dataset.errorCount = String(Number(qa.dataset.errorCount ?? 0) + 1);
  qa.dataset.ready = "false"; status.textContent = "Ошибка: см. отчёт ниже";
}
const previousError = console.error;
const reviewError = (...args: unknown[]) => { fail(args.map(String).join(" ")); previousError.apply(console, args); };
console.error = reviewError;
window.addEventListener("unhandledrejection", event => fail(event.reason), eventOptions);

const arena = new Arena(canvas);
// Lets a review script play any clip, e.g. reviewArena.attack("enemy", "knife", 0).
Object.assign(window, { reviewArena: arena });
let match: Match;
let generation = 0;
let modelsReady = false;
function availableTimes() {
  const choices = callRust<{ id: TimeOfDay }[]>("client.conditions.choices", [FRAMES, chosen.mapId]);
  document.querySelectorAll<HTMLInputElement>("input[name=time]").forEach(input => {
    const available = choices.some(choice => choice.id === input.value);
    input.closest("label")!.hidden = !available;
    input.disabled = !available || qa.dataset.ready !== "true";
    input.checked = input.value === time;
  });
}
function loadout(): Loadout {
  const value = starterLoadout("rifle");
  if (kit.value.includes("nvg")) value.head = "head-sych";
  if (kit.value.includes("suppressed")) value.weapon1 = { item: "rifle-varyag", mods: { muzzle: "muzzle-tishina" } };
  const validation = validateLoadout(value);
  if (validation) throw new Error(`Invalid review loadout: ${JSON.stringify(validation)}`);
  return value;
}
function resize() { arena.resize(canvas.clientWidth, canvas.clientHeight); }
const observer = new ResizeObserver(resize);
observer.observe($("viewport"));

function describe() {
  const weapon = weaponSpec(match, 0, match.fighters[0].weapon);
  qa.dataset.mapId = match.config.mapId;
  qa.dataset.maxDistance = String(match.map.maxDistance);
  qa.dataset.frameId = chosen.id;
  qa.dataset.time = match.config.time;
  qa.dataset.kit = kit.value;
  qa.dataset.nightVision = String(match.config.time === "night" && !!match.fighters[0].gear.nvg);
  qa.dataset.suppressed = String(weapon.suppressor);
  report.textContent = JSON.stringify({
    mapId: match.config.mapId, mapName: match.map.name, frameId: chosen.id, maxDistance: match.map.maxDistance,
    time: match.config.time, weather: match.config.weather, turn: match.turn, distance: match.distance,
    nvgEquipped: !!match.fighters[0].gear.nvg,
    nvgActive: match.config.time === "night" && !!match.fighters[0].gear.nvg,
    suppressor: weapon.suppressor, facesSun: match.fighters[0].facesSun,
    hitChance10: hitChance(match, 0, "rifle", 10, 1, false),
    shield: guardShare(match, 0, "rifle"), revealedByFlash: match.fighters[0].flash,
    gpuErrors: Number(qa.dataset.errorCount),
  }, null, 2);
}
async function reset() {
  const request = ++generation;
  controls.forEach(button => { button.disabled = true; });
  qa.dataset.ready = "false"; status.textContent = "Загружаю выбранную карту…";
  chosen = callRust<Frame>("client.conditions.select", [FRAMES, { mapId: map.value, time, weather: "clear" }]);
  map.value = chosen.mapId; time = chosen.time;
  availableTimes();
  const nextURL = new URL(location.href);
  nextURL.searchParams.set("map", chosen.mapId); nextURL.searchParams.set("time", chosen.time);
  history.replaceState(null, "", nextURL);
  const own = loadout();
  match = createMatch({
    seed: reviewSeed, mapId: chosen.mapId, weather: reviewWeather, time: chosen.time,
    fighters: [kitFighter("qa-me", "Оператор", own), kitFighter("qa-enemy", "Искра", starterLoadout(foeGun))], startDistance: 7,
  });
  plate.src = quality.value === "low" ? chosen.src : chosen.src2x ?? chosen.src;
  plate.alt = chosen.name;
  arena.setFrame(chosen); arena.setQuality(QUALITY[quality.value as QualityId]); arena.setFirstPerson(fp.checked);
  resize(); arena.resume();
  if (!modelsReady) {
    // ?me=female plays the female Persona against the male one, as the game does for a female player.
    const male = "models/persona-male.json", female = "models/persona-female.json";
    if (query.get("me") === "female") await arena.load(female, male, 7); else await arena.load(male, female, 7);
    modelsReady = true;
  }
  if (request !== generation) return;
  arena.arm("me", "rifle", own.weapon1.mods ?? {}); arena.arm("enemy", foeGun, {});
  arena.reset(match.distance); arena.setConditions(match);
  await arena.warm();
  if (request !== generation) return;
  await arena.drawn();
  if (chosen.projectedStage) await new Promise<void>((resolve, reject) => {
    const deadline = performance.now() + 30_000;
    const check = () => {
      if (request !== generation) { resolve(); return; }
      if (canvas.dataset.projectedStage === "true") { resolve(); return; }
      if (performance.now() > deadline) { reject(new Error("Projected variant was not ready in 30 seconds")); return; }
      requestAnimationFrame(check);
    }; check();
  });
  if (request !== generation) return;
  describe();
  if (Number(qa.dataset.errorCount) === 0) {
    qa.dataset.ready = "true"; status.textContent = "Готово · сцена отрисована на GPU · без звука";
    controls.forEach(button => { button.disabled = false; });
    availableTimes();
  }
}
function turn(move: 0 | 2) {
  if (match.result) { void reset().catch(fail); return; }
  const ours: Order = move === 0 ? { action: { type: "shoot", move: 0, zone: "body" }, defend: ["head", "body"] } : { action: { type: "run", move: 2 }, defend: ["head", "body"] };
  const theirs: Order = { action: { type: "shoot", move: 0, zone: "body" }, defend: ["head", "body"] };
  match = resolveTurn(match, [ours, theirs]).match;
  arena.setConditions(match);
  if (move) arena.move(match.distance, move, 0); else arena.attack("me", "rifle", .05);
  describe();
}
document.querySelectorAll<HTMLInputElement>("input[name=time]").forEach(input => input.addEventListener("change", () => { if (input.checked) { time = input.value as TimeOfDay; void reset().catch(fail); } }, eventOptions));
map.addEventListener("change", () => { void reset().catch(fail); }, eventOptions);
kit.addEventListener("change", () => { void reset().catch(fail); }, eventOptions);
quality.addEventListener("change", () => { void reset().catch(fail); }, eventOptions);
fp.addEventListener("change", () => { arena.setFirstPerson(fp.checked); arena.setConditions(match); }, eventOptions);
$("shot").addEventListener("click", () => { try { turn(0); } catch (error) { fail(error); } }, eventOptions);
$("step").addEventListener("click", () => { try { turn(2); } catch (error) { fail(error); } }, eventOptions);
$("reset").addEventListener("click", () => { void reset().catch(fail); }, eventOptions);
function cleanup() {
  if (closed) return;
  closed = true; generation++;
  observer.disconnect(); events.abort();
  if (console.error === reviewError) console.error = previousError;
  arena.dispose(); arena.free();
}
window.addEventListener("pagehide", cleanup, { once: true, ...eventOptions });
import.meta.hot?.dispose(cleanup);
void reset().catch(fail);
