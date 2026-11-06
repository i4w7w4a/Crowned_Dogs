# Контракт онлайн v1

Задачи: #16, #18, #19, #20. База API `/api/online`, внешний mount `/r2/api/online`.
JSON camelCase, время в Unix milliseconds. Сессия определяет игрока, `playerId` в теле не принимается.

## Общие правила

Мутации имеют UUID `commandId`, существующие сущности — `expectedVersion`, бой — также `turn`.
Повтор идентичной команды возвращает сохранённые HTTP status/body до проверки версии и времени.
Изменённое тело с тем же ID даёт `command_id_conflict`. После потери ответа повторять прежний запрос;
после 409 загрузить актуальную сущность и отправить новое намерение с новым ID.

Ответы содержат `serverNow`; историческая квитанция сохраняет прежнее время. После неё GET возвращает
актуальную проекцию. Время устройства влияет только на отображение оставшегося срока.

Ошибки: `{error:code}`: 400 `invalid_command`/`incomplete_order`, 401 `unauthorized`,
404 `not_found` (в том числе чужой бой), 409 `stale_version`/`stale_turn`/`busy`/`already_submitted`/
`invite_closed`/`command_id_conflict`, 410 `deadline_passed`, 503 `incompatible_ruleset`.

## Профиль

Account: `{id,tag,telegram:{id,firstName,lastName,username},profile,appearance,loadout,version}`.
Это существующий формат основы аккаунтов. GET `/me` возвращает `{account,maxLevel}`.
POST `/profile/commands`: `{commandId,expectedVersion,action}` -> `{account,maxLevel}`.
Действия: `{type:"spend",stat}`, `{type:"appearance",value}` (полная проверенная внешность,
жетон не меняется), `{type:"equip",slot,itemId}`, `{type:"mod",slot,itemId}`,
`{type:"paint",itemId,color}`. Изменения профиля при занятом слоте отклоняются `busy`.

PublicPlayer: `{id,tag,name,telegram:{firstName,lastName,username},level,appearance,loadout}`.
Telegram ID соперника не публикуется. Профили, внешность, снаряжение и правила фиксируются при принятии.

## Вызов и бой

Invite: `{id,token,version,status,host:PublicPlayer,expiresAt,matchId,delivery:{status,error},serverNow}`.
Статусы: open/accepted/cancelled/expired; доставка pending/sending/published/failed.
Операторская группа не берётся из запроса игрока. Автор занимает слот при создании вызова.

- POST `/invites` `{commandId}` -> `{invite}`, срок 30 минут.
- GET `/invites/:token` -> `{invite}` после входа.
- POST `/invites/:token/accept|cancel|retry` `{commandId,expectedVersion}` -> `{invite,battle?}`.
  Retry разрешён владельцу открытого вызова с failed доставкой и сохраняет token.
- GET `/activity` -> `{invite:null|Invite,battle:null|Battle}`.
- GET `/matches/:id` -> `{battle}`; GET `/matches` -> последние 30 собственных итогов.
- POST `/matches/:id/commands` `{commandId,expectedVersion,turn,action}` -> `{battle}`.

Battle: `{schemaVersion:1,ruleset:"online-v1",id,version,phase,side,players:[PublicPlayer,PublicPlayer],
ready:[bool,bool],loadingDeadline,deadline,match,own:{draft,order,submitted},submitted:[bool,bool],
result,reward,cancelReason,eventCursor,serverNow}`. Фазы loading/active/finished/cancelled.
Действия: `{type:"ready"}`, `{type:"press",command}`, `{type:"submit"}`, `{type:"surrender"}`.
`command` — существующая команда черновика zone/guard/step/run/dash/knife/grenade/swap/flee.

Canonical side всегда 0/1. Перестановка в local0 делается одним клиентским адаптером, не сервером.
`match.history` полный и упорядоченный, `eventCursor=history.length`; записи имеют номер `turn`.
Сервер выдаёт явную вложенную проекцию match/config/fighters/history/hazard. Seed/rng и чужой
текущий черновик/приказ не выходят в сеть. Поля, добавленные в закрытый snapshot, не становятся публичными.

Loading — 5 минут. До ready обоих нет боевого срока. Loading timeout отменяет бой без проигравшего и наград.
Active deadline задаёт существующий Rust turnSeconds. Поздняя команда может атомарно разрешить один
просроченный ход, но сама отвергается. После простоя разрешается максимум один ход, следующий срок от now.
Surrender заканчивает активный бой с reason=surrender; до первого разрешённого хода опыт/мастерство нулевые.
После первого отчёта обе награды рассчитывает syncGain; ledger, профили, финал и слоты фиксируются вместе.

## Telegram

Канонический порт — `telegram-outbox.ts` из направления #20:
`claimDue(now,leaseMs)` -> `{leaseId,kind,attempts,chatId,messageId,card}` или null;
`complete(leaseId,messageId|null)` и `fail(leaseId,{code,retryAt|null})` -> boolean CAS.
Card содержит token/status/host (telegram,level,tag)/expiresAt. `leaseId` — fencing token;
старый worker не подтверждает новый lease. Один активный claim на приглашение. messageId хранится
в приглашении и переживает повтор/редактирование. Сеть выполняется за пределами транзакции.

SQL события создаются вместе с изменением вызова. Worker читает актуальную committed карточку,
может пропустить закрытый неопубликованный вызов. Видимая отправка Telegram допускает дубль при
сбое между send и commit; token общий, дополнительный бой не создаётся.

## Вход обычного браузера

POST `/auth/browser/start` `{inviteToken?}` -> `{challengeId,botUrl,expiresAt}`. TTL 5 минут.
GET `/auth/browser/status` -> `{status:"pending"|"confirmed"|"expired",expiresAt}`.
POST `/auth/browser/consume` `{}` -> `{account,maxLevel,inviteToken}` и штатная session cookie.
Nonce только в HttpOnly SameSite=Lax cookie и хеше БД; URL содержит отдельный случайный challenge ID.
Бот `/start login_<id>` показывает запрос, callback `login:<id>` явно подтверждает Telegram identity.
BrowserLoginPort: describe(id,now), confirm(id,identity,now). Подтверждение другой identity отвергается.
Consume доступен только исходному браузеру, атомарен и одноразов. Сам `/start` не авторизует.

## Хранение и обновление

Пилот сериализует короткие PvP транзакции одним служебным row lock; это сознательный предел
пропускной способности, а не подтверждение нагрузочной готовности. Telegram и внешняя сеть под lock не выполняются.
Дополнительно действуют уникальные ограничения слота игрока, квитанции и награды (бой,игрок).
Ruleset/schema проверяются перед переходом и проекцией. Несовместимые активные snapshot не мигрируют молча:
обновление после завершения боёв либо отдельной явной миграцией. При изменении правил увеличивать ruleset.
