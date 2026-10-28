import "./style.css";
import "./base/base.css";
import { BaseScreen } from "./base/base-screen.ts";
import { loadLocalLoadout } from "./base/loadout-state.ts";

const root = document.createElement("main");
root.id = "base-app";
document.body.prepend(root);
let inBattle = false;
const base = new BaseScreen(root, async (loadout) => {
  const battle = await import("./main.ts");
  battle.startBattle(loadout, () => {
    inBattle = false;
    history.replaceState(null, "", "#base");
    base.show("base"); base.setActive(true);
    root.querySelector<HTMLButtonElement>(".cd-enter")?.focus();
  });
  inBattle = true;
  base.setActive(false);
  history.replaceState(null, "", "#battle");
});
base.show(location.hash === "#fitting" ? "fitting" : "base");
window.addEventListener("hashchange", () => {
  if (!inBattle) base.show(location.hash === "#fitting" ? "fitting" : "base");
});
document.addEventListener("visibilitychange", () => {
  if (!inBattle) base.setActive(!document.hidden);
});
window.addEventListener("pagehide", () => base.setActive(false));
window.addEventListener("pageshow", () => { if (!inBattle) base.setActive(true); });

// Preserve the existing explicit arena-calibration entry point.
if (/[?&]cal\b/.test(location.search)) {
  void import("./main.ts").then((battle) => {
    battle.startBattle(loadLocalLoadout(), () => { inBattle = false; base.show("base"); base.setActive(true); });
    inBattle = true; base.setActive(false);
  });
}
