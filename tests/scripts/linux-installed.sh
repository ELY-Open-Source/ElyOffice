#!/usr/bin/env bash
# Installed apps or AppImages show themed windows. Usage: linux-installed.sh <installed|appimage>
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
mode=${1:?installed or appimage}
fail() {
  echo "FAIL: $*" >&2
  exit 1
}
[ "$(uname)" = Linux ] && [ -n "${WAYLAND_DISPLAY:-}" ] || {
  echo "SKIP: needs a Wayland session"
  exit 0
}
for tool in hyprctl grim magick tesseract jq git python3; do
  command -v "$tool" >/dev/null || {
    echo "SKIP: no $tool"
    exit 0
  }
done
version=$(awk -F'"' '/^version = / { print $2; exit }' "$root/Cargo.toml")
fixtures=$root/crates/ely-ui/tests/fixtures/omarchy
work=$(mktemp -d)
pid=
trap '[ -z "$pid" ] || stop_app "$pid"; rm -rf "$work"' EXIT
# shellcheck source=tests/scripts/linux-common.sh
. "$root/tests/scripts/linux-common.sh"
linux_inputs "$root" "$work"
current=$work/omarchy/current

program() { # slug
  case $mode in
    installed) echo "/usr/bin/$1" ;;
    appimage)
      local name
      name=$(awk -F'\t' -v slug="$1" '$1 == slug { print $2 }' "$root/packaging/apps.tsv")
      echo "$root/dist/$name-$version-linux-$(uname -m).AppImage"
      ;;
    *) fail "mode is installed or appimage, not $mode" ;;
  esac
}
# The one window's capture handle and address, once mapped.
window() {
  local id
  for _ in $(seq 120); do
    id=$(hyprctl clients -j | jq -r --argjson pid "$pid" \
      '[.[] | select(.pid == $pid and .mapped)] | if length == 1 then "\(.[0].stableId) \(.[0].address)" else "" end') || {
      echo "hyprctl or jq failed"
      return 1
    }
    [ -n "$id" ] && {
      echo "$id"
      return
    }
    kill -0 "$pid" 2>/dev/null || {
      echo "it quit"
      return 1
    }
    sleep 0.5
  done
  echo "no single window"
  return 1
}
# Tiling beside other windows crops the page.
float() { # address
  local w="address:$1" got
  for call in "float({ window = \"$w\", action = \"enable\" })" \
    "resize({ window = \"$w\", x = 1280, y = 800 })" "center({ window = \"$w\" })"; do
    hyprctl dispatch "hl.dsp.window.$call" >/dev/null || return 1
  done
  # A dispatch to a lost window still says ok.
  for _ in $(seq 20); do
    got=$(hyprctl clients -j | jq -r --arg a "$1" '.[] | select(.address == $a) | "\(.floating) \(.size)"') || return 1
    [ "$got" = "true [1280,800]" ] && return
    sleep 0.25
  done
  echo "$got"
  return 1
}
shoot() { # slug, file; the window itself, wherever it is
  local id address said
  said=$(window) || fail "$1 showed no window ($said): $(tail -n 5 "$work/$1.err")"
  read -r id address <<<"$said"
  said=$(float "$address") || fail "Hyprland did not float $1 at 1280x800 (${said:-dispatch failed})"
  sleep 4
  grim -T "$id" "$2" || fail "grim cannot capture $1"
}
check() { # slug, words, args
  local slug=$1 words=$2 exe shot=$work/$1.png
  shift 2
  exe=$(program "$slug")
  [ -x "$exe" ] || fail "no $exe"
  ELY_OMARCHY_DIR="$current" ELY_SMOKE=hold ELY_LOCALE=en-US HOME="$work" XDG_DATA_HOME="$work/data" \
    setsid "$exe" "$@" 2>"$work/$slug.err" &
  pid=$!
  shoot "$slug" "$shot"
  judge "$slug" "$shot" "$words" "$work/$slug.err"
  if [ "$slug" = ely-tree ]; then
    # Omarchy's swap: the open window follows.
    mkdir "$current/next-theme"
    cp "$fixtures/catppuccin-latte.toml" "$current/next-theme/colors.toml"
    rm -rf "$current/theme"
    mv "$current/next-theme" "$current/theme"
    echo catppuccin-latte >"$current/theme.name"
    sleep 2
    shoot "$slug" "$shot"
    red=$(band "$shot" r) || fail "magick cannot read $shot"
    [ "$red" -gt 180 ] || fail "$slug did not follow Omarchy to Latte"
    cp "$fixtures/catppuccin.toml" "$current/theme/colors.toml"
    echo catppuccin >"$current/theme.name"
  fi
  stop_app "$pid"
  pid=
  echo "linux: $mode $slug shows '$words' in Catppuccin"
}
for case in "${cases[@]}"; do
  IFS='|' read -r slug words input <<<"$case"
  if [ -n "$input" ]; then
    check "$slug" "$words" "$work/$input"
  else
    check "$slug" "$words"
  fi
done
