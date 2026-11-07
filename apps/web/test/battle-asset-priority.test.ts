import assert from "node:assert/strict";
import { test } from "node:test";
import * as THREE from "three";
import { RustSceneHand, sceneLoadGuns } from "../../../packages/runtime/src/index.ts";

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

test("a late real gun replaces its placeholder in an existing Hand without losing mods", () => {
  const model = new THREE.Group();
  model.add(new THREE.Group());
  model.children[0]!.name = "hand_r";
  const material = new THREE.MeshStandardMaterial();
  const guns: Record<string, THREE.Object3D> = {};
  const hand = new RustSceneHand({ THREE }, model, new THREE.Matrix4(), material, { guns });
  hand.fit({ rail: "rail_kulak" });
  hand.hold("rifle");
  const rifle = hand.mount.children[1]!;
  assert.equal(rifle.visible, true);
  assert.equal(rifle.getObjectByName("RealRifle"), undefined);

  const real = new THREE.Group();
  real.name = "RealRifle";
  const attachment = new THREE.Mesh(new THREE.BoxGeometry(), material);
  attachment.name = "rail_kulak";
  attachment.userData = { factory: false, slot: "rail" };
  real.add(attachment);
  guns.rifle = real;
  hand.upgradeGuns();

  assert.ok(rifle.getObjectByName("RealRifle"), "the existing group receives the late model");
  assert.equal(rifle.visible, true, "the currently held class stays visible");
  assert.equal(rifle.getObjectByName("rail_kulak")!.visible, true, "the fitted mod survives the replacement");
});
