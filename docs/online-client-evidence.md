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

Два человеческих Telegram-аккаунта, публикация в игровой теме и browser bridge через настоящего бота остаются отдельной приёмкой. В этой задаче production, живой бот и production-БД не изменялись.

## Настоящие API и PostgreSQL: две фазы приёмки

6 ноября 2026, loopback `127.0.0.1:4318/r2/`, native PostgreSQL 18, два изолированных Chromium context. Подписи двух synthetic Telegram users проверял настоящий сервер. A получил штатную HttpOnly cookie через signed auth; B вошёл через официальный Telegram SDK и launch hash. Игровые команды и изменение профиля выполнялись обычными UI-кликами; GET API использовался для проверки. Telegram poller/delivery worker выключены.

Первая фаза на общем `964be3b8`: SDK auth200; сдвиг injected test clock на 301000 мс дал auth401 при неизменном verifier TTL, после чего тот же аккаунт восстановился через live cookie и `/me`. До боя часы возвращены в 0. Обе внешности и очко мощности A сохранены через UI и пережили reload. Затем обнаружен настоящий blocker: modal loader перекрывал лобби. Снимок `real/lobby-loader-red.png` фиксирует его; force-click не применялся.

Исправление `bfc90fb` переносит выбор дуэли до modal loader, сохраняя отмену, retry и готовность 3D. Regression actual RustApplication сначала упал, затем прошёл; 9/9 с `--expose-gc`. Полный локальный check: 260 тестов, 258 passed, 0 failed, 2 skipped; TypeScript/render/motion/scene зелёные. Scoped review координатора одобрен. Auth fixes `260b12e`/`70c1602`: 7/7 узких проверок, включая настоящий login loop. PR27 CI на `25a60d9` завершился check/windows success; общая ветка проверяется отдельно.

Вторая фаза на общей сборке `4977124b08d8c34843b0305bf9eac300b15b0a85` продолжила работу с сохранённых профилей и прошла:

| Проверка | Наблюдение |
|---|---|
| Создание и принятие | Открытие ссылки не принимает вызов; отдельная кнопка accept создаёт loading snapshot с deadline=null и ready=[false,false] |
| Готовность | Оба ready подтверждены; одна штатная гонка дала 409, клиент сам восстановил ready до 200 |
| Приказы | Атака/две защиты/submit обоих игроков отправлены через UI; второй submit дал ровно один одинаковый canonical report |
| Restart между submit | API перезапущен за 1413 мс, та же schema/origin; cookie, match, own.submitted и исходный deadline сохранены; часов в бою не меняли |
| Reload и меню | Активный матч восстановлен; выбор оружия/карты заблокирован; открытие меню не очищает deadline |
| Сдача и результат | Явный confirm → surrender; одинаковый winner/reason/history у обоих игроков |
| Повтор финала | Профили до/после reload полностью совпали, повторного начисления нет |
| Финальные профили | A: уровень1, XP40, 2 свободных очка, мощность1; B: уровень2, XP0, 6 свободных очков. Жетоны сохранены |
| Ошибки браузера | 0 pageerror; браузеры закрыты, test clock=0 |

Первоначальные ready/profile снимки попали в промежуточный poll или анимацию загрузки. Они не используются как чистое визуальное доказательство. Итоги проверены визуально; профили пересняты после завершения анимации в отдельном проходе без игровых/profile POST, с подтверждением неизменности `/me`. Во время ready-race кратко видны `00:00` и техническое `stale_version`; последующие обычные команды и серверный таймер работают.

Артефакты: `research/online-client/real/acceptance.json`, `telegram-cookie-resume.png`, `side0-result.png`, `side1-result.png`, `actor0-profile.png`, `actor1-profile.png`. JSON не содержит initData, cookies, token бота или секретных приказов. Полные ignored runner/logs находятся в `target/real-ui-evidence*`; production bypass отсутствует.
