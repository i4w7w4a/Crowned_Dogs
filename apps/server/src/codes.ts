// Button presses packed into Telegram callback data, which allows only 64 bytes per button.

import { ZONES, type Command, type Zone } from "@crowned-dogs/core";

export const ZONE_KEYS: Record<string, Zone> = Object.fromEntries(ZONES.map((z) => [z[0]!, z])) as Record<string, Zone>;

export function encodeCommand(c: Command): string {
  switch (c.type) {
    case "zone": return `z${c.zone[0]}`;
    case "guard": return `g${c.zone[0]}`;
    case "step": return `s${c.move}`;
    case "run": return `r${c.move}`;
    case "dash": return "d";
    case "knife": return "k";
    case "grenade": return "n";
    case "swap": return "w";
    case "flee": return "f";
  }
}

export function decodeCommand(s: string): Command | null {
  const head = s[0], rest = s.slice(1);
  switch (head) {
    case "z":
    case "g": {
      const zone = ZONE_KEYS[rest];
      if (!zone) return null;
      return head === "z" ? { type: "zone", zone } : { type: "guard", zone };
    }
    case "s":
      return rest === "-1" || rest === "0" || rest === "1" ? { type: "step", move: Number(rest) as -1 | 0 | 1 } : null;
    case "r":
      return rest === "2" || rest === "-2" ? { type: "run", move: Number(rest) as -2 | 2 } : null;
    case "d": return rest ? null : { type: "dash" };
    case "k": return rest ? null : { type: "knife" };
    case "n": return rest ? null : { type: "grenade" };
    case "w": return rest ? null : { type: "swap" };
    case "f": return rest ? null : { type: "flee" };
    default: return null;
  }
}
