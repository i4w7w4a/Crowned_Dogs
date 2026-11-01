import assert from "node:assert/strict";
import { test } from "node:test";
import * as THREE from "three";
import { callRust, sceneApplyLook, sceneDress, sceneSetBeardFirstPerson } from "../../../packages/runtime/src/index.ts";

const styles = ["boxed", "goatee", "full", "tapered", "stubble"];
const faces = ["m1", "m2", "m3", "m4"];
function look(extra: Record<string, unknown> = {}) {
  return callRust<Record<string, unknown>>("client.look.normalize", [{ sex: "male", callsign: "Сокол", tag: "K-053-243", ...extra }]);
}
function fixture() {
  const root = new THREE.Group();
  // A shared atlas/material must not cause another fitted variant to disappear.
  const cards = new THREE.MeshStandardMaterial({ color: 0x674d3a, map: new THREE.Texture() });
  cards.name = "M_Beard_Cards";
  const base = new THREE.MeshStandardMaterial({ color: 0x674d3a });
  base.name = "M_Beard_Root";
  for (const face of faces) for (const style of styles) for (const part of ["cards", "root"]) {
    const mesh = new THREE.Mesh(new THREE.BufferGeometry(), part === "cards" ? cards : base);
    mesh.name = `Beard_${face}_${style}_${part}`;
    root.add(mesh);
  }
  return { root, cards, base };
}

test("every beard selects only its fitted face, with shared materials and safe skin/hair picks", () => {
  const { root, cards } = fixture();
  for (const face of faces) for (const beard of styles) {
    sceneApplyLook(root, look({ face, beard, skin: "deep", hairColor: "grey" }));
    assert.deepEqual(root.children.filter(n => n.visible).map(n => n.name), [`Beard_${face}_${beard}_cards`, `Beard_${face}_${beard}_root`]);
    assert.equal(cards.visible, true);
    assert.equal(cards.map!.wrapS, THREE.ClampToEdgeWrapping);
  }
  sceneApplyLook(root, look());
  assert.deepEqual(root.children.filter(n => n.visible).map(n => n.name), ["Beard_m1_stubble_cards", "Beard_m1_stubble_root"], "the default man wears light stubble");
  sceneApplyLook(root, look({ sex: "female", beard: "full" }));
  assert.ok(root.children.every(n => !n.visible), "female cannot inherit beard meshes");
});

test("independent beard tint is idempotent and gear occlusion preserves the saved choice", () => {
  const { root, cards, base } = fixture();
  const chosen = look({ face: "m3", beard: "tapered", beardColor: "grey", hairColor: "black" });
  sceneApplyLook(root, chosen);
  const color = cards.color.toArray();
  sceneApplyLook(root, chosen);
  assert.deepEqual(cards.color.toArray(), color, "shared materials are not multiplied repeatedly");
  assert.deepEqual(base.color.toArray(), color);
  root.userData.cdBeardOccluded = true;
  sceneApplyLook(root, chosen);
  assert.ok(root.children.every(n => !n.visible));
  assert.equal(chosen.beard, "tapered");
  root.userData.cdBeardOccluded = false;
  sceneApplyLook(root, chosen);
  assert.equal(root.children.filter(n => n.visible).length, 2);
  root.userData.cdBeardFirstPerson = true;
  sceneApplyLook(root, chosen);
  assert.ok(root.children.every(n => !n.visible), "plain beard meshes stay hidden from the first-person camera");
  root.userData.cdBeardFirstPerson = false;
  sceneApplyLook(root, chosen);
  assert.equal(root.children.filter(n => n.visible).length, 2);
});

test("v1 saved Persona gains beard defaults without losing identity or appearance", () => {
  const expected = look({ face: "m4", skin: "olive", eyes: "gold", suit: "navy" });
  const legacy = { ...expected };
  delete legacy.beard; delete legacy.beardColor;
  const migrated = callRust("client.look.load", [JSON.stringify({ version: 1, look: legacy })]);
  assert.deepEqual(migrated, expected);
  const stored = callRust<{ version: number }>("client.look.save", [migrated]);
  assert.equal(stored.version, 2);
  assert.deepEqual(callRust("client.look.load", [JSON.stringify(stored)]), expected);
});

test("actual closed helmet hides the beard, open helmets and hidden alternate gear do not", () => {
  const { root } = fixture();
  const chosen = look({ beard: "boxed" });
  const gear = new THREE.Group();
  const shell = new THREE.Mesh(new THREE.BufferGeometry(), new THREE.MeshStandardMaterial());
  shell.name = "Sfera";
  gear.add(shell); root.add(gear);
  const visibleBeards = () => root.children.filter(n => n.name.startsWith("Beard_") && n.visible);
  sceneApplyLook(root, chosen);
  assert.equal(visibleBeards().length, 0);
  shell.visible = false;
  sceneApplyLook(root, chosen);
  assert.equal(visibleBeards().length, 2);
  shell.visible = true; gear.visible = false;
  sceneApplyLook(root, chosen);
  assert.equal(visibleBeards().length, 2, "hidden alternate kit cannot suppress facial hair");
  gear.visible = true; shell.name = "Helmet";
  sceneApplyLook(root, chosen);
  assert.equal(visibleBeards().length, 2, "open K6 helmet leaves the beard visible");
  shell.name = "Cap";
  sceneApplyLook(root, chosen);
  assert.equal(visibleBeards().length, 2);
  assert.equal(chosen.beard, "boxed");
});

test("strict v2 rejects corrupt beard data and female normalization never exposes male choices", () => {
  const saved = (value: unknown, version = 2) => callRust("client.look.load", [JSON.stringify({ version, look: value })]);
  assert.equal(saved({ ...look(), beard: "unknown" }), null);
  assert.equal(saved({ ...look(), beardColor: "invisible" }), null);
  assert.equal(saved({ ...look(), callsign: "!" }), null);
  const missing = { ...look() }; delete missing.beard;
  assert.equal(saved(missing), null, "v2 is strict; only v1 receives missing defaults");
  const female = look({ sex: "female", beard: "full", beardColor: "grey" });
  assert.equal(female.beard, "none");
  assert.equal(saved({ ...female, beard: "full" }), null);
  const options = callRust<{ female: { beards: unknown[] }, male: { beards: unknown[] } }>("client.look.options", []);
  assert.equal(options.female.beards.length, 0);
  assert.equal(options.male.beards.length, 5);
});

test("first-person toggle affects only the player and restores a changed saved beard", () => {
  const player = fixture().root, enemy = fixture().root;
  const head = new THREE.Bone(); head.name = "head"; player.add(head);
  const original = look({ face: "m1", beard: "full" });
  const updated = look({ face: "m4", beard: "goatee" });
  sceneApplyLook(player, original); sceneApplyLook(enemy, original);
  const shown = (model: THREE.Group) => model.children.filter(n => n.name.startsWith("Beard_") && n.visible).map(n => n.name);
  sceneSetBeardFirstPerson(player, true, original);
  assert.equal(shown(player).length, 0);
  assert.equal(shown(enemy).length, 2);
  assert.equal(head.visible, true);
  sceneApplyLook(player, updated);
  assert.equal(shown(player).length, 0, "reapplying an edited look cannot reveal facial hair in FP");
  sceneSetBeardFirstPerson(player, false, updated);
  assert.deepEqual(shown(player), ["Beard_m4_goatee_cards", "Beard_m4_goatee_root"]);
  assert.deepEqual(shown(enemy), ["Beard_m1_full_cards", "Beard_m1_full_root"]);
  assert.equal(head.visible, true);
});

test("current gear dressing and camera refresh hide Sfera aliases and restore open gear", () => {
  const player = fixture().root;
  for (const name of ["Helmet", "Cap", "Sfera"]) {
    const mesh = new THREE.Mesh(new THREE.BufferGeometry(), new THREE.MeshStandardMaterial());
    mesh.name = name; player.add(mesh);
  }
  const chosen = look({ face: "m2", beard: "full" });
  sceneApplyLook(player, chosen);
  const shown = () => player.children.filter(n => n.name.startsWith("Beard_") && n.visible).length;
  for (const id of ["head-sfera", "head-filin", "head-cap", "head-k6", null]) {
    sceneDress({ THREE }, player, { head: id });
    sceneSetBeardFirstPerson(player, false, chosen);
    const closed = id === "head-sfera" || id === "head-filin";
    assert.equal(shown(), closed ? 0 : 2, `dress then refresh: ${id}`);
    sceneSetBeardFirstPerson(player, true, chosen);
    assert.equal(shown(), 0);
    // ArmsOnly restores the selected helmet before the beard is refreshed on leaving FP.
    sceneDress({ THREE }, player, { head: id });
    sceneSetBeardFirstPerson(player, false, chosen);
    assert.equal(shown(), closed ? 0 : 2, `return from first person: ${id}`);
  }
  assert.equal(chosen.beard, "full");
});
