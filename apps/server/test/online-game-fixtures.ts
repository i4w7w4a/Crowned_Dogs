import { embeddedDatabase, clock } from "./online-fixtures.ts";
import { migrate, OnlineStore } from "../src/online/storage.ts";
import { OnlineGameService } from "../src/online/game-service.ts";
import type { Database } from "../src/online/database.ts";
export async function seedGame(database: Database) {
    await migrate(database);
    const store = new OnlineStore(database);
    const accounts = [];
    for (let id = 1; id <= 3; id++)
        accounts.push((await store.login({ id: String(id), firstName: `Игрок ${id}`, lastName: null, username: null }, clock)).account);
    return { store, accounts, game: new OnlineGameService(database, { chatId: "-1001", threadId: 566, clock: () => clock }) };
}
export async function gameFixture() { const fixture = await embeddedDatabase(); return { ...fixture, ...await seedGame(fixture.database) }; }
