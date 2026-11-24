#!/usr/bin/env bash
set -euo pipefail

source "$(dirname -- "${BASH_SOURCE[0]}")/common.sh"
lock_state
read_cli
prepare_workspace
printf 'Approve this DroidBridge workspace in Antigravity if prompted, then exit Antigravity.\n'
cd -- "$workspace"
exec 9>&-
exec "$cli"
