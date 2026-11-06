import { callRust } from "../../../../packages/runtime/src/index.ts";
import { OnlineError, OnlineTransport } from "./transport.ts";
import { RemoteDuel } from "./remote-duel.ts";
import { isLoginExpired, onlineErrorMessage as message } from "./errors.ts";

const inviteKey="cd:online:invite";
const matchKey="cd:online:match";
const panel=(): HTMLElement => {
  const el=document.createElement("section"); el.className="cd-online"; el.setAttribute("role","dialog"); el.setAttribute("aria-modal","true");
  document.body.append(el); return el;
};
function words(el:HTMLElement, title:string, body:string): void {
  el.replaceChildren(); const h=document.createElement("h1"),p=document.createElement("p"); h.textContent=title; p.textContent=body; el.append(h,p);
}
function button(el:HTMLElement,label:string,action:()=>void): HTMLButtonElement {
  const b=document.createElement("button"); b.type="button"; b.className="glass"; b.textContent=label; b.onclick=action; el.append(b); return b;
}
const delay=(ms:number)=>new Promise<void>(r=>setTimeout(r,ms));

export class OnlineSession {
  api: OnlineTransport;
  account: any;
  activity: any;
  saving=false;
  private status: HTMLElement;
  constructor(base: string) {
    this.api=new OnlineTransport(base,fetch.bind(globalThis),sessionStorage);
    this.status=document.createElement("p"); this.status.className="cd-online-save"; this.status.setAttribute("role","status"); this.status.hidden=true; document.body.append(this.status);
  }
  private setAccount(account:any): void { this.account=account; window.dispatchEvent(new Event("cd-online-profile")); }
  private note(text:string): void { this.status.textContent=text; this.status.hidden=!text; }
  async login(): Promise<void> {
    const tg=(window as any).Telegram?.WebApp;
    const search=new URLSearchParams(location.search);
    const start=search.get("tgWebAppStartParam") ?? tg?.initDataUnsafe?.start_param ?? "";
    const token=search.get("invite") ?? (start.startsWith("duel_")?start.slice(5):null);
    if (token && /^[A-Za-z0-9_-]{1,128}$/.test(token)) sessionStorage.setItem(inviteKey,token);
    // initDataUnsafe is used only for routing; identity always comes from the signed server login.
    const el=panel();
    let restartBrowserLogin=false;
    for (;;) {
      words(el,"Crowned Dogs","Входим в общий профиль…");
      try {
        let data;
        try { data=tg?.initData?await this.api.post("/auth/telegram",{initData:tg.initData}):await this.api.get("/me"); }
        catch (e) {
          if (!(e instanceof OnlineError) || e.status!==401) throw e;
          if (tg?.initData) throw e;
          {
            words(el,"Вход через Telegram","Подтвердите вход в боте. Этот браузер получит тот же профиль и жетон.");
            if(!restartBrowserLogin) await new Promise<void>(r=>button(el,"Войти через Telegram",r));
            restartBrowserLogin=false;
            const inviteToken=sessionStorage.getItem(inviteKey);
            const challenge=await this.api.post("/auth/browser/start",inviteToken?{inviteToken}:{});
            words(el,"Подтвердите вход","Откройте бота, нажмите «Подтвердить вход» и вернитесь сюда.");
            const link=document.createElement("a"); link.textContent="Открыть Telegram"; link.className="glass";
            const url=new URL(challenge.botUrl);
            if (url.protocol!=="https:" || url.hostname!=="t.me") throw Error("invalid_login_link");
            link.href=url.href; link.target="_blank"; link.rel="noopener noreferrer"; el.append(link);
            for (;;) {
              await delay(2500); const status=await this.api.get("/auth/browser/status");
              if (status.status==="expired") throw new OnlineError(410,"login_expired");
              if (status.status==="confirmed") break;
            }
            data=await this.api.post("/auth/browser/consume",{});
            if (data.inviteToken) sessionStorage.setItem(inviteKey,data.inviteToken);
          }
        }
        this.api.bindAccount(data.account.id);
        this.setAccount(data.account);
        try { await this.api.recover(); } catch(e) { if (!(e instanceof OnlineError) || e.status>=500) throw e; }
        await this.refresh();
        el.remove(); tg?.ready?.(); return;
      } catch(e) {
        words(el,"Не удалось войти",message(e));
        // Keep inviteKey: the next iteration includes the same invite in the new challenge.
        await new Promise<void>(r=>button(el,isLoginExpired(e)?"Начать вход заново":"Повторить",r));
        restartBrowserLogin=isLoginExpired(e);
      }
    }
  }
  async refresh(): Promise<void> {
    const me=await this.api.get("/me"); this.setAccount(me.account);
    await this.readActivity();
  }
  private async readActivity(): Promise<void> {
    this.activity=await this.api.get("/activity");
    const remembered=sessionStorage.getItem(matchKey);
    if(!this.activity.battle && remembered) {
      try { const result=await this.api.get(`/matches/${encodeURIComponent(remembered)}`); this.activity={...this.activity,...result}; }
      catch(e) { if(e instanceof OnlineError && e.status===404) sessionStorage.removeItem(matchKey); else throw e; }
    }
  }
  leaveBattle(duel:RemoteDuel): void {
    if(duel.over) sessionStorage.removeItem(matchKey);
    void this.refresh().catch(()=>{});
  }
  async profile(action:object): Promise<void> {
    if (this.saving) throw new OnlineError(409,"saving_profile");
    this.saving=true; this.note("Сохраняем на сервере…");
    document.getElementById("base-app")?.setAttribute("inert","");
    try {
      if(this.api.hasPending) {
        await this.api.recover();
        this.setAccount((await this.api.get("/me")).account);
        this.note("Предыдущее сохранение восстановлено. Проверьте выбор.");
        return;
      }
      const result=await this.api.command("/profile/commands",{expectedVersion:this.account.version,action});
      this.setAccount(result.account); this.note("Сохранено на сервере");
    } catch(e) {
      this.note(message(e));
      try { const me=await this.api.get("/me"); this.setAccount(me.account); } catch { /* keep last confirmed profile */ }
      throw e;
    } finally { this.saving=false; document.getElementById("base-app")?.removeAttribute("inert"); }
  }
  async saveLoadout(loadout:unknown): Promise<void> {
    const actions=callRust<object[]>("client.online.profileActions",[this.account.loadout,loadout]);
    for(const action of actions) await this.profile(action);
  }
  saveAppearance(value:unknown): Promise<void> { return this.profile({type:"appearance",value}); }
  async openBattle(signal?:AbortSignal): Promise<RemoteDuel> {
    const el=panel(); let waiting=false; let failure=""; let cancelled=false;
    const act=(fn:()=>Promise<unknown>)=>async()=>{
      if(waiting)return; waiting=true;
      try { await fn(); failure=""; } catch(e) { failure=message(e); } finally { waiting=false; }
    };
    try {
      for (;;) {
        signal?.throwIfAborted();
        if(cancelled) throw new DOMException("Возвращение на базу","AbortError");
        if (!waiting) {
          try {
            await this.api.recover();
            await this.readActivity();
            if(this.activity.battle) {
              sessionStorage.setItem(matchKey,this.activity.battle.id);
              sessionStorage.removeItem(inviteKey);
              return new RemoteDuel(this.api,{...this.activity.battle,serverNow:this.activity.serverNow??this.activity.battle.serverNow});
            }
            const token=sessionStorage.getItem(inviteKey);
            const invite=token?(await this.api.get(`/invites/${encodeURIComponent(token)}`)).invite:this.activity.invite;
            if (!invite) {
              words(el,"Вызов на дуэль",failure || "Бот опубликует приглашение в подключённом игровом чате.");
              button(el,"Вызвать на дуэль",act(async()=>{await this.api.command("/invites",{});}));
            } else {
              const owner=invite.host.id===this.account.id;
              words(el,owner?"Ваш вызов":"Приглашение на дуэль",`${invite.host.name} · ${invite.host.tag} · уровень ${invite.host.level}`);
              const p=document.createElement("p"); p.textContent=failure || (invite.status!=="open"?"Приглашение закрыто.":owner?"Ждём второго игрока. Открытие ссылки не принимает вызов.":"Участие начнётся только после вашего подтверждения."); el.append(p);
              if(invite.status==="open") {
                const endpoint=`/invites/${encodeURIComponent(invite.token)}`;
                if(!owner) button(el,"Принять вызов",act(async()=>{const result=await this.api.command(endpoint+"/accept",{expectedVersion:invite.version}); if(result.battle) this.activity={battle:result.battle};}));
                else {
                  button(el,"Отменить вызов",act(()=>this.api.command(endpoint+"/cancel",{expectedVersion:invite.version})));
                  if(invite.delivery.status==="failed") { p.textContent="Бот не смог опубликовать вызов. Можно повторить отправку или отменить."; button(el,"Повторить отправку",act(()=>this.api.command(endpoint+"/retry",{expectedVersion:invite.version}))); }
                }
              } else button(el,"Закрыть приглашение",()=>sessionStorage.removeItem(inviteKey));
            }
          } catch(e) { words(el,"Восстановление соединения",message(e)); }
          button(el,"На базу",()=>{cancelled=true;});
        }
        await delay(2500);
      }
    } finally { el.remove(); }
  }
}

export let online: OnlineSession;
export async function connectOnline(): Promise<OnlineSession> {
  online=new OnlineSession(import.meta.env.VITE_ONLINE_API ?? `${import.meta.env.BASE_URL}api/online`);
  (globalThis as any).__cdOnline=online;
  await online.login();
  return online;
}
