export interface PendingStorage { getItem(key: string): string | null; setItem(key: string, value: string): void; removeItem(key: string): void }
export class OnlineError extends Error {
  status: number;
  constructor(status: number, code: string) { super(code); this.status = status; }
}
export class OnlineTransport {
  private base: string;
  private send: typeof fetch;
  private storage: PendingStorage;
  private key: string;
  private pending: {path:string,body:string} | null;
  private running: Promise<any> | null = null;
  constructor(base: string, send: typeof fetch, storage: PendingStorage) {
    this.base=base.replace(/\/$/, ""); this.send=send; this.storage=storage;
    this.key=`cd:online:pending:${this.base}`;
    const raw=storage.getItem(this.key);
    this.pending=raw ? JSON.parse(raw) : null;
  }
  private async request(path: string, body?: string): Promise<any> {
    if (!/^\/[a-zA-Z0-9_/?=&%-]*$/.test(path) || path.startsWith("//")) throw Error("invalid_api_path");
    const response=await this.send(this.base+path, {method:body===undefined?"GET":"POST", credentials:"same-origin", cache:"no-store",
      headers:body===undefined?{}:{"Content-Type":"application/json"}, body, signal:AbortSignal.timeout(15000)});
    const data=await response.json();
    if (!response.ok) throw new OnlineError(response.status, data.error ?? "request_failed");
    return data;
  }
  get(path: string): Promise<any> { return this.request(path); }
  bindAccount(id: string): void {
    if(this.running) throw Error("request_in_flight");
    this.key=`cd:online:pending:${this.base}:${id}`;
    const raw=this.storage.getItem(this.key);
    this.pending=raw?JSON.parse(raw):null;
  }
  get hasPending(): boolean { return this.pending !== null; }
  post(path: string, body: object): Promise<any> { return this.request(path, JSON.stringify(body)); }
  async command(path: string, body: object): Promise<any> {
    if (this.pending) throw new OnlineError(409,"pending_command");
    this.pending={path, body:JSON.stringify({...body,commandId:crypto.randomUUID()})};
    // Persist before sending: a page can close after the server commits, before the reply arrives.
    this.storage.setItem(this.key,JSON.stringify(this.pending));
    return this.recover();
  }
  async recover(): Promise<any> {
    if (this.running) return this.running;
    if (!this.pending) return null;
    const pending=this.pending;
    this.running=(async () => {
      try {
        const data=await this.request(pending.path,pending.body);
        this.pending=null; this.storage.removeItem(this.key);
        return data;
      } catch (error) {
        if (error instanceof OnlineError && error.status < 500) {
          this.pending=null; this.storage.removeItem(this.key);
        }
        throw error;
      } finally { this.running=null; }
    })();
    return this.running;
  }
}
