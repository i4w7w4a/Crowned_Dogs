import assert from "node:assert/strict";
import { test } from "node:test";
import * as THREE from "three";
import { sceneLoadGuns } from "../../../packages/runtime/src/index.ts";

const files: Record<string, string> = {
  pistol: "pustelga", shotgun: "taran", rifle: "varyag", sniper: "berkut", knife: "knife", grenade: "grenade",
};

function host(requests: string[]) {
  return {
    loadModel: async (url: string) => {
      requests.push(url);
      return { scene: new THREE.Group() };
    },
  };
}

test("priority gun loading waits only for the fighters' classes, knife and grenade", async () => {
  const requests: string[] = [];
  const guns = await sceneLoadGuns(host(requests), ["rifle", "sniper", "knife", "grenade"]);
  const expected = ["models/varyag.json", "models/berkut.json", "models/knife.json", "models/grenade.json"];
  assert.deepEqual(requests.sort(), expected.sort());
  assert.deepEqual(Object.keys(guns).sort(), ["grenade", "knife", "rifle", "sniper"]);
});

test("gun loading without a priority retains the complete QA set", async () => {
  const requests: string[] = [];
  const guns = await sceneLoadGuns(host(requests));
  assert.deepEqual(requests.sort(), Object.values(files).map(name => `models/${name}.json`).sort());
  assert.deepEqual(Object.keys(guns).sort(), Object.keys(files).sort());
});
