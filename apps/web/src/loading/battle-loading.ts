import { activate_after_ready, run_battle_loading } from "../../../../packages/runtime/src/index.ts";
export interface LoadingPresentation {
  show(onBack: () => void): void;
  ready(): Promise<void>;
  failed(): Promise<"retry" | "back">;
  hide(): void;
}
export function activateAfterReady(
  prepare: () => Promise<void>, beforeEnter: () => Promise<void>, enter: () => void, signal?: AbortSignal,
): Promise<void> { return activate_after_ready(prepare, beforeEnter, enter, signal) as Promise<void>; }
export function runBattleLoading(
  launch: (beforeEnter: () => Promise<void>, signal: AbortSignal) => Promise<void>, view: LoadingPresentation,
): Promise<boolean> { return run_battle_loading(launch, view) as Promise<boolean>; }
