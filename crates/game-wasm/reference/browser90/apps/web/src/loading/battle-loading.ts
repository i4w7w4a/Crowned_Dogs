/** Coordinates the real loading operation with its presentation, including retry/cancel. */
export interface LoadingPresentation {
  show(onBack: () => void): void;
  ready(): Promise<void>;
  failed(): Promise<"retry" | "back">;
  hide(): void;
}

/** The timer and fight controls activate only after readiness and the overlay's exit. */
export async function activateAfterReady(
  prepare: () => Promise<void>, beforeEnter: () => Promise<void>, enter: () => void, signal?: AbortSignal,
): Promise<void> {
  signal?.throwIfAborted();
  await prepare();
  signal?.throwIfAborted();
  await beforeEnter();
  signal?.throwIfAborted();
  enter();
}

/** A cancelled download may finish later, but must never activate a battle. */
export async function runBattleLoading(
  launch: (beforeEnter: () => Promise<void>, signal: AbortSignal) => Promise<void>,
  view: LoadingPresentation,
): Promise<boolean> {
  try {
    for (;;) {
      const controller = new AbortController();
      view.show(() => controller.abort());
      try {
        await abortable(launch(() => view.ready(), controller.signal), controller.signal);
        return true;
      } catch (error) {
        if (controller.signal.aborted) return false;
        if (await view.failed() !== "retry") return false;
      }
    }
  } finally { view.hide(); }
}

function abortable<T>(promise: Promise<T>, signal: AbortSignal): Promise<T> {
  return new Promise<T>((resolve, reject) => {
    const abort = () => reject(new DOMException("Loading cancelled", "AbortError"));
    if (signal.aborted) { abort(); return; }
    signal.addEventListener("abort", abort, { once: true });
    promise.then(resolve, reject).finally(() => signal.removeEventListener("abort", abort));
  });
}
