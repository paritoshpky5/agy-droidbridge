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

  local release=v0.1.0 cli_version=1.2.7
  local release_url=https://github.com/paritoshpky5/agy-droidbridge/releases/download
  local upstream_url=https://github.com/google-antigravity/antigravity-cli/releases/download
  local archive=agy_cli_linux_arm64.tar.gz
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
      '588044f0e139b38738f53e7fff660991cd0b7487e19973e5cb84fd6a78c0ba5b' "$work/launcher.tar.gz"
    tar -xzf "$work/launcher.tar.gz" -C "$work/launcher"
    fetch "$release_url/$release/$asset" \
      'ce61b052c1256e79658c176ee947626cc0a794631dea1aaa4540f196f2ff4471' "$work/launcher/$asset"
    if [[ -e $app || -L $app ]]; then
      mv -T -- "$app" "$work/old-launcher"
    fi
    mv -T -- "$work/launcher" "$app"
  fi

  source "$app/scripts/common.sh"
  local chosen=${AGY_BIN:-} cli_home="$state/termux-cli-$cli_version"
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
      printf 'Installing Antigravity %s for Termux. This downloads the official CLI and builds its compatibility shim.\n' "$cli_version"
      pkg install -y clang python patchelf
      mkdir -p -- "$cli_home"
      fetch 'https://codeload.github.com/Aarstad/agy-termux-musl/tar.gz/0429897e9bde85773f65177f1db9444e20040bf1' \
        '35d1abad391cb163d44b06550015d19275b44c5595695157ca295c2c706f5ef7' "$work/termux.tar.gz"
      tar -xzf "$work/termux.tar.gz" --strip-components=1 -C "$cli_home"
      cat "$app/scripts/termux-musl-stdio.c" >> "$cli_home/shim.c"
      fetch "$upstream_url/$cli_version/$archive" \
        '8ddbb669158de1d1bc4c1fe5c130dca8f51da80d62569a54a4133f06768a723b' "$work/$archive"
      tar -xzf "$work/$archive" -C "$work" antigravity
      bash "$cli_home/install.sh" --from-binary "$work/antigravity" ||
        fail 'The Termux compatibility build failed. Share the installer error so it can be diagnosed.'
      check_cli "$chosen" || fail 'Antigravity was installed but could not run with the required flags.'
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
