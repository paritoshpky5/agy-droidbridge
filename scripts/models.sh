#!/usr/bin/env bash
set -euo pipefail

source "$(dirname -- "${BASH_SOURCE[0]}")/common.sh"
lock_state
[[ -x $binary ]] || die 'Run ./agy-droidbridge install first.'
[[ -z ${AGY_MODELS_FILE:-} ]] || die 'Unset AGY_MODELS_FILE to update the saved catalog.'
read_cli

catalog=$(mktemp "$state/models.XXXXXX")
trap 'rm -f -- "$catalog"' EXIT
printf 'Reading models from Antigravity.\n'
if ! timeout 30 "$cli" models </dev/null >"$catalog"; then
  die 'Could not read the model list. Run agy models directly and sign in if prompted.'
fi
"$binary" --check-models "$catalog" || die 'The CLI did not return a usable model list. The saved list is unchanged.'
mv -f -- "$catalog" "$state/models.txt"
if running; then
  stop_proxy
  start_proxy
fi
printf 'Saved the model list for future starts.\n'
