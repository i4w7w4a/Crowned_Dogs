import { callRust } from "../../../../packages/runtime/src/index.ts";
import { OnlineTransport } from "./transport.ts";

/** Public projection only. The transport never receives or reconstructs a reducer snapshot. */
export class RemoteDuel {
  readonly online = true;
  snapshot: any;
  private view: any;
  private api: OnlineTransport;
  private sampledAt = performance.now();
  constructor(api: OnlineTransport, battle: any) { this.api=api; this.accept(battle); }
  accept(battle: any): void {
    if (!battle || (this.snapshot && (battle.id!==this.snapshot.id || battle.version<this.snapshot.version))) return;
    this.snapshot=battle;
    this.view=callRust("client.online.view",[battle]);
    this.sampledAt=performance.now();
  }
  get match(): any { return this.view.match; }
  get draft(): any { return this.view.draft; }
  get fighters(): any { return this.view.fighters; }
  get over(): boolean { return this.view.over; }
  get seconds(): number {
    const deadline=this.snapshot.deadline;
    return deadline===null?0:Math.max(0,Math.ceil((deadline-this.snapshot.serverNow-(performance.now()-this.sampledAt))/1000));
  }
  async refresh(): Promise<any> {
    await this.api.recover();
    const result=await this.api.get(`/matches/${encodeURIComponent(this.snapshot.id)}`);
    return {...result.battle,serverNow:result.serverNow??result.battle.serverNow};
  }
  async command(action: object): Promise<void> {
    const b=this.snapshot;
    const result=await this.api.command(`/matches/${encodeURIComponent(b.id)}/commands`,{expectedVersion:b.version,turn:b.match.turn,action});
    this.accept({...result.battle,serverNow:result.serverNow??result.battle.serverNow});
  }
  press(command: object): Promise<void> { return this.command({type:"press",command}); }
  submit(): Promise<void> { return this.command({type:"submit"}); }
  ready(): Promise<void> { return this.command({type:"ready"}); }
  surrender(): Promise<void> { return this.command({type:"surrender"}); }
  async setMove(move: number): Promise<void> {
    const commands=callRust<object[]>("client.online.moveCommands",[this.draft,move]);
    for (const command of commands) await this.press(command);
  }
}
