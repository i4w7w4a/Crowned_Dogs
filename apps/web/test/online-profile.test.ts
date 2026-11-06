import assert from "node:assert/strict";
import test from "node:test";
import {load_local_loadout,load_local_look,save_local_loadout,save_local_look} from "../../../packages/runtime/src/index.ts";
test("online profile reads ignore local progress and return isolated server snapshots",()=>{
  const world=globalThis as any, previous=world.__cdOnline, previousStore=world.localStorage;
  const account={appearance:{callsign:"Соль",tag:"K-001-009",sex:"male"},loadout:{weapon1:{item:"pistol-pustelga"}}};
  const intentions:any[]=[];
  world.__cdOnline={account,saveLoadout:async(v:any)=>{intentions.push(["loadout",v]);},saveAppearance:async(v:any)=>{intentions.push(["appearance",v]);}};
  world.localStorage={getItem:()=>{throw Error("local profile must not be read");},setItem:()=>{throw Error("local profile must not be written");}};
  try {
    const look=load_local_look(),kit=load_local_loadout(undefined);
    assert.equal(look.tag,"K-001-009"); look.tag="fake"; kit.weapon1.item="fake";
    assert.equal(account.appearance.tag,"K-001-009"); assert.equal(account.loadout.weapon1.item,"pistol-pustelga");
    assert.equal(save_local_loadout({weapon1:{item:"new"}},undefined),false,"pending network save is never reported as confirmed");
    assert.equal(save_local_look({callsign:"Новый"}),false);
    assert.deepEqual(intentions,[["loadout",{weapon1:{item:"new"}}],["appearance",{callsign:"Новый"}]]);
  } finally {world.__cdOnline=previous;world.localStorage=previousStore;}
});
