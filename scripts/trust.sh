#!/usr/bin/env bash
set -euo pipefail

source "$(dirname -- "${BASH_SOURCE[0]}")/common.sh"
lock_state
read_cli
prepare_workspace
open_workspace
