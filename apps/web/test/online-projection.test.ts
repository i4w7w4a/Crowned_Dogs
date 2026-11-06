import assert from "node:assert/strict";
import test from "node:test";
import { readFileSync } from "node:fs";
import { callRust } from "../../../packages/runtime/src/index.ts";
const fixtures=JSON.parse(readFileSync(new URL("./fixtures/online-public-v1.json",import.meta.url),"utf8"));
test("server public fixtures drive both render perspectives without private reducer fields",()=>{
  for(const b of Object.values(fixtures) as any[]) for(const side of [0,1]) {
    const view=callRust<any>("client.online.view",[{...b,side}]);
    assert.equal(view.fighters[0].id,b.players[side].id);
    assert.equal(view.match.fighters[0].hp,b.match.fighters[side].hp);
    const controls=callRust<any>("ui.battleControls",[view.match,view.draft,false]);
    assert.ok(Number.isFinite(controls.dist));
    assert.equal(view.match.rng,undefined);
    assert.equal(view.match.config.seed,undefined);
    if(b.result?.winner!==null && b.result) assert.equal(view.match.result.winner,b.result.winner===side?0:1);
  }
});
test("public opponent names are escaped at the actual HTML log boundary",()=>{
  const hostile='<img src=x onerror="alert(1)"><b>name</b>';
  const b=structuredClone(fixtures.finished); b.players[0].name=hostile; b.match.fighters[0].name=hostile;
  const view=callRust<any>("client.online.view",[b]);
  assert.equal(view.fighters[1].name,hostile,"projection preserves the name as data");
  const log=callRust<any>("ui.fightLog",[view.match.history[0],["Ты",hostile],["pistol","pistol"]]);
  for(const markup of [log.entry,log.ticker]) {
    assert.ok(markup.includes("&lt;img src=x onerror=&quot;alert(1)&quot;&gt;&lt;b&gt;name&lt;/b&gt;"));
    assert.ok(!markup.includes("<img")); assert.ok(!markup.includes("<b>name"));
  }
});
