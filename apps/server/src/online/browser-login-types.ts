export interface BrowserLoginIdentity { id: string; firstName: string; lastName: string | null; username: string | null }
/** Implemented by the shared SQL account service; only trusted Telegram updates may confirm. */
export interface BrowserLoginPort {
  describe(id: string, now: number): Promise<{ origin: string; expiresAt: number } | null>;
  confirm(id: string, identity: BrowserLoginIdentity, now: number): Promise<"confirmed" | "expired" | "already_confirmed">;
}
