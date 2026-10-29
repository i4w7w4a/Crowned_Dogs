// Run once against the pre-migration TypeScript; fixtures are an independent oracle.
import { writeFileSync } from 'node:fs';
import * as load from '../../../apps/web/src/base/loadout-state.ts';
import { LocalDuel, moveCommand } from '../../../apps/web/src/game/local-duel.ts';
import * as core from '../../../packages/core/src/index.ts';
import * as cues from '../../../packages/audio/src/cues.ts';
import * as bank from '../../../packages/audio/src/bank.ts';
import * as words from '../../../apps/web/src/hud/words.ts';
import { QUALITY } from '../../../apps/web/src/arena/quality.ts';
import { MusicDirector } from '../../../packages/audio/src/music.ts';
const fixtures=[];
const add=(method,args,value)=>fixtures.push({method,args,value});
const state=d=>structuredClone({match:d.match,draft:d.draft,fighters:d.fighters});
const data={SOUNDS:bank.SOUNDS,TRACKS:bank.TRACKS,FIGHT_PLAYLIST:bank.FIGHT_PLAYLIST,RADIO_PLAYLIST:bank.RADIO_PLAYLIST,BASE_PLAYLIST:bank.BASE_PLAYLIST,FITTING_PLAYLIST:bank.FITTING_PLAYLIST,MIX:bank.MIX,TIMING:cues.TIMING,QUALITY,ZONE_TO:words.ZONE_TO,GUN_ACC:words.GUN_ACC,GUN_WITH:words.GUN_WITH,TIME_WORD:words.TIME_WORD,WEATHER_WORD:words.WEATHER_WORD};
writeFileSync(new URL('../fixtures/client.json',import.meta.url),JSON.stringify(data,null,2)+'\n');
for(const n of [-105,-21,-5,-2,-1,0,1,2,4,5,11,12,14,20,21,22,24,25,100,101,111])add('words.plural',[n,'one','few','many'],words.plural(n,'one','few','many'));
for(const n of [-3,-2,-1,0,1,2,3,4])add('client.moveCommand',[n],moveCommand(n));
for(const weapon of core.WEAPON_IDS){
 const kit=core.starterLoadout(weapon); add('client.isValidLoadout',[kit],true);
 for(const slot of core.GEAR_SLOTS){
  add('client.itemsForSlot',[slot],load.itemsForSlot(slot));add('client.equippedItemId',[kit,slot],load.equippedItemId(kit,slot));
  for(const item of load.itemsForSlot(slot))add('client.equipItem',[kit,slot,item.id],load.equipItem(kit,slot,item.id));
 }
 for(const slot of core.MOD_SLOTS){
  add('client.weaponModsForSlot',[kit,slot],load.weaponModsForSlot(kit,slot));add('client.effectiveWeaponModId',[kit,slot],load.effectiveWeaponModId(kit,slot));
  for(const mod of load.weaponModsForSlot(kit,slot)){ const next=load.equipWeaponMod(kit,slot,mod.id);add('client.equipWeaponMod',[kit,slot,mod.id],next);add('client.equipWeaponMod',[next,slot,null],load.equipWeaponMod(next,slot,null)); }
 }
}
for(const value of [null,[],{}, {weapon1:null}, {...core.starterLoadout('rifle'),weapon2:undefined},{...core.starterLoadout('rifle'),weapon1:{item:'rifle-varyag',mods:{wrong:'x'}}}])add('client.isValidLoadout',[value],false);
for(const ours of core.WEAPON_IDS)for(const theirs of core.WEAPON_IDS)add('audio.fightSounds',[ours,theirs],cues.fightSounds(ours,theirs));
add('audio.fightSounds',[['rifle','pistol'],['sniper','pistol']],cues.fightSounds(['rifle','pistol'],['sniper','pistol']));
for(let i=0;i<12;i++){
 const weapon=core.WEAPON_IDS[i%4],opts={seed:`client-oracle-${i}`,player:{name:'Тень',weapon,kit:i%2?core.VETERAN_KITS[weapon]:core.starterLoadout(weapon)},bot:{name:'Искра',weapon:core.WEAPON_IDS[(i+1)%4]}};
 const d=new LocalDuel(opts);add('client.duel.create',[opts],state(d));
 for(let turn=0;turn<18&&!d.over;turn++){
  for(const cmd of [{type:'zone',zone:core.ZONES[turn%4]},{type:'guard',zone:'head'},{type:'guard',zone:'body'}]){const before=state(d),result=d.press(cmd);add('client.duel.press',[before,cmd],{state:state(d),result});}
  if(turn%4===1){const before=state(d),result=d.setMove(turn%3-1);add('client.duel.setMove',[before,turn%3-1],{state:state(d),result});}
  for(const key of ['missing','chance','plannedDistance','spare','over'])add('client.duel.query',[state(d),key],d[key]);
  const before=state(d),result=d.timeout();add('client.duel.timeout',[before],{state:state(d),result});
  for(const side of [0,1]){
   const ctx={viewer:side};add('audio.cuesForTurn',[result,ctx],cues.cuesForTurn(result,ctx));
   add('words.turnLine',[result,side,d.match.fighters[side].weapon],words.turnLine(result,side,d.match.fighters[side].weapon));
  }
 }
 if(d.over)for(const side of [0,1])add('words.endWords',[d.match.result,side,'Искра',d.match.turn],words.endWords(d.match.result,side,'Искра',d.match.turn));
}
let operations=[],randomCalls=0;const deck={};
for(const method of ['play','pause','resume','stop','cue'])deck[method]=(...args)=>operations.push({method,args});
const director=new MusicDirector(deck,()=>{randomCalls++;return .37;});
const musicState=()=>({scene:director.scene,menuOpen:director.menuOpen,radioOn:director.radioOn,retuned:director.retuned,fightPaused:director.fightPaused,fightIndex:bank.FIGHT_PLAYLIST.indexOf(director.fightList.now),radioIndex:director.radioList.items.indexOf(director.radioList.now),radioPlaylist:[...director.radioList.items]});
add('audio.music.new',[],musicState());
for(const [method,arg] of [['openMenu'],['enterMenu'],['playlist',bank.BASE_PLAYLIST],['nextStation'],['enterFight'],['playlist',bank.FITTING_PLAYLIST],['openMenu'],['ended','fight'],['ended','radio'],['closeMenu'],['tune','fxck-wag-6'],['openMenu'],['closeMenu'],['enterMenu'],['playlist',bank.RADIO_PLAYLIST],['tune','fog-3'],['enterFight'],['ended','fight'],['silence']]){
 const before=musicState(), count=randomCalls;operations=[];director[method](arg);
 add('audio.music.needsRandom',[before,method,arg??null],randomCalls!==count);
 add('audio.music.step',[before,method,arg??null,.37],{state:musicState(),operations});
 add('audio.music.state',[musicState()],director.state);
}
writeFileSync(new URL('../fixtures/oracle.json',import.meta.url),JSON.stringify(fixtures)+'\n');
console.log(`Wrote ${fixtures.length} client/audio/words differential cases`);
