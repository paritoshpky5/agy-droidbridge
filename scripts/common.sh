#!/usr/bin/env bash
set -euo pipefail
umask 077

state=${AGY_STATE_DIR:-"$HOME/.local/share/agy-droidbridge"}
pid_file="$state/proxy.pid"
binary="$state/agy-droidbridge"
workspace="$state/workspace"

die() {
  printf '%s\n' "$*" >&2
  exit 1
}

lock_state() {
  mkdir -p -- "$state"
  chmod 700 -- "$state"
  exec 9>"$state/lock"
  flock -x 9
}

read_port() {
  port=${AGY_PORT:-8766}
  if [[ -z ${AGY_PORT:-} && -f "$state/port" ]]; then
    read -r port < "$state/port"
  fi
  [[ $port =~ ^[1-9][0-9]{0,4}$ ]] && (( port <= 65535 )) || die 'Invalid AGY_PORT.'
}

process_start() {
  local pid=$1 stat
  local -a fields
  [[ $pid =~ ^[1-9][0-9]*$ && -r /proc/$pid/stat ]] || return 1
  IFS= read -r stat < "/proc/$pid/stat" || return 1
  read -ra fields <<< "${stat##*) }"
  [[ ${fields[0]:-Z} != Z && -n ${fields[19]:-} ]] || return 1
  printf '%s\n' "${fields[19]}"
}

running() {
  [[ -f $pid_file ]] || return 1
  read -r pid launched < "$pid_file" || return 1
  [[ $launched == "$(process_start "$pid")" && -n $launched ]]
}

read_cli() {
  local configured=${AGY_BIN:-}
  if [[ -z $configured && -f "$state/cli" ]]; then
    read -r configured < "$state/cli"
  fi
  cli=$(command -v -- "${configured:-agy}") ||
    die 'Antigravity CLI was not found. Use the one-paste Termux setup in the README or set AGY_BIN to a working executable.'
  [[ -f $cli && -x $cli ]] || die 'Antigravity CLI is not an executable file.'
  cli=$(realpath -- "$cli")
}

check_cli() {
  local cli_help flag
  if ! cli_help=$(timeout 30 env -u LD_PRELOAD "$1" --help 2>&1); then
    printf 'Antigravity CLI could not run from %s.\n%s\n' "$1" "$cli_help" >&2
    return 1
  fi
  for flag in --agent --disable-slash-commands --output-format --model; do
    if [[ $cli_help != *"$flag"* ]]; then
      printf 'The installed Antigravity CLI does not list %s. Update agy, then run install again.\n' "$flag" >&2
      return 1
    fi
  done
}

prepare_workspace() {
  [[ -x $binary ]] || die 'Run ./agy-droidbridge install first.'
  "$binary" --prepare-workspace "$workspace"
}

agent_available() {
  local agents
  agents=$(cd -- "$workspace" && timeout 10 "$cli" agents 2>/dev/null) || return 1
  grep -Fxq 'agy-droidbridge-text' <<< "$agents"
}

open_workspace() (
  printf 'Sign in to Antigravity if prompted, approve this DroidBridge workspace, then exit Antigravity to continue.\n'
  cd -- "$workspace"
  exec 9>&-
  exec "$cli"
)

auth_probe() {
  local key=$1
  curl --noproxy '*' --silent --fail --connect-timeout 1 --max-time 2 \
    --output /dev/null "http://127.0.0.1:$port/healthz" || return
  printf 'Authorization: Bearer %s\n' "$key" |
    curl --noproxy '*' --silent --fail --connect-timeout 1 --max-time 2 \
      --header @- --output /dev/null "http://127.0.0.1:$port/v1/models"
}

start_proxy() {
  if running; then
    printf 'Bridge already running (PID %s).\n' "$pid"
    return
  fi
  [[ -x $binary ]] || die 'Run ./agy-droidbridge install first.'
  read_port
  read_cli
  prepare_workspace
  if ! agent_available; then
    die 'Antigravity has not loaded the DroidBridge text agent. Run ./agy-droidbridge trust, approve the DroidBridge workspace in Antigravity, exit, then run ./agy-droidbridge start.'
  fi
  [[ -s "$state/api-key" ]] || die 'Bridge API key is missing. Run setup.'
  local key attempt
  read -r key < "$state/api-key"
  if curl --noproxy '*' --silent --connect-timeout 1 --max-time 2 \
      --output /dev/null "http://127.0.0.1:$port/healthz"; then
    die 'The configured port already serves HTTP.'
  fi

  (
    if [[ -z ${AGY_MODELS_FILE:-} && -s "$state/models.txt" ]]; then
      export AGY_MODELS_FILE="$state/models.txt"
    fi
    export AGY_HOST=127.0.0.1 AGY_PORT="$port" AGY_BIN="$cli" AGY_API_KEY="$key" AGY_WORKSPACE="$workspace"
    exec nohup "$binary"
  ) </dev/null >>"$state/bridge.log" 2>&1 9>&- &
  pid=$!

  for ((attempt = 0; attempt < 30; attempt++)); do
    if process_start "$pid" >/dev/null && auth_probe "$key" && launched=$(process_start "$pid"); then
      printf '%s %s\n' "$pid" "$launched" > "$pid_file"
      printf '%s\n' "$port" > "$state/port"
      printf 'Bridge running at http://127.0.0.1:%s/v1 (PID %s).\n' "$port" "$pid"
      return
    fi
    sleep 0.2
  done
  if launched=$(process_start "$pid"); then
    printf '%s %s\n' "$pid" "$launched" > "$pid_file"
    stop_proxy
  fi
  die 'Bridge did not pass its authenticated startup check. See the private bridge.log.'
}

stop_proxy() {
  if ! running; then
    rm -f -- "$pid_file"
    printf 'Bridge is stopped.\n'
    return
  fi
  kill -TERM "$pid" || die 'Unable to signal the running proxy.'
  for ((attempt = 0; attempt < 32; attempt++)); do
    if ! running; then
      rm -f -- "$pid_file"
      printf 'Bridge stopped.\n'
      return
    fi
    sleep 0.2
  done
  die 'Bridge did not stop before the deadline.'
}

status_proxy() {
  if ! running; then
    printf 'Bridge is stopped.\n'
    return 1
  fi
  read_port
  local key
  read -r key < "$state/api-key"
  if ! auth_probe "$key"; then
    printf 'Bridge PID %s is running but the localhost API is unavailable.\n' "$pid"
    return 1
  fi
  printf 'Bridge running at http://127.0.0.1:%s/v1 (PID %s).\n' "$port" "$pid"
}
