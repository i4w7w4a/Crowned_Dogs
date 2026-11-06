# Проверка постоянного PvP (#18)

Дата: 6 ноября 2026. Базовый кандидат сервера: `4f2fa83`; защита квот за proxy — `3369a75`.
Зависимости: основа #17 (`155ee2e`), PostgreSQL CI #21 (`fd30e07`), Telegram #20 (`eaed518`, `65b7770`, `da79f79`).
Клиент #19 и выпуск #21 проверяются координатором отдельно.

## RED → GREEN

| Поведение | Наблюдавшийся RED | Проверенный GREEN |
|---|---|---|
| Rust loading/ready, приказы, projection | операции server.online отсутствовали | 9 тестов reducer: отдельный loading deadline, первый приказ сохранён, один разрешённый ход |
| JSON-стык хода | integer запроса не совпадал с f64 ядра: stale_turn | следующий ход принимается после SQL/JSON round-trip |
| Тайные поля | futurePrivate проходил через вложенные fighter/weapon/report/own поля | вложенные allowlist и собственный приказ, seed/rng отсутствуют |
| Внешность | строка вместо объекта вызывала Rust panic | отказ invalid_command, профиль не изменён |
| Мгновенная сдача | syncGain выдавал XP без отчёта хода | обе награды нулевые до первого resolved turn; после — штатный syncGain |
| SQL бой | отсутствующее постоянное хранилище | один match при гонке принимающих, receipt replay, уникальный ledger, rollback всей финализации |
| Время после очереди SQL | ожидавшая блокировку команда давала 200 после deadline | 410 и максимум один просроченный ход, новый срок от текущих серверных часов |
| Browser bridge | отсутствующее durable подтверждение; поздний consume проходил | nonce hash, явный confirm, одна session при гонке, expiry после lock, rollback consume вместе с session |
| Outbox | отсутствующие leases; истёкший вызов оставался open при claim | fencing старого lease, очередь per invite, сохранение message/topic, expiry до публикации |
| HTTP | игровые маршруты давали 404 | mounted /r2/api/online: Mini App + browser login → один матч → общий итог, повторы и запрет подмены actor |
| Квоты за proxy | второй подтверждённый игрок получал 429 из общей socket-квоты | отдельная квота verified account.id; адрес для auth только от точного доверенного peer |

## Выполненные проверки кандидата

- Финальный `npm run check` с локальным native PostgreSQL: **275 passed, 0 failed**, всего 276; единственный skip — штатный GC-тест. Types и render/motion/scene orchestration пройдены. Предыдущий кандидат без test DB давал 272 passed и два дополнительных opt-in native skip.
- Оба native-файла отдельно на настоящем PostgreSQL **18.6**: **2 passed, 0 failed, 0 skipped**. Только изолированная loopback БД `crowned_dogs_test` и собственные случайные schema.
- Native проверяет несколько реальных соединений: конкурентное принятие, восемь повторов команды, новый pool после сохранения первого приказа, гонку submit/timeout, rollback ledger, восемь consume одного challenge, fencing истёкшей аренды.
- `npm run web:build`: production-сборка успешна.
- Limiter: 6 сфокусированных config/HTTP сценариев прошли. Два verified аккаунта не делят socket-квоту; spoof заголовка от недоверенного peer не обходит auth limit, CIDR-конфигурация отвергается.
- Полный `cargo test -p game-client -p game-core` пройден при первом reducer; после основы и укрепления проекции повторены затронутые `online` и `sync_cap`.

Для воспроизведения native-проверки использовать отдельную `ONLINE_TEST_DATABASE_URL`, не production-переменную:

```text
node --test --test-concurrency=1 apps/server/test/online-postgres.test.ts apps/server/test/online-pvp-postgres.test.ts
```

Тестовый guard отвергает удалённый host, неподходящее имя БД и query overrides. Кластер не удаляется;
каждый тест удаляет только свою валидированную случайную schema. Секреты и реальные профили в fixture не используются.

## Границы результата

Это доказательство серверного поведения в изолированных тестах. Живой bot poller, отправка в группу,
production-БД и полный бой двух реальных Telegram-аккаунтов в 3D здесь не запускались. Reconnect проверен
новым экземпляром сервиса/пулом и reopen embedded БД; аварийное выключение production-хоста не имитировалось.

Короткие PvP-транзакции сериализованы одним row lock: пилот без заявленной нагрузочной готовности.
Старые receipts/history сохраняются; retention требует отдельной эксплуатационной политики.
Видимый дубль Telegram при сбое между send и сохранением message ID возможен, но token и бой остаются одними.
Импорт прежних жетонов, резервное восстановление, proxy-топология, объединение клиента и выпуск — задачи #25/#21.
