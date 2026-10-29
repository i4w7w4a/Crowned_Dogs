/** Typed facade over the Rust local-duel state machine. */
import { callRust } from "../../../../packages/runtime/src/index.ts";
import type { Command, Draft, Loadout, Match, Side, TimeOfDay, TurnReport, Weather, WeaponId } from "@crowned-dogs/core";
export interface LocalFighter { name: string; weapon: WeaponId; kit?: Loadout; tag?: string }
export interface LocalDuelOptions {
  seed?: string; mapId?: string; weather?: Weather; time?: TimeOfDay; startDistance?: number;
  player: LocalFighter; bot: LocalFighter;
}
interface DuelState { fighters: [LocalFighter, LocalFighter]; match: Match; draft: Draft }
export const MOVE_MIN = -2;
export const MOVE_MAX = 3;
export const moveCommand = (move: number): Command => callRust("client.moveCommand", [move]);
export class LocalDuel {
  readonly me: Side = 0;
  readonly bot: Side = 1;
  readonly fighters: [LocalFighter, LocalFighter];
  match: Match;
  draft: Draft;
  constructor(opts: LocalDuelOptions) {
    const state = callRust<DuelState>("client.duel.create", [{ ...opts, seed: opts.seed ?? crypto.randomUUID() }]);
    this.fighters = state.fighters; this.match = state.match; this.draft = state.draft;
  }
  private snapshot(): DuelState { return { fighters: this.fighters, match: this.match, draft: this.draft }; }
  private change<T>(method: string, value?: unknown): T {
    const { state, result } = callRust<{ state: DuelState; result: T }>("client.duel." + method, [this.snapshot(), value]);
    this.match = state.match; this.draft = state.draft;
    return result;
  }
  private query<T>(property: string): T { return callRust("client.duel.query", [this.snapshot(), property]); }
  press(command: Command): string | null { return this.change("press", command); }
  setMove(move: number): string | null { return this.change("setMove", move); }
  get missing(): string | null { return this.query("missing"); }
  get chance(): number | null { return this.query("chance"); }
  get plannedDistance(): number { return this.query("plannedDistance"); }
  get spare(): WeaponId | null { return this.query("spare"); }
  get over(): boolean { return this.query("over"); }
  submit(): TurnReport | string { return this.change("submit"); }
  timeout(): TurnReport { return this.change("timeout"); }
}
