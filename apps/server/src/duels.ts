// Live duels: who fights whom, the orders being put together, turn timers.
// Knows nothing about Telegram, so the same logic can later serve the web client.
// Everything is in memory for now: a restart ends all running fights.

import type { Command, Draft, Loadout, Match, Order, Side, TurnReport, WeaponId, Weather, TimeOfDay } from "@crowned-dogs/core";
import { callRust } from "../../../packages/runtime/src/index.ts";

export interface Player {
  id: string;
  name: string;
  /** Dog tag from the contract. Bots have none. */
  tag?: string;
  weapon: WeaponId;
  /** Gear for the fight. Without it: the factory weapon and starter gear. */
  kit?: Loadout;
  bot: boolean;
}

export interface Duel {
  id: string;
  players: [Player, Player];
  match: Match;
  drafts: [Draft, Draft];
  /** Locked orders. A side with no order when the timer runs out misses the turn. */
  orders: [Order | null, Order | null];
  /** When the current turn ends, in ms since 1970. */
  deadline: number;
}

export interface Invite {
  code: string;
  host: Player;
  expires: number;
}

export type DuelEvent =
  | { type: "start"; duel: Duel }
  /** A turn was played. If duel.match.result is set, the fight is over. */
  | { type: "turn"; duel: Duel; report: TurnReport };

export interface Clock {
  now(): number;
  setTimeout(fn: () => void, ms: number): unknown;
  clearTimeout(handle: unknown): void;
}

const realClock: Clock = {
  now: () => Date.now(),
  setTimeout: (fn, ms) => setTimeout(fn, ms),
  clearTimeout: (h) => clearTimeout(h as ReturnType<typeof setTimeout>),
};

export interface DuelServiceOptions {
  onEvent: (e: DuelEvent) => void | Promise<void>;
  clock?: Clock;
  /** Source of seeds for fights and invite codes. Random by default. */
  seed?: () => string;
  /** Fix the map and conditions (tests, events). */
  setting?: { mapId: string; weather: Weather; time: TimeOfDay };
}

/** Only host clocks, random seeds and event delivery remain here. */
export class DuelError extends Error {}
interface ServiceState { duels: Record<string, Duel>; byPlayer: Record<string, string>; invites: Record<string, Invite> }
interface Response { state?: ServiceState; value?: unknown; events?: DuelEvent[]; error?: string; needsSeed?: boolean }
export class DuelService {
  private state = callRust<ServiceState>("server.duels.new", []);
  private references = new Map<string, Duel>();
  private timers = new Map<string, { handle: unknown; turn: number; deadline: number }>();
  private clock: Clock;
  private seed: () => string;
  private opts: DuelServiceOptions;
  constructor(opts: DuelServiceOptions) {
    this.opts = opts; this.clock = opts.clock ?? realClock;
    this.seed = opts.seed ?? (() => crypto.randomUUID());
  }
  private reference(duel: Duel): Duel {
    const known = this.references.get(duel.id);
    if (known) { Object.assign(known, duel); return known; }
    this.references.set(duel.id, duel); return duel;
  }
  private send<T>(method: string, args: unknown[]): T {
    const seeds: string[] = [], now = this.clock.now();
    let result: Response;
    for (;;) {
      result = callRust<Response>("server.duels.step", [this.state, method, args, now, seeds, this.opts.setting]);
      if (!result.needsSeed) break;
      seeds.push(this.seed());
    }
    this.state = result.state!;
    for (const [id, duel] of Object.entries(this.state.duels)) this.state.duels[id] = this.reference(duel);
    const value = result.value as Duel | null;
    const returned = value && typeof value === "object" && "match" in value ? this.reference(value) : result.value;
    this.syncTimers();
    for (const event of result.events ?? []) this.emit({ ...event, duel: this.reference(event.duel) });
    for (const id of this.references.keys()) if (!this.state.duels[id]) this.references.delete(id);
    if (result.error) throw new DuelError(result.error);
    return returned as T;
  }
  duelOf(playerId: string): Duel | undefined { return this.send<Duel | null>("duelOf", [playerId]) ?? undefined; }
  get(duelId: string): Duel | undefined { return this.send<Duel | null>("get", [duelId]) ?? undefined; }
  sideOf(duel: Duel, playerId: string): Side | null { return callRust("server.sideOf", [duel, playerId]); }
  startVsBot(player: Omit<Player, "bot">): Duel { return this.send("startVsBot", [player]); }
  createInvite(host: Omit<Player, "bot">): Invite { return this.send("createInvite", [host]); }
  inviteInfo(code: string): Invite | undefined { return this.send<Invite | null>("inviteInfo", [code]) ?? undefined; }
  acceptInvite(code: string, guest: Omit<Player, "bot">): Duel { return this.send("acceptInvite", [code, guest]); }
  press(playerId: string, duelId: string, turn: number, command: Command): Duel { return this.send("press", [playerId, duelId, turn, command]); }
  submit(playerId: string, duelId: string, turn: number): Duel { return this.send("submit", [playerId, duelId, turn]); }
  isReady(duel: Duel, side: Side): boolean { return callRust("server.isReady", [duel, side]); }
  private syncTimers(): void {
    for (const [id, timer] of this.timers) if (!this.state.duels[id]) { this.clock.clearTimeout(timer.handle); this.timers.delete(id); }
    for (const duel of Object.values(this.state.duels)) {
      const previous = this.timers.get(duel.id);
      if (previous?.turn === duel.match.turn && previous.deadline === duel.deadline) continue;
      if (previous) this.clock.clearTimeout(previous.handle);
      const turn = duel.match.turn;
      const handle = this.clock.setTimeout(() => { this.send("timeout", [duel.id, turn]); }, Math.max(0, duel.deadline - this.clock.now()));
      this.timers.set(duel.id, { handle, turn, deadline: duel.deadline });
    }
  }
  private emit(event: DuelEvent): void {
    try { void Promise.resolve(this.opts.onEvent(event)).catch(error => console.error("duel event failed", error)); }
    catch (error) { console.error("duel event failed", error); }
  }
  close(): void { for (const timer of this.timers.values()) this.clock.clearTimeout(timer.handle); this.timers.clear(); }
}
