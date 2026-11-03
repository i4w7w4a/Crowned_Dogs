#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEPLOY="$ROOT/infra/deploy.sh"

fail() {
  printf 'FAIL: %s\n' "$1" >&2
  exit 1
}

assert_contains() {
  local haystack="$1" needle="$2"
  case "$haystack" in
    *"$needle"*) ;;
    *) fail "Ожидалось: $needle" ;;
  esac
}

assert_not_contains() {
  local haystack="$1" needle="$2"
  case "$haystack" in
    *"$needle"*) fail "Не ожидалось: $needle" ;;
    *) ;;
  esac
}

[[ -f "$DEPLOY" ]] || fail "infra/deploy.sh отсутствует"
bash -n "$DEPLOY"

content="$(<"$DEPLOY")"
attributes="$(<"$ROOT/.gitattributes")"
assert_contains "$attributes" '*.sh text eol=lf'
assert_contains "$content" 'set -Eeuo pipefail'
assert_contains "$content" 'flock -n 9'
assert_contains "$content" 'git fetch origin main'
assert_contains "$content" 'git merge --ff-only FETCH_HEAD'
assert_contains "$content" 'git diff --quiet'
assert_contains "$content" 'git diff --cached --quiet'
assert_contains "$content" 'npm ci'
assert_contains "$content" 'npm run check'
assert_contains "$content" 'npm run web:build'
assert_contains "$content" 'systemctl restart'
assert_contains "$content" 'docker restart'
assert_contains "$content" 'running healthy'
assert_contains "$content" 'curl -fsSI'
assert_not_contains "$content" 'reset --hard'
assert_not_contains "$content" 'push --force'

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
mkdir -p "$TMP/project/.git" "$TMP/bin"
: > "$TMP/commands.log"

cat > "$TMP/bin/mock" <<'MOCK'
#!/usr/bin/env bash
set -u
name="$(basename "$0")"
printf '%s %s\n' "$name" "$*" >> "$MOCK_LOG"
if [[ "$name" == git ]]; then
  case "$*" in
    'remote get-url origin')
      printf '%s\n' "${MOCK_REMOTE:-https://github.com/code-of-stalker/Crowned-Dogs.git}"
      ;;
    'branch --show-current')
      printf '%s\n' 'main'
      ;;
    'diff --quiet'|'diff --cached --quiet')
      [[ "${MOCK_DIRTY:-0}" == 1 ]] && exit 1
      ;;
  esac
fi
if [[ "${MOCK_FAIL_AT:-}" == "$name $*" ]]; then
  exit 42
fi
if [[ "$name" == docker && "$1" == inspect ]]; then
  printf '%s\n' 'running healthy'
fi
exit 0
MOCK
chmod +x "$TMP/bin/mock"
for command in git npm systemctl docker curl flock sleep; do
  ln -s "$TMP/bin/mock" "$TMP/bin/$command"
done

run_deploy() {
  MOCK_LOG="$TMP/commands.log" \
  CROWNED_DOGS_PROJECT_DIR="$TMP/project" \
  CROWNED_DOGS_LOCK_FILE="$TMP/deploy.lock" \
  PATH="$TMP/bin:$PATH" \
  "$@" bash "$DEPLOY"
}

run_deploy env
log="$(<"$TMP/commands.log")"
assert_contains "$log" 'npm ci'
assert_contains "$log" 'npm run check'
assert_contains "$log" 'npm run web:build'
assert_contains "$log" 'systemctl restart crowned-dogs-bot.service'
assert_contains "$log" 'docker restart crowned-dogs-web'
assert_contains "$log" 'curl -fsSI https://app.4-i-7.ru/r2/'

: > "$TMP/commands.log"
run_deploy env MOCK_REMOTE='https://github.com/i4w7w4a/Crowned_Dogs.git'
log="$(<"$TMP/commands.log")"
assert_contains "$log" 'systemctl restart crowned-dogs-bot.service'

: > "$TMP/commands.log"
if run_deploy env MOCK_DIRTY=1; then
  fail 'Деплой не остановился при серверных правках'
fi
log="$(<"$TMP/commands.log")"
assert_not_contains "$log" 'npm ci'
assert_not_contains "$log" 'systemctl restart'

: > "$TMP/commands.log"
if run_deploy env MOCK_FAIL_AT='npm run check'; then
  fail 'Деплой не остановился при ошибке проверок'
fi
log="$(<"$TMP/commands.log")"
assert_not_contains "$log" 'systemctl restart'
assert_not_contains "$log" 'docker restart'

printf 'PASS: синтаксис, обязательные шаги, порядок и безопасные отказы\n'
