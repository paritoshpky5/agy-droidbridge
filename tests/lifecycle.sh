#!/usr/bin/env bash
set -euo pipefail

root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)
launcher="$root/agy-droidbridge"
sandbox=$(mktemp -d)
export AGY_STATE_DIR="$sandbox/state"
export AGY_BIN="$sandbox/bin/agy"
export PREFIX=/data/data/com.termux/files/usr
export PATH="$sandbox/bin:$PATH"
unset AGY_MODELS_FILE AGY_PORT
mkdir -p "$sandbox/bin" "$AGY_STATE_DIR"

cleanup() {
  "$launcher" stop >/dev/null 2>&1 || true
  rm -f -- "$sandbox/result" "$sandbox/fake-binary" "$sandbox/bin/"{agy,curl,flock,getprop,uname}
  rm -f -- "$AGY_STATE_DIR/"{agy-droidbridge,api-key,bridge.log,cli,lock,models.txt,port,proxy.pid,test-mode,test-running,test-catalog,test-trusted}
  rmdir -- "$AGY_STATE_DIR/workspace" 2>/dev/null || true
  rmdir -- "$sandbox/bin" "$AGY_STATE_DIR" "$sandbox" 2>/dev/null || true
}
trap cleanup EXIT

for script in "$launcher" "$root"/scripts/*.sh; do bash -n "$script"; done
"$launcher" --help | grep -F 'install [--interactive] [arm64-binary]'
if "$launcher" status extra >"$sandbox/result" 2>&1; then exit 1; fi
grep -F 'takes no arguments' "$sandbox/result"

cat > "$sandbox/bin/uname" <<'EOF'
#!/usr/bin/env bash
printf 'aarch64\n'
EOF
cat > "$sandbox/bin/getprop" <<'EOF'
#!/usr/bin/env bash
printf '34\n'
EOF
cat > "$sandbox/bin/flock" <<'EOF'
#!/usr/bin/env bash
[[ "$*" == '-x 9' ]]
EOF
cat > "$sandbox/bin/curl" <<'EOF'
#!/usr/bin/env bash
[[ -f "$AGY_STATE_DIR/test-running" ]]
EOF
cat > "$sandbox/bin/agy" <<'EOF'
#!/usr/bin/env bash
case "${1:-}" in
  --help) printf 'usage: agy --agent --output-format --model\n' ;;
  models) cat "$AGY_STATE_DIR/test-mode" ;;
  agents) if [[ -f "$AGY_STATE_DIR/test-trusted" ]]; then printf 'agy-droidbridge-text\n'; fi ;;
  '') touch "$AGY_STATE_DIR/test-trusted" ;;
esac
EOF
chmod +x "$sandbox/bin/"*
if "$launcher" install "$sandbox/missing" >"$sandbox/result" 2>&1; then exit 1; fi
grep -F 'binary file not found' "$sandbox/result"
if "$launcher" install >"$sandbox/result" 2>&1; then exit 1; fi
grep -F 'does not list --disable-slash-commands' "$sandbox/result"
sed 's/--agent /--agent --disable-slash-commands /' "$sandbox/bin/agy" > "$sandbox/result"
cat "$sandbox/result" > "$sandbox/bin/agy"

cat > "$sandbox/fake-binary" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
if [[ ${1:-} == --check-models ]]; then
  grep -Eq '^valid-model[[:space:]]+Valid model$' "$2"
  exit
fi
if [[ ${1:-} == --prepare-workspace ]]; then
  mkdir -p -- "$2"
  exit
fi
[[ $AGY_HOST == 127.0.0.1 && $AGY_PORT == 18766 ]]
[[ $AGY_API_KEY =~ ^[a-f0-9]{64}$ ]]
[[ $AGY_WORKSPACE == "$AGY_STATE_DIR/workspace" ]]
printf '%s\n' "${AGY_MODELS_FILE:-default}" > "$AGY_STATE_DIR/test-catalog"
trap 'rm -f -- "$AGY_STATE_DIR/test-running"; exit 0' TERM
touch "$AGY_STATE_DIR/test-running"
while sleep .2; do :; done
EOF

if AGY_PORT=18766 "$launcher" install "$sandbox/fake-binary" >"$sandbox/result" 2>&1; then exit 1; fi
grep -F './agy-droidbridge trust' "$sandbox/result"
"$launcher" trust
"$launcher" start
key_before=$(cat "$AGY_STATE_DIR/api-key")
"$launcher" status
[[ $(cat "$AGY_STATE_DIR/test-catalog") == default ]]
printf 'valid-model  Valid model\n' > "$AGY_STATE_DIR/test-mode"
"$launcher" models
[[ $(cat "$AGY_STATE_DIR/test-catalog") == "$AGY_STATE_DIR/models.txt" ]]
printf 'invalid flag\n' > "$AGY_STATE_DIR/test-mode"
if "$launcher" models >"$sandbox/result" 2>&1; then exit 1; fi
grep -F 'saved list is unchanged' "$sandbox/result"
grep -Fx 'valid-model  Valid model' "$AGY_STATE_DIR/models.txt"
"$launcher" status
"$launcher" restart
[[ $(cat "$AGY_STATE_DIR/api-key") == "$key_before" ]]
"$launcher" stop
if "$launcher" status; then exit 1; fi
rm -f -- "$AGY_STATE_DIR/test-trusted"
"$launcher" install --interactive "$sandbox/fake-binary"
[[ -f "$AGY_STATE_DIR/test-trusted" ]]
"$launcher" status
"$launcher" stop
printf 'Installer, lifecycle, and model refresh passed with local shims.\n'
