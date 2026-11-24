#!/usr/bin/env bash
set -euo pipefail

root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)
source "$root/scripts/common.sh"

usage='Usage: ./agy-droidbridge install [--interactive] [arm64-binary]'
if [[ ${1:-} == --help || ${1:-} == -h ]]; then
  printf '%s\nBuild the proxy on ARM64 Termux or install an existing Android ARM64 binary.\n' "$usage"
  exit 0
fi
interactive=0
if [[ ${1:-} == --interactive ]]; then
  interactive=1
  shift
fi
(( $# <= 1 )) || die "$usage"
[[ $(uname -m) == aarch64 && ${PREFIX:-} == */files/usr ]] \
  && [[ $(getprop ro.build.version.sdk) =~ ^[0-9]+$ ]] \
  || die 'Run this command inside Termux on an ARM64 Android device. Check with uname -m.'

if (( $# )); then
  [[ -f $1 ]] || die 'Proxy binary file not found. Pass the path to an Android ARM64 build.'
fi
read_port
read_cli
check_cli "$cli" || exit 1

packages=()
command -v curl >/dev/null || packages+=(curl)
command -v flock >/dev/null || packages+=(util-linux)
if [[ $# -eq 0 ]]; then
  command -v cargo >/dev/null || packages+=(rust)
  command -v clang >/dev/null || packages+=(clang)
fi
if (( ${#packages[@]} )); then
  printf 'Installing Termux packages: %s\n' "${packages[*]}"
  pkg install -y "${packages[@]}"
fi

lock_state
read_port
read_cli

if [[ $# -eq 0 ]]; then
  printf 'Building the Rust proxy. The first build may take a few minutes.\n'
  (cd -- "$root" && cargo build --locked --release)
  artifact="$root/target/release/agy-droidbridge"
else
  artifact=$1
fi
[[ -f $artifact ]] || die 'Proxy binary not found.'
install -m 700 -- "$artifact" "$state/proxy.new"
if running; then
  stop_proxy
fi
mv -f -- "$state/proxy.new" "$binary"
printf '%s\n' "$port" > "$state/port"
printf '%s\n' "$cli" > "$state/cli"
if [[ ! -s "$state/api-key" ]]; then
  key=$(od -An -N32 -tx1 /dev/urandom | tr -d '[:space:]')
  [[ $key =~ ^[a-f0-9]{64}$ ]] || die 'Unable to generate the API key.'
  printf '%s\n' "$key" > "$state/api-key"
fi
chmod 600 -- "$state/api-key"
if (( interactive )); then
  prepare_workspace
  if ! agent_available; then
    open_workspace
  fi
fi
start_proxy
printf 'Find the API key in %s/api-key. Use it in the key or token field of your BYOK tool.\n' "$state"
