// Starts the Telegram bot. Token comes from BOT_TOKEN (see .env.example).

import { createBot } from "./bot.ts";
import { OperatorRegistry } from "./operators.ts";

const token = process.env.BOT_TOKEN?.trim();
if (!token) {
  console.error("Нет BOT_TOKEN. Получи токен у @BotFather, положи его в файл .env (образец в .env.example) и запусти снова.");
  process.exit(1);
}

// Signed contracts and dog tags live in a file, so they survive a restart. Fights do not yet.
const operators = new OperatorRegistry({ file: process.env.OPERATORS_FILE?.trim() || "data/operators.json" });
const { bot, duels } = createBot(token, { operators });

try {
  await bot.init();
} catch (err) {
  console.error("Не получилось подключиться к Telegram. Проверь BOT_TOKEN и интернет.", err);
  process.exit(1);
}

await bot.api.setMyCommands([
  { command: "fight", description: "Бой с ботом" },
  { command: "duel", description: "Позвать друга на дуэль" },
  { command: "rules", description: "Правила коротко" },
  { command: "me", description: "Мой жетон" },
]);

const stop = () => {
  duels.close();
  void bot.stop();
};
process.once("SIGINT", stop);
process.once("SIGTERM", stop);

await bot.start({ onStart: (me) => console.log(`Бот @${me.username} запущен`) });
