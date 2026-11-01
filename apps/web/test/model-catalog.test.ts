import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import test from "node:test";
import { ITEMS, compileKit, getItem, starterLoadout } from "@crowned-dogs/core";
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
});

test("available equipment resolves to packaged models and existing mesh prefixes", () => {
  type Gltf = { nodes?: { name?: string; mesh?: number; children?: number[] }[] };
  const parsed = new Map<string, Gltf>();
  for (const item of ITEMS.filter(item => item.kind !== "mod" && availableInFitting(item))) {
    const spec = ITEM_MODELS[item.id];
    assert.ok(spec, `${item.id}: available equipment needs a preview model`);
    const [file, prefix] = spec.split("#");
    let gltf = parsed.get(file!);
    if (!gltf) {
      const path = new URL(`../public/preview/${file}.json`, import.meta.url);
      assert.ok(existsSync(path), `${item.id}: missing preview/${file}.json`);
      const wrapper = JSON.parse(readFileSync(path, "utf8")) as { data: string };
      const bytes = Buffer.from(wrapper.data, "base64");
      assert.equal(bytes.toString("ascii", 0, 4), "glTF", `${file}: valid GLB header`);
      assert.equal(bytes.readUInt32LE(4), 2, `${file}: supported GLB version`);
      assert.equal(bytes.readUInt32LE(8), bytes.length, `${file}: complete model payload`);
      assert.equal(bytes.readUInt32LE(16), 0x4e4f534a, `${file}: GLB JSON chunk`);
      gltf = JSON.parse(bytes.subarray(20, 20 + bytes.readUInt32LE(12)).toString("utf8")) as Gltf;
      parsed.set(file!, gltf);
    }
    if (prefix) {
      assert.ok(gltf.nodes?.some(node => node.name?.startsWith(prefix) && (node.mesh !== undefined || node.children?.length)), `${item.id}: no mesh branch named ${prefix} in ${file}`);
    }
  }
});
