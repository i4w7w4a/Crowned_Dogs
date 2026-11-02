/** Telegram markup and view policy are produced by the Rust engine. */
import { callRust } from "../../../packages/runtime/src/index.ts";
import type { Side, TurnReport } from "@crowned-dogs/core";
import type { Duel } from "./duels.ts";
export interface Button {
  text: string;
  /** Callback data sent back on a tap. */
  data: string;
  /** Opens a link instead of sending data. */
  url?: string;
}

export interface Screen {
  text: string;
  buttons: Button[][];
}


export type AfterContract = "" | "bot" | "duel" | `duel_${string}`;
export type PickMode = "bot" | "duel" | `join:${string}`;
export const { CONTRACT, VETERAN_NOTE, WELCOME, RULES } = callRust<Record<"CONTRACT" | "VETERAN_NOTE" | "WELCOME" | "RULES", string>>("server.view.data", []);
export const escapeHtml = (text: string): string => callRust("server.view.escapeHtml", [text]);
export const steps = (count: number): string => callRust("server.view.steps", [count]);
export const turnData = (duel: Duel, payload: string): string => callRust("server.view.turnData", [duel, payload]);
export const turnScreen = (duel: Duel, side: Side): Screen => callRust("server.view.turnScreen", [duel, side]);
export const turnReport = (duel: Duel, side: Side, report: TurnReport): string => callRust("server.view.turnReport", [duel, side, report]);
export const finalScreen = (duel: Duel, side: Side): Screen => callRust("server.view.finalScreen", [duel, side]);
export const contractButtons = (next: AfterContract = ""): Button[][] => callRust("server.view.contractButtons", [next]);
export const signedText = (tag: string): string => callRust("server.view.signedText", [tag]);
export const tagText = (tag: string, name: string): string => callRust("server.view.tagText", [tag, name]);
export const nameWithTag = (player: { name: string; tag?: string }): string => callRust("server.view.nameWithTag", [player]);
export const menuButtons = (): Button[][] => callRust("server.view.menuButtons", []);
export const weaponPicker = (mode: PickMode, veteran = false): Button[][] => callRust("server.view.weaponPicker", [mode, veteran]);
