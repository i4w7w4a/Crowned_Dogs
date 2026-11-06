import { onlineRuntimeConfig } from "./config.ts";
import { connectDatabase } from "./database.ts";
import { createOnlineServer } from "./http.ts";
import { OnlineStore } from "./storage.ts";
import { OnlineGameService } from "./game-service.ts";
import { BrowserLoginService } from "./browser-login.ts";
import { SqlTelegramOutbox } from "./outbox.ts";
import { createOnlineBot } from "./telegram-bot.ts";
import { TelegramTransport } from "./telegram-transport.ts";
import { TelegramOutboxWorker } from "./telegram-outbox.ts";
import { periodic } from "./lifecycle.ts";
async function main() {
    const config = onlineRuntimeConfig(process.env);
    const db = connectDatabase(config.databaseUrl);
    db.pool.on("error", () => console.error("Online database connection lost"));
    try {
        const store = new OnlineStore(db);
        const game = new OnlineGameService(db, config);
        const browserLogin = new BrowserLoginService(store, config);
        const server = createOnlineServer({ store, game, browserLogin, ...config });
        if (!(await store.healthy()))
            throw new Error("Database not migrated");
        await game.tick(Date.now());
        const bot = createOnlineBot(config.botToken, { browserLogin, botUsername: config.botUsername, webAppUrl: config.webAppUrl });
        bot.catch(() => console.error("Online Telegram handler failed"));
        const worker = new TelegramOutboxWorker({ outbox: new SqlTelegramOutbox(db),
            transport: new TelegramTransport({ token: config.botToken, allowedChatIds: [config.chatId] }), botUsername: config.botUsername });
        await new Promise<void>((resolve, reject) => {
            server.once("error", reject);
            server.listen(config.port, config.host, () => { server.removeListener("error", reject); resolve(); });
        });
        console.log("Online API ready");
        const deadlines = periodic(() => game.tick(Date.now()), 1000, () => console.error("Online deadline transaction failed"));
        const deliveries = periodic(() => worker.runOnce(), 1000, () => console.error("Online outbox transaction failed"));
        let stopping = false;
        let polling: Promise<void> = Promise.resolve();
        const stop = async () => {
            if (stopping)
                return;
            stopping = true;
            const timeout = setTimeout(() => server.closeAllConnections(), 10000);
            timeout.unref();
            const httpClosed = new Promise<void>(resolve => server.close(() => resolve()));
            // stop() interrupts polling but does not await middleware; the start() promise does.
            const stopped = await Promise.allSettled([deadlines.stop(), deliveries.stop(), worker.stop(), bot.stop(), httpClosed]);
            if (stopped.some(r => r.status === "rejected"))
                process.exitCode = 1;
            await polling;
            clearTimeout(timeout);
            await db.pool.end();
        };
        const requestStop = () => { void stop().catch(() => { console.error("Online shutdown failed"); process.exitCode = 1; }); };
        process.once("SIGINT", requestStop);
        process.once("SIGTERM", requestStop);
        // One poller for the new runtime. Deployment stops the legacy bot before starting this service.
        polling = bot.start({ allowed_updates: ["message", "callback_query"] }).catch(() => {
            console.error("Online Telegram polling failed");
            process.exitCode = 1;
            requestStop();
        });
    }
    catch (error) {
        await db.pool.end();
        throw error;
    }
}
main().catch(() => { console.error("Online API startup failed; check configuration, database and migrations"); process.exitCode = 1; });
