/** Grammy construction only. Rust owns route handlers, contract flow and screen lifecycle. */
import { Bot, GrammyError } from "grammy";
import type { UserFromGetMe } from "grammy/types";
import { VETERAN_KITS, WEAPON_IDS } from "@crowned-dogs/core";
import { bind_server_bot } from "../../../packages/runtime/src/index.ts";
import { DuelError, DuelService, type DuelEvent } from "./duels.ts";
import { OperatorRegistry } from "./operators.ts";

export function createBot(
  token: string, opts: { botInfo?: UserFromGetMe; operators?: OperatorRegistry } = {},
): { bot: Bot; duels: DuelService; operators: OperatorRegistry } {
  const bot = new Bot(token, opts.botInfo ? { botInfo: opts.botInfo } : {});
  const operators = opts.operators ?? new OperatorRegistry();
  let renderEvent: (event: DuelEvent) => Promise<void>;
  const duels = new DuelService({ onEvent: event => renderEvent(event) });
  renderEvent = bind_server_bot(bot, duels, operators, { GrammyError, DuelError, VETERAN_KITS, WEAPON_IDS });
  return { bot, duels, operators };
}
