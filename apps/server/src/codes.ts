/** Telegram callback byte format is implemented and validated by Rust. */
import type { Command, Zone } from "@crowned-dogs/core";
import { callRust } from "../../../packages/runtime/src/index.ts";
export const ZONE_KEYS: Record<string, Zone> = { h: "head", b: "body", g: "groin", l: "legs" };
export const encodeCommand = (command: Command): string => callRust("server.encodeCommand", [command]);
export const decodeCommand = (text: string): Command | null => callRust("server.decodeCommand", [text]);
