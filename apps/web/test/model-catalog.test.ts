import assert from "node:assert/strict";
import { existsSync } from "node:fs";
import test from "node:test";
import { compileKit, getItem, starterLoadout } from "@crowned-dogs/core";
import { ITEM_MODELS, availableInFitting } from "../src/base/model-catalog.ts";
import { equipItem, loadLocalLoadout, saveLocalLoadout } from "../src/base/loadout-state.ts";
import { LocalDuel } from "../src/game/local-duel.ts";

test("the approved Sych placeholder is selectable, previewable and retains NVG after saving and entering battle", () => {
  assert.equal(availableInFitting(getItem("head-sych")), true);
  assert.equal(ITEM_MODELS["head-sych"], ITEM_MODELS["head-k6"]);
  assert.equal(ITEM_MODELS["head-sych"], "tripo-helmet");
  assert.ok(existsSync(new URL(`../public/preview/${ITEM_MODELS["head-sych"]}.json`, import.meta.url)));
  const kit = equipItem(starterLoadout("rifle"), "head", "head-sych");
  const values = new Map<string, string>();
  const storage = { getItem: (key: string) => values.get(key) ?? null, setItem: (key: string, value: string) => { values.set(key, value); } };
  assert.equal(saveLocalLoadout(kit, storage), true);
  const restored = loadLocalLoadout(storage);
  assert.equal(restored.head, "head-sych");
  assert.equal(compileKit(restored).nvg, true);
  const duel = new LocalDuel({ seed: "sych-placeholder", mapId: "saltHall", time: "night", weather: "clear",
    player: { name: "Тень", weapon: "rifle", kit: restored }, bot: { name: "Искра", weapon: "rifle" } });
  assert.equal(duel.match.fighters[0].gear.nvg, true);
  for (const id of ["head-cap", "head-sfera", "head-marevo", "head-filin", "body-light", "body-plita4", "body-bastion", "suit-leshy", "suit-ryvok", "suit-ten", "suit-solonchak"]) {
    assert.equal(availableInFitting(getItem(id)), false, id);
    assert.equal(ITEM_MODELS[id], undefined, id);
  }
});
