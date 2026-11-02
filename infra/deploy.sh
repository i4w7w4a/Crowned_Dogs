#!/usr/bin/env bash
set -Eeuo pipefail

PROJECT_DIR="${CROWNED_DOGS_PROJECT_DIR:-/srv/crowned-dogs}"
LOCK_FILE="${CROWNED_DOGS_LOCK_FILE:-/run/lock/crowned-dogs-deploy.lock}"
BOT_SERVICE="${CROWNED_DOGS_BOT_SERVICE:-crowned-dogs-bot.service}"
WEB_CONTAINER="${CROWNED_DOGS_WEB_CONTAINER:-crowned-dogs-web}"
PUBLIC_URL="${CROWNED_DOGS_PUBLIC_URL:-https://app.4-i-7.ru/r2/}"

log() {
  printf '[deploy] %s\n' "$*"
}

fail() {
  printf '[deploy] ОШИБКА: %s\n' "$*" >&2
  exit 1
}

trap 'printf "[deploy] ОШИБКА: команда прервана на строке %s\n" "$LINENO" >&2' ERR

exec 9>"$LOCK_FILE"
flock -n 9 || fail 'другой деплой уже выполняется'

[[ -d "$PROJECT_DIR/.git" ]] || fail "не найден Git-репозиторий: $PROJECT_DIR"
cd "$PROJECT_DIR"

remote="$(git remote get-url origin)"
case "$remote" in
  'https://github.com/code-of-stalker/Crowned-Dogs.git'|'git@github.com:code-of-stalker/Crowned-Dogs.git') ;;
  *) fail "неожиданный origin: $remote" ;;
esac

[[ "$(git branch --show-current)" == 'main' ]] || fail 'на сервере должна быть ветка main'
git diff --quiet || fail 'на сервере есть незакоммиченные правки; деплой остановлен'
git diff --cached --quiet || fail 'на сервере есть индексированные правки; деплой остановлен'

if [[ -f /etc/crowned-dogs/ssh_config ]]; then
  export GIT_SSH_COMMAND='ssh -F /etc/crowned-dogs/ssh_config'
fi

log 'Получаю main из GitHub'
git fetch origin main
git merge --ff-only FETCH_HEAD

log 'Устанавливаю зависимости по lockfile'
npm ci

log 'Проверяю типы, тесты и рендер'
npm run check

log 'Собираю браузерную версию'
npm run web:build

log 'Перезапускаю Telegram-бота'
systemctl restart "$BOT_SERVICE"
systemctl is-active --quiet "$BOT_SERVICE"

log 'Перезапускаю веб-контейнер'
docker restart "$WEB_CONTAINER" >/dev/null

health=''
for attempt in {1..12}; do
  health="$(docker inspect --format '{{.State.Status}} {{.State.Health.Status}}' "$WEB_CONTAINER")"
  [[ "$health" == 'running healthy' ]] && break
  sleep 5
done
[[ "$health" == 'running healthy' ]] || fail "веб-контейнер не стал healthy: $health"

curl -fsSI "$PUBLIC_URL" >/dev/null

log "Готово: $(git rev-parse --short HEAD)"
log "Бот: $(systemctl is-active "$BOT_SERVICE"); веб: $health"
