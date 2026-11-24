#!/usr/bin/env bash
set -euo pipefail
umask 077

fail() {
  printf 'DroidBridge setup: %s\n' "$*" >&2
  exit 1
}

fetch() {
  local url=$1 checksum=$2 file=$3
  curl --proto '=https' --proto-redir '=https' -fLsS --retry 2 -o "$file" "$url" ||
    fail "Could not download ${url##*/}. Check your connection and retry."
  printf '%s  %s\n' "$checksum" "$file" | sha256sum --check --status ||
    fail "Checksum check failed for ${url##*/}."
}

install_droidbridge() {
  [[ $(uname -m) == aarch64 && ${PREFIX:-} == */files/usr ]] ||
    fail 'Open Termux on your ARM64 Android phone and paste the command there.'
  [[ $(getprop ro.build.version.sdk) =~ ^[0-9]+$ ]] ||
    fail 'Could not detect the Android version.'
  (( $(getprop ro.build.version.sdk) >= 24 )) ||
    fail 'Android 7 or newer is required.'

  local -a packages=()
  command -v curl >/dev/null || packages+=(curl)
  command -v tar >/dev/null || packages+=(tar)
  command -v flock >/dev/null || packages+=(util-linux)
  if (( ${#packages[@]} )); then
    pkg install -y "${packages[@]}"
  fi

  local release=v0.1.1 cli_version=1.5.16
  local release_url=https://github.com/paritoshpky5/agy-droidbridge/releases/download
  local upstream_url=https://github.com/google-antigravity/antigravity-cli/releases/download
  local archive=agy_cli_linux_arm64_musl.tar.gz
  local state=${AGY_STATE_DIR:-"$HOME/.local/share/agy-droidbridge"}
  mkdir -p -- "$state"
  chmod 700 -- "$state"
  exec 8>"$state/bootstrap.lock"
  flock -x 8

  bootstrap_tmp=$(mktemp -d "$state/bootstrap.XXXXXXXX")
  trap 'rm -rf -- "$bootstrap_tmp"' EXIT
  local work=$bootstrap_tmp
  local app="$state/launcher-$release"
  local launcher="$app/agy-droidbridge"
  local asset=agy-droidbridge-aarch64-linux-android
  if [[ ! -x $launcher || ! -s $app/$asset ]]; then
    printf 'Downloading the DroidBridge release…\n'
    mkdir -p -- "$work/launcher"
    fetch "$release_url/$release/agy-droidbridge-launcher.tar.gz" \
      'dbad9a1f7dd318b51be6c2af853aa1bb9904ee68fb3b68c25e51794e03e3b8d3' "$work/launcher.tar.gz"
    tar -xzf "$work/launcher.tar.gz" -C "$work/launcher"
    fetch "$release_url/$release/$asset" \
      'ce61b052c1256e79658c176ee947626cc0a794631dea1aaa4540f196f2ff4471' "$work/launcher/$asset"
    if [[ -e $app || -L $app ]]; then
      mv -T -- "$app" "$work/old-launcher"
    fi
    mv -T -- "$work/launcher" "$app"
  fi

  source "$app/scripts/common.sh"
  local chosen=${AGY_BIN:-} cli_home="$state/linux-cli-$cli_version"
  if [[ -z $chosen && -s $state/cli ]]; then
    read -r chosen < "$state/cli"
  fi
  if [[ -z $chosen ]]; then
    chosen=$(command -v agy || true)
  fi

  if [[ -n $chosen ]] && check_cli "$chosen" 2>/dev/null; then
    printf 'Using the Antigravity CLI already available in Termux.\n'
  elif [[ -n ${AGY_BIN:-} ]]; then
    check_cli "$AGY_BIN" || fail 'Correct AGY_BIN or unset it to let setup install Antigravity.'
  else
    chosen="$cli_home/agy"
    if ! check_cli "$chosen" 2>/dev/null; then
      printf 'Preparing the official Antigravity %s Linux build for Termux.\n' "$cli_version"
      local -a cli_packages=()
      [[ -x $PREFIX/bin/proot ]] || cli_packages+=(proot)
      [[ -s $PREFIX/etc/resolv.conf ]] || cli_packages+=(resolv-conf)
      if (( ${#cli_packages[@]} )); then
        pkg install -y "${cli_packages[@]}"
      fi
      mkdir -p -- "$work/cli"
      fetch "$upstream_url/$cli_version/$archive" \
        '79e2477421b1146ff2e45c4540f19715f20bd61550368508b9072157842560e8' "$work/$archive"
      tar -xzf "$work/$archive" -C "$work/cli" antigravity
      install -m 700 -- "$app/scripts/termux-agy" "$work/cli/agy"
      check_cli "$work/cli/agy" || fail 'The official CLI could not start through Termux PRoot.'
      if [[ -e $cli_home || -L $cli_home ]]; then
        mv -T -- "$cli_home" "$work/old-cli"
      fi
      mv -T -- "$work/cli" "$cli_home"
    fi
    if [[ ! -e $PREFIX/bin/agy && ! -L $PREFIX/bin/agy ]]; then
      ln -s -- "$chosen" "$PREFIX/bin/agy"
    fi
  fi

  (
    exec 8>&-
    AGY_BIN="$chosen" bash "$launcher" install --interactive "$app/$asset"
  ) || fail 'DroidBridge could not start. Resolve the error above and paste the setup command again.'
  if [[ ! -e $PREFIX/bin/agy-droidbridge && ! -L $PREFIX/bin/agy-droidbridge ]]; then
    ln -s -- "$launcher" "$PREFIX/bin/agy-droidbridge"
  fi
  printf 'Use agy-droidbridge status to check the local API.\n'
}

# The initial command may stream this file over stdin. Give Antigravity a terminal for sign-in.
if [[ -t 0 ]]; then
  install_droidbridge
else
  install_droidbridge </dev/tty
fi
