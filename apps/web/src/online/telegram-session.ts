import { OnlineError, OnlineTransport, type PendingStorage } from "./transport.ts";

async function digest(initData:string):Promise<string> {
  const bytes=await crypto.subtle.digest("SHA-256",new TextEncoder().encode(initData));
  return Array.from(new Uint8Array(bytes),b=>b.toString(16).padStart(2,"0")).join("");
}

export async function telegramSession(api: OnlineTransport, initData: string, storage: PendingStorage, key: string): Promise<any> {
  let authenticated;
  try { authenticated=await api.post("/auth/telegram",{initData}); }
  catch(error) {
    if(!(error instanceof OnlineError) || error.status!==401) throw error;
    const reopen=()=>new OnlineError(401,"mini_app_reopen");
    let binding;
    try {
      binding=JSON.parse(storage.getItem(key)??"null");
      if(!binding?.accountId || binding.digest!==await digest(initData)) throw reopen();
    } catch { throw reopen(); }
    // The cookie remains the authority. This hint only prevents a different launch/account
    // from silently receiving a previously authenticated player's profile.
    let current;
    try { current=await api.get("/me"); }
    catch(error) { if(error instanceof OnlineError && error.status===401) throw reopen(); throw error; }
    if(current.account.id!==binding.accountId) throw reopen();
    return current;
  }
  try { storage.setItem(key,JSON.stringify({digest:await digest(initData),accountId:authenticated.account.id})); }
  catch { /* Storage restrictions must not invalidate a successful server login. */ }
  return authenticated;
}
