// Silent development review; not part of the production entry or game flow.
import { HomeStage } from "./home-stage.ts";
const canvas=document.querySelector<HTMLCanvasElement>("#base")!;
const progress=document.querySelector<HTMLInputElement>("#progress")!;
const status=document.querySelector<HTMLOutputElement>("#status")!;
const stage=new HomeStage(canvas,"/img/base-stage/manifest.json");
let route="battle",value=0,frame=0;
function draw(){stage.draw(route,value);status.value=`${route==="battle"?"Корона":"Туннель"} · ${Math.round(value*100)}%`;}
function stop(){cancelAnimationFrame(frame);frame=0;}
function set(v:number){stop();value=v;progress.value=String(v*1000);draw();}
await stage.ready;
document.querySelector<HTMLButtonElement>("#crown")!.onclick=()=>{route="battle";set(0);};
document.querySelector<HTMLButtonElement>("#tunnel")!.onclick=()=>{route="fitting";set(0);};
for(const [id,v] of [["start",0],["middle",.5],["end",1]] as const) document.querySelector<HTMLButtonElement>(`#${id}`)!.onclick=()=>set(v);
progress.oninput=()=>set(Number(progress.value)/1000);
document.querySelector<HTMLButtonElement>("#play")!.onclick=()=>{
  stop();const start=performance.now();
  const tick=(now:number)=>{value=Math.min(1,(now-start)/(route==="battle"?2100:2400));progress.value=String(value*1000);draw();if(value<1)frame=requestAnimationFrame(tick);};
  frame=requestAnimationFrame(tick);
};
addEventListener("resize",draw);
addEventListener("pagehide",()=>{stop();stage.dispose();},{once:true});
draw();
