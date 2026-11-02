import "./style.css";
import "./base/base.css";
import { BaseScreen, type BaseView } from "./base/base-screen.ts";
import { loadLocalLoadout, type Loadout } from "./base/loadout-state.ts";
import { MawLoader } from "./loading/maw-loader.ts";
import { runBattleLoading } from "./loading/battle-loading.ts";
import { gameAudio } from "./audio/arena-audio.ts";
import { mountSoundSettings, showSoundSettings } from "./base/sound-settings.ts";

const root = document.createElement("main");
root.id = "base-app";
document.body.prepend(root);
const loader = new MawLoader();
let inBattle = false;
let loading = false;
const base = new BaseScreen(root, enterBattle);
mountSoundSettings();
// browsers start sound only after a tap; iOS counts touchend and click, not pointerdown, so try on each
for (const e of ["pointerdown", "touchend", "click", "keydown"]) document.addEventListener(e, () => gameAudio.unlock(), { passive: true });

/** A base screen with its own radio songs and the settings button. */
function showBase(view: BaseView): void {
  base.show(view);
  showSoundSettings(true);
}

function returnToBase(): void {
  inBattle = false;
  history.replaceState(null, "", "#base");
  showBase("base"); base.setActive(true);
  root.querySelector<HTMLButtonElement>(".cd-enter")?.focus();
}

async function enterBattle(loadout: Loadout): Promise<void> {
  if (loading || inBattle) return;
  loading = true;
  base.setActive(false);
  showSoundSettings(false);
  try {
    const entered = await runBattleLoading(async (beforeEnter, signal) => {
      const battle = await import("./main.ts");
      signal.throwIfAborted();
      await battle.startBattle(loadout, returnToBase, beforeEnter, signal);
    }, loader);
    if (entered) {
      inBattle = true;
      history.replaceState(null, "", "#battle");
    } else returnToBase();
  } finally { loading = false; }
}

const viewFromHash = (): BaseView => location.hash === "#weapon" ? "weapon" : location.hash === "#fitting" ? "fitting" : "base";
showBase(viewFromHash());
window.addEventListener("hashchange", () => {
  if (!inBattle && !loading) showBase(viewFromHash());
});
document.addEventListener("visibilitychange", () => {
  if (!inBattle && !loading) base.setActive(!document.hidden);
});
window.addEventListener("pagehide", () => base.setActive(false));
window.addEventListener("pageshow", () => { if (!inBattle && !loading) base.setActive(true); });

// Preserve the existing explicit arena-calibration entry point.
if (/[?&]cal\b/.test(location.search)) {
  void enterBattle(loadLocalLoadout());
}
