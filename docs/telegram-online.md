# Telegram для сохраняемого онлайн-PvP

Задача [#20](https://github.com/i4w7w4a/Crowned_Dogs/issues/20), общий выпуск [#16](https://github.com/i4w7w4a/Crowned_Dogs/issues/16). Модули этого направления отвечают за карточки, Telegram transport, доставку outbox и подтверждение входа на сайте. Приглашения, бой, аккаунт, SQL и HTTP принадлежат общему онлайн-сервису. Живые отправки и production-настройки в этой работе не выполнялись.

## Подключение в общем runtime

- `createOnlineBot(token, {browserLogin,botUsername,webAppUrl,...})` из `apps/server/src/online/telegram-bot.ts` создаёт Grammy Bot без запуска. Общий runtime вызывает `bot.start()` ровно один раз. `botInfo` и `now` позволяют проверить фабрику без живого API.
- Этот Bot не создаёт `DuelService`/`OperatorRegistry`. `/start`, `/duel`, `/fight`, `/me`, `/rules` и прежние боевые кнопки направляют в Mini App. Правила и профиль в старом in-memory сервисе не обновляются.
- `TelegramTransport({token,allowedChatIds,...})` использует официальный HTTPS Bot API. `apiRoot` предусмотрен для контролируемого API; незашифрованный адрес допустим только на loopback. Редиректы запрещены. Токен и Telegram description не попадают в возвращаемые ошибки.
- `TelegramOutboxWorker({outbox,transport,botUsername,...}).runOnce()` доставляет одну запись. Таймер принадлежит общему runtime, второй Telegram poller здесь отсутствует. Worker не выполняет параллельные отправки при наложении двух вызовов.
- При остановке runtime сначала прекращает новые итерации, вызывает `worker.stop()` и `bot.stop()`, дожидается завершения активных обработчиков и только затем закрывает БД. `worker.stop()` отменяет HTTP-запросы, не начинает новую отправку после отмены и оставляет незавершённый lease для восстановления. Уже принятый Telegram запрос отменить нельзя.

## Доставка

Источник истины для интерфейса — `TelegramOutboxPort`/`TelegramDelivery` в `telegram-outbox.ts`. SQL-адаптер реализуется в серверном направлении #18.

`claimDue(now,leaseMs)` возвращает `{leaseId,kind,attempts,chatId,messageId,card}`. Каждый claim выдаёт новый fencing token `leaseId`; `complete(leaseId,messageId)` и `fail(leaseId,{code,retryAt})` возвращают false для устаревшего lease. SQL сериализует события одного приглашения и хранит исходные `chatId/messageId`. Транспорт использует актуальную committed карточку `{token,status,host:{telegram,level,tag},expiresAt}`; все сроки в миллисекундах Unix.

Worker получает lease на 60 секунд. Три последовательных запроса Telegram ограничены максимум 10 секундами каждый. Перед каждой отправкой или редактурой проверяются разрешённая группа, её тип и права бота через `getChat`/`getChatMember`. Операторская группа выбирается в серверной конфигурации; публичный клиент не присылает произвольный `chatId`.

Успешный `sendMessage` сохраняет message ID; `editMessageText` использует прежний ID. Принятый вызов сохраняет кнопку открытия боя, отменённый/истёкший убирает её. Если отменённый/истёкший вызов ещё не был опубликован, новая карточка не отправляется; outbox завершается с null.

Безопасные коды ошибок: `chat_not_allowed`, `bot_forbidden`, `telegram_rate_limited`, `telegram_unavailable`, `telegram_bad_request`. Для 429 учитывается `retry_after`, для временных ошибок — задержка с ростом до минуты. Ошибки прав и запрет группы требуют исправления настройки и явного retry/cancel. Ручная команда retry должна повторять исходный вызов с тем же token, не создавать новый.

Сбой после Telegram send до SQL acknowledgement может дать повтор поста. Это ограничение внешнего API: повтор содержит тот же token, не создаёт второй вызов/бой. Сбой acknowledgement не превращается в другой transport failure и не вызывает немедленную повторную отправку. Переподнятый worker повторно получает lease через SQL.

## Ссылки и подтверждение входа

В группе карточка содержит URL `https://t.me/<botUsername>?startapp=duel_<token>`. Открытие показывает вызов; принятие — отдельное действие в игре. `web_app` не используется в группе. Имя и username экранируются как HTML, перевод строки в имени не позволяет подделать строки карточки. Внешние ссылки из пользовательского имени не превращаются в HTML.

Для обычного браузера согласован bridge с TTL 5 минут:

1. `POST /auth/browser/start {inviteToken?:string}` возвращает `{challengeId,botUrl,expiresAt}`. Сервер сохраняет приглашение, создаёт случайный challenge ID (24 байта/base64url, 32 символа) и nonce исходного браузера (32 байта). Nonce доступен только HttpOnly cookie; БД хранит хеш.
2. `botUrl` ведёт в `/start login_<challengeId>`. Команда только показывает доверенный домен и предупреждение подтверждать лишь вход, начатый самим игроком. Это ещё не авторизация.
3. Кнопка `login:<challengeId>` вызывает `BrowserLoginPort.confirm` с identity из доверенного Telegram update. Подтверждение принимается только в личном чате самого пользователя. Введённый username и URL identity не принимаются.
4. `GET /auth/browser/status` возвращает `{status:pending|confirmed|expired,expiresAt}` только исходному браузеру с nonce cookie.
5. `POST /auth/browser/consume {}` одноразово возвращает штатную сессию и `{account,maxLevel,inviteToken}`. Сайт и Mini App получают один аккаунт через общий store. Сессионный token не переносится в URL. Подтверждение другой identity не меняет уже подтверждённую заявку.

SQL, cookie, защита повторов и HTTP этих маршрутов реализуются в #18; экран и сохранение token — в #19. Bot использует `BrowserLoginPort` из `browser-login-types.ts`, проверяет совпадение origin заявки с настроенным `webAppUrl`. Ни один Telegram handler сам не выдаёт сессию.

## Настройки для выпуска

Оператор выполняет настройки после общей приёмки; эти действия не автоматизированы в тестах.

1. В BotFather у выбранного бота настроить **Main Mini App** на окончательный HTTPS URL игры. Проверить username: он должен совпадать с `botUsername` runtime. Настроить кнопку меню на тот же HTTPS URL.
2. Разрешить добавление бота в группы. Добавить именно этого бота в выбранную игровую группу и разрешить отправку сообщений; права администратора нужны только если обычному участнику писать запрещено. Отключать privacy mode для этого пути не требуется.
3. В операторской конфигурации задать числовой ID группы в shared game service и тот же ID в allowlist транспорта. Миграция группы или новый ID не должны приниматься автоматически из пользовательского запроса/ошибки Telegram.
4. Передать токен только серверному окружению. Клиенту доступны username/публичные ссылки. BotFather Login/OIDC secrets для bridge не нужны.
5. На cutover остановить прежний poller, затем запустить единственный online runtime с API, Bot и outbox/deadline workers. При наличии webhook сначала явно определить переход к long polling; не удалять webhook скрытно. Два процесса с одним токеном одновременно не запускать.
6. Проверить двумя аккаунтами: публикацию, вход/явное принятие, изменение карточки, отмену/истечение, сбой прав бота, retry того же token, вход с сайта с сохранённым приглашением и тот же жетон в Mini App. Проверка локального API не заменяет этот выпускной прогон.

## Проверки без живых сообщений

```powershell
node --test apps/server/test/telegram-cards.test.ts apps/server/test/telegram-transport.test.ts apps/server/test/telegram-outbox.test.ts apps/server/test/telegram-online-bot.test.ts
npm run check
npm run web:build
```

Транспортные тесты поднимают временный HTTP API только на `127.0.0.1`. Bot-тесты передают synthetic `botInfo` и перехватывают Grammy API. Реальные токены, Telegram updates, группы и БД VPS не используются.

Источники: [InlineKeyboardButton](https://core.telegram.org/bots/api#inlinekeyboardbutton), [Main Mini App](https://core.telegram.org/bots/webapps#launching-the-main-mini-app), [ChatMember](https://core.telegram.org/bots/api#chatmember), [Bot deep links](https://core.telegram.org/bots/features#deep-linking).
