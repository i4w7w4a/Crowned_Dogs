// One-way migration oracle. Runtime does not import or evaluate the old TS engine.
import * as rng from '../../../packages/core/reference/rng.ts';
import * as rules from '../../../packages/core/reference/rules.ts';
import * as items from '../../../packages/core/reference/items.ts';
import * as loadout from '../../../packages/core/reference/loadout.ts';
import * as match from '../../../packages/core/reference/match.ts';
import * as draft from '../../../packages/core/reference/draft.ts';
import * as ai from '../../../packages/core/reference/ai.ts';
import * as tag from '../../../packages/core/reference/tag.ts';
import { writeFileSync, mkdirSync } from 'node:fs';
const core = {...rng,...rules,...items,...loadout,...match,...draft,...ai,...tag};
const root = new URL('../', import.meta.url);
mkdirSync(new URL('catalog/', root), {recursive:true});
const constants = Object.fromEntries(Object.entries(core).filter(([k,v]) => typeof v !== 'function' && !(v instanceof Map)));
writeFileSync(new URL('catalog/catalog.json',root),JSON.stringify(constants,null,2)+'\n');
const cases=[];
function record(method,args) {
  const input=structuredClone(args);
  try {
    let expected=core[method](...args);
    if (['nextFloat','nextInt','roll','pickWeighted'].includes(method)) expected={value:expected,state:args[0]};
    if (method==='botOrder'&&args[2]) expected={value:expected,state:args[2]};
    cases.push({method,args:input,expected:structuredClone(expected)});
    return expected;
  } catch(error) {cases.push({method,args:input,error:error.message});}
}
for (const seed of ['','duel-42','Соляной цех 🐺',0,-1,417093]) {
  record('seedRng',[seed]);const state=core.seedRng(seed);
  for(let i=0;i<10;i++) {record('nextFloat',[state]);record('nextInt',[state,17]);record('roll',[state,.35]);record('pickWeighted',[state,[['a',3],['b',0],['c',1]]]);}
}
for (const v of [0,1,417093,999999,-1,1.5,1000000])record('formatTag',[v]);
for (const t of ['K-417-093','k417093','К 417 093','  к--001 - 002  ','K-000-000','X417093','k12-3456','K-123-4567','K123  456','K123456 '])record('parseTag',[t]);
for(let d=1;d<=16;d++){record('band',[d]);for(const w of Object.values(core.WEAPONS))record('baseDamage',[w,d]);}
for(const armor of [0,10,20,50,250])for(const pen of [0,.04,.3,1])record('armorFactor',[armor,pen,8,.04]);
for(const armor of [0,20,50])for(const move of [-3,-2,-1,0,1,2,3])record('grenadeDamage',[armor,move]);
for(const item of core.ITEMS){record('getItem',[item.id]);record('describeItem',[item]);record('describeEffects',[item.effects]);record('effectLines',[item.effects]);if(item.kind==='grenade')record('describeGrenade',[item.grenade]);}
record('getItem',['nonexistent']);record('weaponItem',['nonexistent']);
for(const weapon of core.WEAPON_IDS){
  record('weaponItem',[weapon]);record('starterLoadout',[weapon]);
  for(const slot of core.MOD_SLOTS)record('modsFor',[weapon,slot]);
  for(const kit of [core.starterLoadout(weapon),core.VETERAN_KITS[weapon]]){
    record('validateLoadout',[kit]);record('compileKit',[kit]);record('kitFighter',['test','ТЕНЬ',kit]);record('weaponParts',[kit.weapon1]);
    for(const slot of core.GEAR_SLOTS){const bad=structuredClone(kit);if(slot==='weapon1'||slot==='weapon2')bad[slot]={item:'bogus'};else bad[slot]='bogus';record('validateLoadout',[bad]);record('compileKit',[bad]);}
  }
  for(const mod of core.ITEMS.filter(i=>i.kind==='mod'&&i.fits.includes(weapon))){
    const kit=core.starterLoadout(weapon);kit.weapon1.mods={[mod.slot]:mod.id};record('compileKit',[kit]);record('applyWeaponEffects',[core.WEAPONS[weapon],[mod.effects]]);
  }
}
const battles=[];
for(let i=0;i<96;i++) {
  const w1=core.WEAPON_IDS[i%4],w2=core.WEAPON_IDS[Math.floor(i/4)%4];
  const kit1=structuredClone(i%3===1?core.starterLoadout(w1):core.VETERAN_KITS[w1]);
  const kit2=structuredClone(i%3===1?core.starterLoadout(w2):core.VETERAN_KITS[w2]);
  kit1.grenade=['grenade-rg1','grenade-grad','grenade-emp','grenade-smoke'][Math.floor(i/4)%4];
  kit2.grenade=['grenade-rg1','grenade-grad','grenade-emp','grenade-smoke'][Math.floor(i/16)%4];
  const config={seed:`rust-parity-${i}`,mapId:Object.keys(core.MAPS)[Math.floor(i/4)%4],weather:core.WEATHERS[Math.floor(i/16)%4],time:core.TIMES[Math.floor(i/4)%4],startDistance:1+i%13,
    fighters:i%3===0?[{id:'me',name:'ТЕНЬ',weapon:w1},{id:'bot',name:'ИСКРА',weapon:w2}]:[core.kitFighter('me','ТЕНЬ',kit1),core.kitFighter('bot','ИСКРА',kit2)]};
  let m=core.createMatch(config);
  record('createMatch',[config]);
  if(i<16){
    for(const side of [0,1]){
      for(const method of ['turnLimit','turnSeconds','distanceLimit','canFlee'])record(method,[m]);
      for(const method of ['swapWeapon','grenadeSpec','canThrowGrenade','canSwap','botRng'])record(method,[m,side]);
      for(const w of [...core.WEAPON_IDS,'knife']){
        record('weaponSpec',[m,side,w]);record('guardShare',[m,side,w]);record('critChance',[m,w,side]);
        for(const zone of core.ZONES)record('expectedHit',[m,side,w,zone,m.distance]);
        for(const still of [0,2])for(const dash of [false,true])record('hitChance',[m,side,w,m.distance,still,dash]);
      }
      for(const move of [-3,-2,-1,0,1,2,3]){record('plannedDistance',[m,move]);record('canUseKnife',[m,move]);record('grenadeHit',[m,side,move]);}
      record('conditionAccuracy',[m,side,m.distance]);record('botOrder',[m,side,core.seedRng(i)]);
      let d=core.freshDraft(m,side);record('freshDraft',[m,side]);record('grenadeRefusal',[m,side]);
      const commands=[{type:'guard',zone:'head'},{type:'guard',zone:'body'},{type:'zone',zone:'head'},{type:'dash'},{type:'guard',zone:'legs'},{type:'dash'},{type:'knife'},{type:'zone',zone:'body'},{type:'step',move:-1},{type:'grenade'},{type:'grenade'},{type:'swap'},{type:'swap'},{type:'run',move:2},{type:'run',move:-2},{type:'step',move:1},{type:'flee'},{type:'flee'}];
      for(const command of commands){const next=record('applyCommand',[m,side,d,command]);if(typeof next==='object')d=next;
        record('missing',[d]);record('guardsNeeded',[d]);record('actionOf',[d]);record('draftChance',[m,side,d]);record('toOrder',[m,side,d]);}
    }
  }
  const orders=[];
  while(!m.result){const input=[core.botOrder(m,0),core.botOrder(m,1)];orders.push(input);m=core.resolveTurn(m,input).match;}
  battles.push({config,orders,expected:m});
}
mkdirSync(new URL('tests/',root),{recursive:true});
// Repeated match/weapon inputs are interned to keep the fixture small and reviewable.
const pool=[], indices=new Map();
for (const c of cases) c.args=c.args.map(value=>{
  if (!value||typeof value!=='object')return value;
  const key=JSON.stringify(value);if(key.length<100)return value;
  let index=indices.get(key);if(index===undefined){index=pool.length;indices.set(key,index);pool.push(value);}
  return {$ref:index};
});
writeFileSync(new URL('tests/oracle.json',root),JSON.stringify({pool,cases,battles}));
console.log(`${cases.length} function cases, ${battles.length} complete seeded duels`);
