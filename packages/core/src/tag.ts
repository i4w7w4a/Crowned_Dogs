// The dog tag an operator gets when signing the contract. Its number,
// K-XXX-XXX with six digits, is the player's id in the bot and in the client.

export const TAG_COUNT = 1_000_000;

/** 417093 -> "K-417-093". */
export function formatTag(n: number): string {
  if (!Number.isInteger(n) || n < 0 || n >= TAG_COUNT) throw new RangeError(`no tag for ${n}`);
  const d = String(n).padStart(6, "0");
  return `K-${d.slice(0, 3)}-${d.slice(3)}`;
}

/**
 * Reads a tag the way people type it: "K-417-093", "k417093", "К 417 093"
 * (Cyrillic К) all give "K-417-093". Anything else gives null.
 */
export function parseTag(text: string): string | null {
  const m = /^[KkКк][\s-]*(\d{3})[\s-]*(\d{3})$/.exec(text.trim());
  return m ? `K-${m[1]}-${m[2]}` : null;
}
