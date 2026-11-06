# Приёмка клиента #19

Код: `f54803fb854a801c888fb93223358015254bc814`. Draft PR: #27. Дата: 6 ноября 2026.

| Проверка | Результат |
|---|---|
| `npm run check` | 248 тестов: 246 passed, 0 failed, 2 skipped; TypeScript и render/motion/scene suites зелёные |
| `npm run web:build` | Успешная production-сборка |
| `cargo test -p game-client -p game-wasm` | 54 passed, 1 ignored; native-тесты клиентских crate, без серверной БД |
| Самостоятельные проверки сетевой границы | Lost reply/reload, аккаунтные квитанции, stale snapshot, разделение poll/animation, server-clock, намерения профиля, server-only чтение профиля |
| Публичные поля | Hostile `<img src=x onerror="alert(1)"><b>name</b>` сохранён как текст, кодируется в настоящем `ui.fightLog`; headers/base/result используют textContent |
| Персоны одного пола | Реальные Three.js skin/eye материалы для male+male и female+female независимы; исходные материалы asset cache не изменяются; beard-loading regressions зелёные |

Полные локальные логи лежат в игнорируемых `target/online-check-final.log` и `target/online-build-final.log`.

Дополнение ревью: `1db3796` распознаёт backend `browser_login_expired`, полный check после этой правки — 247 passed / 2 skipped, build зелёный. `f31123a` проверяет настоящий `OnlineSession.login()` через status confirmed → consume410 → видимый текст/кнопку → явный restart → второй start с тем же inviteToken → consume200. Два узких теста зелёные; независимая проверка направления #20 также 2/2. Типы web и web build после переноса CSS/baseURL прошли. Fixtures используют валидные 32-символьные synthetic tokens. Live Telegram в этих тестах не участвует.

## Видимые результаты

| Сценарий | Снимок |
|---|---|
| Side 1: «Соль» — игрок, «Искра» — соперник, desktop | `research/online-client/side1-desktop.jpg` |
| Side 1 на 390×844, две мужские Персоны | `research/online-client/side1-mobile.jpg` |
| Server result/reward после сохранённого отчёта | `research/online-client/side1-result.jpg` |
| Side 0: «Искра» — игрок, «Соль» — соперник, 390×844, q=low | `research/online-client/side0-mobile.jpg` |

Проверены выбор головы для атаки, две зоны защиты, подтверждение приказа, показ отчёта и surrender-итога. В меню заблокированы оружие, карта, время и погода; «Сдаться» и «Уйти на базу (бой продолжается)» различаются. Все просмотры без звука (`?muted=1`). Для снимка стороны 0 после повторной загрузки ожидался видимый готовый кадр; промежуточный loader-снимок удалён и не используется как доказательство.

Стенд — отдельный `tools/online-client-preview.mjs` на loopback. Он воспроизводит публичные fixtures, созданные настоящим Rust reducer в серверном коммите `ad0b6e0`. Это проверка **рендера и протокольных границ**, не полноценный двухпользовательский матч: fixture-процесс не рассчитывает новую серверную игру и не начисляет прогресс.

## Оставшаяся общая приёмка #21

Два настоящих Telegram-аккаунта, публикация в игровой теме, browser bridge через настоящего бота, SQL-транзакции/рестарт и полный бой двух браузеров проверяются после соединения с #18/#20. В этой задаче production, живой бот и БД не изменялись. Независимое ревью и GitHub CI выполняются координатором/Actions; локальные проверки не выдаются за их результат.
