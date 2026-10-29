// Live duels: who fights whom, the orders being put together, turn timers.
// Knows nothing about Telegram, so the same logic can later serve the web client.
// Everything is in memory for now: a restart ends all running fights.

import type { Command, Draft, Loadout, Match, Order, Side, TurnReport, WeaponId, Weather, TimeOfDay } from "@crowned-dogs/core";
import { callRust, RustDuelService } from "../../../packages/runtime/src/index.ts";

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

export interface DuelServiceOptions {
  onEvent: (e: DuelEvent) => void | Promise<void>;
  clock?: Clock;
  /** Source of seeds for fights and invite codes. Random by default. */
  seed?: () => string;
  /** Fix the map and conditions (tests, events). */
  setting?: { mapId: string; weather: Weather; time: TimeOfDay };
}

/** Rust owns scheduling, shared references and event delivery; Node supplies host capabilities. */
export class DuelError extends Error {}
export class DuelService {
  private runtime: RustDuelService;
  private options: DuelServiceOptions;
  constructor(opts: DuelServiceOptions) {
    this.options = opts; // Host owns callback lifetime; Rust keeps only a weak reference.
    this.runtime = new RustDuelService(opts, DuelError, {
      clock: { now: Date.now, setTimeout, clearTimeout }, seed: () => crypto.randomUUID(),
    });
  }
  duelOf(playerId: string): Duel | undefined { return this.runtime.send("duelOf", [playerId]); }
  get(duelId: string): Duel | undefined { return this.runtime.send("get", [duelId]); }
  sideOf(duel: Duel, playerId: string): Side | null { return callRust("server.sideOf", [duel, playerId]); }
  startVsBot(player: Omit<Player, "bot">): Duel { return this.runtime.send("startVsBot", [player]); }
  createInvite(host: Omit<Player, "bot">): Invite { return this.runtime.send("createInvite", [host]); }
  inviteInfo(code: string): Invite | undefined { return this.runtime.send("inviteInfo", [code]); }
  acceptInvite(code: string, guest: Omit<Player, "bot">): Duel { return this.runtime.send("acceptInvite", [code, guest]); }
  press(playerId: string, duelId: string, turn: number, command: Command): Duel { return this.runtime.send("press", [playerId, duelId, turn, command]); }
  submit(playerId: string, duelId: string, turn: number): Duel { return this.runtime.send("submit", [playerId, duelId, turn]); }
  isReady(duel: Duel, side: Side): boolean { return callRust("server.isReady", [duel, side]); }
  close(): void { this.runtime.close(); }
}
