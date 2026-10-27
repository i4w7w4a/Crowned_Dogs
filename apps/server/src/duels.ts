// Live duels: who fights whom, the orders being put together, turn timers.
// Knows nothing about Telegram, so the same logic can later serve the web client.
// Everything is in memory for now: a restart ends all running fights.

import {
  MAPS, REGION_EVENTS, TIMES, WEAPON_IDS, WEATHERS, applyCommand, botOrder, createMatch, freshDraft, nextInt,
  pickWeighted, resolveTurn, seedRng, toOrder, turnSeconds, type Command, type Draft, type Match, type Order,
  type Side, type TurnReport, type WeaponId,
} from "@crowned-dogs/core";

export interface Player {
  id: string;
  name: string;
  /** Dog tag from the contract. Bots have none. */
  tag?: string;
  weapon: WeaponId;
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
  setting?: { mapId: string; weather: (typeof WEATHERS)[number]; time: (typeof TIMES)[number] };
}

/** Thrown with a message that can be shown to the player as is. */
export class DuelError extends Error {}

const INVITE_TTL = 30 * 60 * 1000;
const CODE_ALPHABET = "ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
const BOT_NAMES = ["Искра", "Сварка", "Глушь", "Ржавый", "Соль"];

export class DuelService {
  private duels = new Map<string, Duel>();
  private byPlayer = new Map<string, string>();
  private invites = new Map<string, Invite>();
  private timers = new Map<string, unknown>();
  private clock: Clock;
  private seed: () => string;
  private opts: DuelServiceOptions;

  constructor(opts: DuelServiceOptions) {
    this.opts = opts;
    this.clock = opts.clock ?? realClock;
    this.seed = opts.seed ?? (() => crypto.randomUUID());
  }

  duelOf(playerId: string): Duel | undefined {
    const id = this.byPlayer.get(playerId);
    return id ? this.duels.get(id) : undefined;
  }

  get(duelId: string): Duel | undefined {
    return this.duels.get(duelId);
  }

  sideOf(duel: Duel, playerId: string): Side | null {
    if (duel.players[0].id === playerId) return 0;
    if (duel.players[1].id === playerId) return 1;
    return null;
  }

  // ---------- starting fights ----------

  startVsBot(player: Omit<Player, "bot">): Duel {
    this.ensureFree(player.id);
    const rng = seedRng(this.seed());
    const bot: Player = {
      id: `bot:${nextInt(rng, 1e9)}`,
      name: BOT_NAMES[nextInt(rng, BOT_NAMES.length)]!,
      weapon: WEAPON_IDS[nextInt(rng, WEAPON_IDS.length)]!,
      bot: true,
    };
    return this.start({ ...player, bot: false }, bot);
  }

  createInvite(host: Omit<Player, "bot">): Invite {
    this.ensureFree(host.id);
    for (const [code, inv] of this.invites) if (inv.host.id === host.id) this.invites.delete(code);
    const rng = seedRng(this.seed());
    let code = "";
    do {
      code = Array.from({ length: 6 }, () => CODE_ALPHABET[nextInt(rng, CODE_ALPHABET.length)]).join("");
    } while (this.invites.has(code));
    const invite: Invite = { code, host: { ...host, bot: false }, expires: this.clock.now() + INVITE_TTL };
    this.invites.set(code, invite);
    return invite;
  }

  inviteInfo(code: string): Invite | undefined {
    const inv = this.invites.get(code);
    if (inv && inv.expires < this.clock.now()) {
      this.invites.delete(code);
      return undefined;
    }
    return inv;
  }

  acceptInvite(code: string, guest: Omit<Player, "bot">): Duel {
    const inv = this.inviteInfo(code);
    if (!inv) throw new DuelError("Приглашение не найдено или устарело");
    if (inv.host.id === guest.id) throw new DuelError("Это твоё приглашение. Перешли ссылку другу");
    this.ensureFree(guest.id);
    if (this.byPlayer.has(inv.host.id)) throw new DuelError("Соперник уже в другом бою");
    this.invites.delete(code);
    return this.start(inv.host, { ...guest, bot: false });
  }

  private ensureFree(playerId: string): void {
    if (this.byPlayer.has(playerId)) throw new DuelError("Ты уже в бою. Сначала закончи его");
  }

  private start(a: Player, b: Player): Duel {
    const seed = this.seed();
    const rng = seedRng(`${seed}:setting`);
    const maps = Object.keys(MAPS);
    const odds = REGION_EVENTS.calm.odds;
    const setting = this.opts.setting ?? {
      mapId: maps[nextInt(rng, maps.length)]!,
      weather: pickWeighted(rng, WEATHERS.map((w) => [w, odds[w]] as const)),
      time: TIMES[nextInt(rng, TIMES.length)]!,
    };
    const match = createMatch({
      seed,
      ...setting,
      fighters: [
        { id: a.id, name: a.name, weapon: a.weapon },
        { id: b.id, name: b.name, weapon: b.weapon },
      ],
    });
    const duel: Duel = {
      id: this.newDuelId(),
      players: [a, b],
      match,
      drafts: [freshDraft(match, 0), freshDraft(match, 1)],
      orders: [null, null],
      deadline: 0,
    };
    this.duels.set(duel.id, duel);
    for (const p of duel.players) if (!p.bot) this.byPlayer.set(p.id, duel.id);
    this.armTimer(duel);
    this.emit({ type: "start", duel });
    return duel;
  }

  private newDuelId(): string {
    const rng = seedRng(this.seed());
    let id = "";
    do {
      id = Array.from({ length: 6 }, () => CODE_ALPHABET[nextInt(rng, CODE_ALPHABET.length)]).join("").toLowerCase();
    } while (this.duels.has(id));
    return id;
  }

  // ---------- during a fight ----------

  /** Checks that a button belongs to the current turn of this player's fight. */
  private locate(playerId: string, duelId: string, turn: number): { duel: Duel; side: Side } {
    const duel = this.duels.get(duelId);
    if (!duel || duel.match.result) throw new DuelError("Этот бой уже закончился");
    const side = this.sideOf(duel, playerId);
    if (side === null) throw new DuelError("Это не твой бой");
    if (duel.match.turn !== turn) throw new DuelError("Этот ход уже прошёл");
    return { duel, side };
  }

  press(playerId: string, duelId: string, turn: number, cmd: Command): Duel {
    const { duel, side } = this.locate(playerId, duelId, turn);
    if (duel.orders[side]) throw new DuelError("Приказ уже отдан, ждём соперника");
    const next = applyCommand(duel.match, side, duel.drafts[side], cmd);
    if (typeof next === "string") throw new DuelError(next);
    duel.drafts[side] = next;
    return duel;
  }

  /** Locks the order. Plays the turn at once when everyone is ready. */
  submit(playerId: string, duelId: string, turn: number): Duel {
    const { duel, side } = this.locate(playerId, duelId, turn);
    if (duel.orders[side]) throw new DuelError("Приказ уже отдан, ждём соперника");
    const order = toOrder(duel.match, side, duel.drafts[side]);
    if (typeof order === "string") throw new DuelError(order);
    duel.orders[side] = order;
    const waiting = duel.players.some((p, s) => !p.bot && !duel.orders[s]);
    if (!waiting) this.play(duel);
    return duel;
  }

  /** Is this side's order locked for the current turn? */
  isReady(duel: Duel, side: Side): boolean {
    return duel.orders[side] !== null;
  }

  private armTimer(duel: Duel): void {
    this.disarm(duel.id);
    const ms = turnSeconds(duel.match) * 1000;
    duel.deadline = this.clock.now() + ms;
    const turn = duel.match.turn;
    this.timers.set(duel.id, this.clock.setTimeout(() => {
      const d = this.duels.get(duel.id);
      if (d && !d.match.result && d.match.turn === turn) this.play(d);
    }, ms));
  }

  private disarm(duelId: string): void {
    const t = this.timers.get(duelId);
    if (t !== undefined) this.clock.clearTimeout(t);
    this.timers.delete(duelId);
  }

  private play(duel: Duel): void {
    const m = duel.match;
    const orders = duel.players.map((p, s) =>
      p.bot ? botOrder(m, s as Side) : duel.orders[s as Side],
    ) as [Order | null, Order | null];
    const { match, report } = resolveTurn(m, orders);
    duel.match = match;
    duel.orders = [null, null];
    duel.drafts = [freshDraft(match, 0), freshDraft(match, 1)];
    if (match.result) this.finish(duel);
    else this.armTimer(duel);
    this.emit({ type: "turn", duel, report });
  }

  private finish(duel: Duel): void {
    this.disarm(duel.id);
    for (const p of duel.players) if (this.byPlayer.get(p.id) === duel.id) this.byPlayer.delete(p.id);
    this.duels.delete(duel.id);
  }

  private emit(e: DuelEvent): void {
    try {
      void Promise.resolve(this.opts.onEvent(e)).catch((err) => console.error("duel event failed", err));
    } catch (err) {
      console.error("duel event failed", err);
    }
  }

  /** Stops all timers (for a clean shutdown and in tests). */
  close(): void {
    for (const id of [...this.timers.keys()]) this.disarm(id);
  }
}
