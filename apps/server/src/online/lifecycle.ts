/** Schedule after completion, so database stalls cannot build an overlapping queue. */
export function periodic(task: () => Promise<unknown>, interval: number, failed: () => void): {
    stop: () => Promise<void>;
} {
    let stopped = false;
    let timer: ReturnType<typeof setTimeout> | undefined;
    let flight: Promise<void>;
    const run = () => {
        flight = Promise.resolve().then(task).then(() => { }, failed).finally(() => {
            if (!stopped) {
                timer = setTimeout(run, interval);
                timer.unref();
            }
        });
    };
    run();
    return { stop: async () => { stopped = true; clearTimeout(timer); await flight; } };
}
