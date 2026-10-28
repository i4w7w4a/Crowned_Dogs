// A fight against the bot that runs entirely in the browser, on the same core
// rules as the Telegram bot. Later the server will judge online fights and this
// stays for training. No DOM here, so it is tested with node.

import {
  DASH_STEPS, RUN_STEPS, TIMES, WEATHERS, applyCommand, botOrder, createMatch, distanceLimit, draftChance,
  freshDraft, kitFighter, missing, resolveTurn, swapWeapon, toOrder, type Command, type Draft, type FighterSetup,
  type Loadout, type Match, type Side, type TimeOfDay, type TurnReport, type Weather, type WeaponId,
} from "@crowned-dogs/core";

export interface LocalFighter {
  name: string;
  weapon: WeaponId;
  /** Gear with mods and a possible sidearm; without it the fight uses the plain numbers of rules.ts. */
  kit?: Loadout;
  /** Dog tag from the contract. */
  tag?: string;
}

export interface LocalDuelOptions {
  seed?: string;
  mapId?: string;
  weather?: Weather;
  time?: TimeOfDay;
  startDistance?: number;
  player: LocalFighter;
  bot: LocalFighter;
}

/** The furthest the move control goes each way: a run back, a dash forward. */
export const MOVE_MIN = -RUN_STEPS;
export const MOVE_MAX = DASH_STEPS;

/** The move control is one number; the core splits it into a step, a run or a dash. */
export function moveCommand(move: number): Command {
  if (move >= DASH_STEPS) return { type: "dash" };
  if (Math.abs(move) >= RUN_STEPS) return { type: "run", move: move > 0 ? 2 : -2 };
  return { type: "step", move: Math.sign(move) as -1 | 0 | 1 };
}

export class LocalDuel {
  readonly me: Side = 0;
  readonly bot: Side = 1;
  readonly fighters: [LocalFighter, LocalFighter];
  match: Match;
  draft: Draft;

  constructor(opts: LocalDuelOptions) {
    const seed = opts.seed ?? crypto.randomUUID();
    const pick = <T>(list: readonly T[], salt: number): T => list[(hash(seed) + salt) % list.length]!;
    this.fighters = [opts.player, opts.bot];
    this.match = createMatch({
      seed,
      mapId: opts.mapId ?? "saltHall",
      weather: opts.weather ?? pick(WEATHERS, 1),
      time: opts.time ?? pick(TIMES, 2),
      ...(opts.startDistance ? { startDistance: opts.startDistance } : {}),
      fighters: [setup("me", opts.player), setup("bot", opts.bot)],
    });
    this.draft = freshDraft(this.match, this.me);
  }

  /** One tap. Returns a short text to show when the tap is not allowed. */
  press(c: Command): string | null {
    const next = applyCommand(this.match, this.me, this.draft, c);
    if (typeof next === "string") return next;
    this.draft = next;
    return null;
  }

  /** Sets the move control: -2 run back … 0 stand … 3 dash. */
  setMove(move: number): string | null {
    const target = Math.max(MOVE_MIN, Math.min(MOVE_MAX, Math.round(move)));
    if (target === this.draft.move && this.draft.kind !== "flee") return null;
    const c = moveCommand(target);
    // Run and dash commands toggle in the core, so leave them first if one is set.
    if ((this.draft.kind === "run" || this.draft.kind === "dash") && c.type !== "step") this.press({ type: "step", move: 0 });
    return this.press(c);
  }

  /** What the order still lacks, in words for the "end turn" button. Null when it is complete. */
  get missing(): string | null {
    return missing(this.draft);
  }

  /** Hit chance of the aimed shot or knife if the bot stands still. */
  get chance(): number | null {
    return draftChance(this.match, this.me, this.draft);
  }

  /** Distance after our own move if the bot stands still. */
  get plannedDistance(): number {
    return Math.max(1, Math.min(distanceLimit(this.match), this.match.distance - this.draft.move));
  }

  /** The other gun of our kit, the one a swap would put in hand. Null without a sidearm. */
  get spare(): WeaponId | null {
    return swapWeapon(this.match, this.me);
  }

  get over(): boolean {
    return this.match.result !== null;
  }

  /** Locks our order, lets the bot answer and plays the turn. Returns the report, or why the order is not ready. */
  submit(): TurnReport | string {
    if (this.over) return "Бой окончен";
    const order = toOrder(this.match, this.me, this.draft);
    if (typeof order === "string") return order;
    return this.play(order);
  }

  /** The turn timer ran out: whatever is ready is sent, the rest is filled like a real player in a hurry would. */
  timeout(): TurnReport {
    const order = toOrder(this.match, this.me, this.draft);
    return this.play(typeof order === "string" ? null : order);
  }

  private play(order: Parameters<typeof resolveTurn>[1][0]): TurnReport {
    const orders: Parameters<typeof resolveTurn>[1] = [null, null];
    orders[this.me] = order;
    orders[this.bot] = botOrder(this.match, this.bot);
    const { match, report } = resolveTurn(this.match, orders);
    this.match = match;
    this.draft = freshDraft(match, this.me);
    return report;
  }
}

function setup(id: string, f: LocalFighter): FighterSetup {
  return f.kit ? kitFighter(id, f.name, f.kit) : { id, name: f.name, weapon: f.weapon };
}

/** A small stable hash, so the same seed gives the same weather and time of day. */
function hash(s: string): number {
  let h = 2166136261;
  for (let i = 0; i < s.length; i++) h = Math.imul(h ^ s.charCodeAt(i), 16777619);
  return h >>> 0;
}
