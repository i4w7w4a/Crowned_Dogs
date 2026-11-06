# Данные и выпуск online

Участок #25: доверенный импорт жетонов и проверяемое восстановление. Основа — #23 (`fd30e07b33a398ff5af60392aa53a2006f16f882`, runtime `155ee2efc528bd2d059cf153d86fd9e92432be7e`). Эти SHA не обозначают будущий интегрированный релиз. Перед выпуском координатор фиксирует точный принятый commit и фактические версии миграций из БД. Код этого участка не включает production и не изменяет старый реестр.

## 1. Импорт до первой регистрации

Источник — только доверенный серверный JSON-массив `{telegramId:string, tag:string, name:string, signedAt:number}`. `signedAt` — миллисекунды Unix. Дополнительные поля, недопустимые типы, нулевые номера жетона, повторные ID/жетоны и конфликты с БД отвергаются. localStorage и присланный игроком JSON источниками доверия не являются.

В корне уже собранного checkout (Node 22.18+, `npm ci`, `npm run rust:build`):

```bash
# IMPORT_DATABASE_URL поступает из защищённой конфигурации процесса, не из аргументов.
# Все переменные путей и имён оператор задаёт явно; имена БД проверяются CLI.
node apps/server/src/online/import-cli.ts \
  --file "$LEGACY_FILE" --database-env IMPORT_DATABASE_URL \
  --expect-database "$ONLINE_DB_NAME"

# Только после успешного dry-run и резервирования исходного файла:
node apps/server/src/online/import-cli.ts \
  --file "$LEGACY_FILE" --database-env IMPORT_DATABASE_URL \
  --expect-database "$ONLINE_DB_NAME" --apply
```

`--file` — абсолютный путь к обычному JSON-файлу до 10 MiB. CLI открывает его только на чтение. URL допускает только явные credentials, loopback, указанное имя БД, без query/fragment; он разбирается в отдельные параметры `pg`, исходный URL повторно не интерпретируется. Для удалённого обслуживания нужен предварительно проверенный защищённый локальный туннель. Инструмент не создаёт его сам. Пароли не попадают в argv и отчёт. Не включать `set -x` и не сохранять дамп окружения.

Отчёт JSON содержит режим и количества `total/created/existing`; у dry-run `created` означает «будет создано». Ошибка: `ok:false`, количества по категориям без имён, ID, жетонов, путей и SQL. Успех — exit 0; отказ — exit 1. `database_failure` требует приватной проверки доступности/прав/миграций; сырой текст ошибки намеренно не публикуется.

Одна SQL-транзакция берёт тот же `online_tag_counters(id=1) FOR UPDATE`, что регистрация. Каждую ID↔tag связь сверяет до записи; ошибка откатывает всё. Новый аккаунт получает свежие `core.syncFresh`, `client.look.normalize`, `core.starterLoadout` из Rust: уровень 1, общий максимум каталога 4. Сохраняются жетон, Telegram ID, имя и дата подписания. Сессия импортом не создаётся.

Совпадающая существующая связь остаётся целиком нетронутой: XP, уровень, награды, внешность, снаряжение, версия, сессии и имя не сбрасываются. Счётчик не уменьшается; занятые номера остаются в таблице, которую читает Rust allocator. CLI можно повторять. Он не разрешает параллельную выдачу номеров старым файловым ботом: старый poller необходимо остановить до финального чтения реестра.

## 2. Резервная копия и проверка восстановления

Нужны native PostgreSQL 18 и клиентские `pg_dump`/`pg_restore` 18. `PG_BIN` — необязательный абсолютный каталог бинарников (на Windows `.exe`), иначе используется PATH. Каждый subprocess получает заново заданные PGHOST/PGPORT/PGDATABASE/PGUSER/PGPASSWORD; случайные унаследованные PG-параметры удаляются. Вывод ошибок subprocess не публикуется. Таймаут каждой операции — 10 минут; крупные базы требуют отдельного эксплуатационного решения.

```bash
# BACKUP_DATABASE_URL — защищённое окружение. BACKUP_DIR — НОВЫЙ абсолютный каталог.
node apps/server/src/online/backup-cli.ts backup \
  --database-env BACKUP_DATABASE_URL --expect-database "$ONLINE_DB_NAME" \
  --directory "$BACKUP_DIR" --code-version "$RELEASE_COMMIT"

# Заранее создать НОВУЮ пустую тестовую БД на отдельном локальном тестовом PG18.
# Имя строго crowned_dogs_test_restore_<суффикс>, отдельные credentials/URL.
node apps/server/src/online/backup-cli.ts verify-restore \
  --database-env RESTORE_DATABASE_URL --expect-database "$RESTORE_DB_NAME" \
  --directory "$BACKUP_DIR"
```

Копия: `archive.dump` (custom format) и `manifest.json`. Каталог создаётся эксклюзивно (0700), файлы 0600; на Windows права дополнительно определяются ACL родительского каталога. Существующий каталог не перезаписывается. При сбое остаётся неполная копия; отсутствие корректного manifest означает, что её нельзя считать пригодной. CLI не удаляет копии и не делает автоматическую ротацию.

Один repeatable-read snapshot экспортируется в `pg_dump`. По тому же snapshot записываются версии `public.online_schema_migrations`, состав таблиц/столбцов, число строк и SHA-256 всех строк каждой пользовательской таблицы (включая другие пользовательские схемы, партиции и будущие таблицы). Значения строк в manifest отсутствуют. Сортировка детерминирована, строки читаются порциями. PostgreSQL-последовательности, функции, ограничения и индексы входят в архив; отпечаток сверяет таблицы/столбцы/данные/миграции, не доказывает эквивалентность всех функций или sequence state. Native-тест дополнительно проверяет продолжение identity sequence после restore.

Перед restore проверяется SHA-256 архива. Цель — только loopback, PG18 и пустая БД с обязательным тестовым префиксом. `pg_restore --single-transaction --exit-on-error` читает stdin; отсутствуют `--clean`/`--create`. После восстановления отпечатки должны точно совпасть. Повтор в уже заполненную БД запрещён и не меняет её. Даже неудачную тестовую БД инструмент сам не удаляет. У тестового имени и credentials не должно быть рабочего runtime; на время drill к ней обращается только оператор. Восстанавливать разрешено лишь доверенные копии: SQL внутри архива исполняется сервером, checksum подтверждает целостность, но не происхождение.

В базе копируются аккаунты, профили, жетоны/allocator, хэши сессий, а после интеграции #18/#20 — PvP snapshot, receipts, ledger наград, outbox, nonce и настройки чатов. Они чувствительны: дамп и manifest не публикуются в GitHub/CI artifacts. `--no-owner --no-privileges` обеспечивает переносимость тестового восстановления; роли, пароли и GRANT для production нужно восстановить отдельно из защищённой конфигурации.

Не входят: глобальные роли PostgreSQL, systemd/Compose/Caddy/env, bot token, исходный legacy JSON, приватные звуки, сборки и код. Их резервируют отдельно, с версиями и ограниченным доступом. Обычный volume на том же VPS не является внешней копией.

Предлагаемая первая эксплуатационная цель: ежедневная проверенная копия + копия перед каждым выпуском; **RPO до 24 часов**, **целевой RTO до 60 минут**. RTO ещё нужно замерить на эксплуатационном drill. Это не обещание нулевой потери; потеря VPS уничтожит и его локальные dump. До живого выпуска назначить владельца внешнего зашифрованного хранилища, ключей восстановления, контроля возраста копии и ротации (например 7 ежедневных + 4 еженедельных). Проверять внешнюю копию периодическим restore. Для меньшего RPO отдельно настроить physical backup/WAL/PITR; ежедневный логический dump их не заменяет.

## 3. Шаблоны инфраструктуры

- `infra/online/postgres.compose.yaml`: отдельные PostgreSQL18 image/volume/network и loopback port; все имена и путь secret-файла явные. Для PG18 volume монтируется в `/var/lib/postgresql`. Не подключать старый или чужой volume. Image фиксируется утверждённым digest. `pg_isready` проверяет процесс БД, не схему.
- `infra/online/online.service.in`: заменить все `@…@` в частной копии, выбрать непривилегированного пользователя, защищённый EnvironmentFile, уже собранный checkout и точный Node. Миграции при каждом старте не запускаются. `infra/online/online.env.example` — только список конфигурации, не готовый секрет.
- `infra/online/project.Caddyfile`: только project web container. `PROJECT_WEB_PORT` сохраняет существующий внутренний порт; `ONLINE_UPSTREAM` — адрес, достижимый **из этого контейнера**. `handle` сохраняет `/r2/api/online` целиком. `127.0.0.1` внутри контейнера не указывает на хост. Root проверяет bridge/private bind, ACL и свободные порты; внешний доступ к API и БД напрямую закрыт. Общий gateway и соседние проекты не перезапускаются.

Runtime-роли выдать минимальные права по финальной схеме: CONNECT/USAGE, SELECT/INSERT/UPDATE/DELETE на нужные таблицы и USAGE/SELECT на нужные sequences. DDL остаётся migration-role, backup-role должна видеть все строки/таблицы. RLS/частичный доступ не допускаются для полной копии. Реальные GRANT проверяются координатором после интеграции всех миграций. Административные CLI не используют runtime-role по умолчанию.

Стык #20 подтверждён исполнителем: `apps/server/src/online/main.ts` после интеграции #18 объединяет API, единственный Telegram poller и outbox/deadline workers. Дополнительно обязательны `ONLINE_BOT_USERNAME` (без @), `ONLINE_CHAT_ID` (отрицательный числовой ID), `ONLINE_THREAD_ID` (положительный ID темы), `ONLINE_WEBAPP_URL` (тот же origin, без query/hash/userinfo). Не запускать второй `apps/server/src/main.ts` или `npm run bot`. Перед установкой unit координатор сверяет окончательный SHA/config: исходная основа #23 этих четырёх параметров ещё не читает.

IP-цепочка по контракту #18: project Caddy доверяет X-Forwarded-For только от **одного проверенного IP** `SHARED_GATEWAY_PEER`; `trusted_proxies_strict` выбирает адрес справа, не доверяя подставленному левому значению. `header_up X-Crowned-Client-IP {client_ip}` всегда заменяет входящий custom header. API принимает его только от явного списка конкретных project Caddy peer IP в `ONLINE_TRUSTED_PROXY_IPS` (через запятую, без CIDR/hostname; по умолчанию никому не доверяет). Эти два списка обозначают разные стороны соединений. Нельзя подставлять `private_ranges`, весь Docker subnet или `0.0.0.0/0`. Root проверяет реальные socket peers и ACL. Auth/unauth лимитируются по проверенному IP, а авторизованные игровые запросы — по account.id, чтобы два игрока не делили лимит одного proxy. [Опции доверия Caddy](https://caddyserver.com/docs/caddyfile/options#trusted-proxies).

## 4. Выпуск: заполняемый лист координатора

Неподтверждённое поле означает незавершённый выпуск. Эти шаги выполняет координатор, не агент данных.

| Поле | Что зафиксировать приватно |
|---|---|
| Release | точный merged SHA, digest PG18 image, Node/WASM версии |
| Схема | фактические версии до/после (`SELECT version FROM online_schema_migrations ORDER BY version`) |
| Данные | hash и дата legacy-файла, количество записей, пути двух защищённых копий |
| Restore | дата, отдельная цель, результат CLI, длительность, проверенная внешняя копия |
| Runtime | API service/port/bind; единственный poller service/entrypoint; владельцы конфигурации |
| Proxy | исходный и новый project Caddy config/hash, проверенный upstream из контейнера |
| Откат | предыдущий код/статическая сборка, совместимость со схемой, путь fallback |

1. Принять PR по #16/#18/#19/#20/#25 и получить зелёный общий CI с native PG18; отдельно проверить две реальные сессии/клиента. Не объявлять целый онлайн готовым по результату этого участка.
2. Подготовить и проверить отдельную БД/роль/volume; выполнить `docker compose --env-file "$COMPOSE_ENV" -f infra/online/postgres.compose.yaml config --quiet`. Файл env и secret лежат вне checkout. Проверить image digest, новый volume, закрытые порты, права и свободное место.
3. Собрать точный релиз (`npm ci`, `npm run check`, `npm run web:build`). Приватные Sonniss assets подключить из существующего серверного источника; их не отправлять в Git. Проверить наличие в итоговой приватной сборке. Сохранить предыдущий dist отдельно до переключения.
4. Остановить старый poller и убедиться, что `getUpdates` больше не запускается им. Заблокировать новые регистрации/бои на время cutover. Сохранить неизменённый legacy-файл и текущую конфигурацию. При повторном выпуске снять pre-migration dump, проверить restore и внешнюю копию.
5. Явно выполнить миграции принятого релиза: `npm run online:migrate` с migration-role. Зафиксировать версии. Для первого пустого PG снять копию после миграции, до импорта, затем провести dry-run и `--apply` импорта. Реальные ID/жетоны сверить приватно; не выводить их в issue. Повторный dry-run должен показать только `existing`.
6. Создать post-import backup и восстановить его в отдельную пустую тестовую БД. Подтвердить количества, уровень1/потолок4, сохранение жетонов и работоспособность следующей регистрации на тестовой копии. Тестовую регистрацию в рабочей БД не делать.
7. Проверить скоординированные API/poller entrypoints и конфигурацию Telegram из #20. Запустить один новый runtime/poller; старый остаётся остановлен. Проверить migrations readiness через API `/r2/api/online/health` напрямую по private bind и из project Caddy. Настроить доверенные proxy/rate limits согласно финальному HTTP-контракту: общий socket IP за proxy не является лимитом отдельного игрока.
8. `caddy validate` нового project config с реальными env, затем reload **только project Caddy**. Если новые env требуют пересоздания контейнера — заменить только project web с сохранением mount/port, не общий gateway. Проверить HTTPS API, cookie Path/Secure, wasm MIME, статические пути и приватное аудио. Поменять frontend на принятую сборку.
9. Открыть регистрацию. Провести полный бой двух аккаунтов, повторный вход, перезапуск между приказами, однократность награды и доставку в согласованную тему. Проверить journal без токенов/PII; подтвердить ровно один poller. Зафиксировать итог в #21 и только затем закрывать приёмку.

## 5. Откат

До открытия регистрации: остановить новый poller/API, закрыть API-маршрут или вернуть сохранённый project config/dist; вернуть прежний код можно только при подтверждённой совместимости с текущей схемой. После первого живого серверного прогресса старый файловый бот **не включать**: он выдаёт независимые жетоны и не хранит online прогресс.

Если совместимость кода со схемой неизвестна — оставить maintenance, сохранить текущий dump и подготовить forward fix/revert через отдельный PR. Откат кода сам по себе не откатывает SQL, финалы боёв и награды. Автоматического `down migration` здесь нет.

Восстановление старой копии означает потерю записей после её snapshot. Сначала restore и проверка в новом изолированном PG, затем отдельное решение координатора о переносе runtime на восстановленную БД. До переключения нужно разобрать уже отправленные Telegram-сообщения/outbox, чтобы не повторить внешние действия. Никогда не выполнять destructive restore поверх рабочей БД «для проверки».

## 6. Проверки участка

```bash
node --test apps/server/test/online-import.test.ts apps/server/test/online-import-cli.test.ts \
  apps/server/test/online-data-target.test.ts apps/server/test/online-backup.test.ts apps/server/test/online-backup-cli.test.ts
# Только dedicated loopback БД crowned_dogs_test или crowned_dogs_test_*; PG_BIN с clients18.
node --test apps/server/test/online-data-postgres.test.ts
# Необязательно: реальный Caddy2.10; CADDY_BIN — абсолютный путь executable.
node --test apps/server/test/online-proxy.test.ts
```

Native-сценарии используют `ONLINE_TEST_DATABASE_URL` и общий guard. Импорт создаёт случайную schema; backup создаёт две случайные test DB из `template0` (нужен CREATEDB у тестовой роли), в конце удаляет только свои цели. Без URL import-test явно skipped; без URL или PG_BIN backup-test явно skipped. В CI PG_BIN может указывать wrappers к клиентам18 внутри одноразового service container: передать PGDATABASE/PGUSER/PGPASSWORD/PGAPPNAME через env и явно настроить PGHOST/PGPORT внутри контейнера; dump идёт в stdout, restore из stdin. Не публиковать backup-каталоги как artifacts.

Проверено в разработке на настоящем PostgreSQL18.6: default dry-run, полный импорт, сохранение файла, idempotency/прогресс/сессии, конфликты, поздний rollback, фактическое ожидание allocator lock другой connection, полный dump/restore, checksum-отказ до записи, отказ заполненной/неподходящей цели, восстановление сессии и sequence. На Caddy2.10.2 выполнены настоящие HTTP-проверки поддельных XFF/custom header от доверенного и недоверенного peer, неизменного API path/query и fallback static route; без CADDY_BIN эти тесты явно skipped. Полные check/build и состояние CI фиксируются отдельно в PR. Контейнерные шаблоны и реальный cutover требуют проверки на целевом хосте; этот файл не утверждает их deployment.

Справка: [PostgreSQL18 pg_dump](https://www.postgresql.org/docs/18/app-pgdump.html), [pg_restore](https://www.postgresql.org/docs/18/app-pgrestore.html), [официальный image](https://hub.docker.com/_/postgres), [Caddy handle](https://caddyserver.com/docs/caddyfile/directives/handle).
