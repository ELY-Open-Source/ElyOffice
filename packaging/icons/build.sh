#!/usr/bin/env bash
# Renders assets/icons/<slug>.{icns,png,ico} from the SVG masters.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
[ "$(uname)" = Darwin ] || { echo "build.sh: macOS only (AppKit draws the icons)" >&2; exit 1; }
icons=$root/assets/icons
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
swiftc -O -o "$work/icon" "$root/packaging/icons/icon.swift"
# The small cut, where one exists, below 48 px.
master() { # slug, then -document if so, variant ("" or -macos), px
  [ $# -eq 4 ] || set -- "$1" "" "$2" "$3"
  local small=$icons/$1$2-small$3.svg
  if [ "$4" -lt 48 ] && [ -f "$small" ]; then echo "$small"; else echo "$icons/$1$2$3.svg"; fi
}
cut -f1 "$root/packaging/apps.tsv" | while read -r slug; do
  set=$work/$slug.iconset
  mkdir "$set"
  for px in 16 32 128 256 512; do
    "$work/icon" "$px" "$(master "$slug" -macos "$px")" "$set/icon_${px}x${px}.png"
    "$work/icon" $((px * 2)) "$(master "$slug" -macos $((px * 2)))" "$set/icon_${px}x${px}@2x.png"
  done
  iconutil -c icns -o "$icons/$slug.icns" "$set"
  if [ -f "$icons/$slug-document-macos.svg" ]; then
    set=$work/$slug-document.iconset
    mkdir "$set"
    for px in 16 32 128 256 512; do
      "$work/icon" "$px" "$(master "$slug" -document -macos "$px")" "$set/icon_${px}x${px}.png"
      "$work/icon" $((px * 2)) "$(master "$slug" -document -macos $((px * 2)))" "$set/icon_${px}x${px}@2x.png"
    done
    iconutil -c icns -o "$icons/$slug-document.icns" "$set"
    echo "$icons/$slug-document.icns"
  fi
  "$work/icon" 512 "$icons/$slug.svg" "$icons/$slug.png"
  sizes=()
  for px in 16 24 32 48 64 128 256; do
    "$work/icon" "$px" "$(master "$slug" "" "$px")" "$work/$slug-$px.png"
    sizes+=("$work/$slug-$px.png")
  done
  python3 "$root/packaging/icons/ico.py" "$icons/$slug.ico" "${sizes[@]}"
  echo "$icons/$slug.{icns,png,ico}"
done
