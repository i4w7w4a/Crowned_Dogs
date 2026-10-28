import "./style.css";
import "./base/base.css";
import { BaseScreen, type BaseView } from "./base/base-screen.ts";
import { loadLocalLoadout, type Loadout } from "./base/loadout-state.ts";
import { MawLoader } from "./loading/maw-loader.ts";
import { runBattleLoading } from "./loading/battle-loading.ts";

const root = document.createElement("main");
root.id = "base-app";
document.body.prepend(root);
const loader = new MawLoader();
let inBattle = false;
let loading = false;
const base = new BaseScreen(root, enterBattle);

function returnToBase(): void {
  inBattle = false;
  history.replaceState(null, "", "#base");
  base.show("base"); base.setActive(true);
  root.querySelector<HTMLButtonElement>(".cd-enter")?.focus();
}

async function enterBattle(loadout: Loadout): Promise<void> {
  if (loading || inBattle) return;
  loading = true;
  base.setActive(false);
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
base.show(viewFromHash());
window.addEventListener("hashchange", () => {
  if (!inBattle && !loading) base.show(viewFromHash());
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
