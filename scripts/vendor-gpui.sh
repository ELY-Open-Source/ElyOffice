#!/usr/bin/env bash
# vendor's gpui is crates.io's release plus patches/gpui.
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
mode=${1:-check}
case $mode in
  check | write) ;;
  *) echo "usage: vendor-gpui.sh [check|write]" >&2; exit 2 ;;
esac
version=0.3.12
# Each crate with the hash crates.io's index gives it.
crates="gpui-pre 29db89613527079737f53752eab27fb65c9a7dd94979256c65a0e590c9fd34d8
gpui-pre-linux 54bab418d0b7c027af9495f278619b46887362b3bd6446e246c87c5b0fa167df
gpui-pre-macos 2de7d3f4ad51ac387e822495db363fcc1188d349793365731d054444c4cb06ee
gpui-pre-windows 7201762420222d5b5ce5bdf0056996199212a3349bfb2457a3a0238c688681f3"
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
mkdir "$work/vendor"
# Inside another repository git apply skips every path.
git init -q "$work"
while read -r name sum; do
  crate=$work/$name.crate
  curl -fsSL -o "$crate" "https://static.crates.io/crates/$name/$name-$version.crate"
  got=$(shasum -a 256 "$crate" | cut -d' ' -f1)
  if [ "$got" != "$sum" ]; then
    echo "vendor-gpui: $name $version hash $got, want $sum" >&2
    exit 1
  fi
  tar -xzf "$crate" -C "$work/vendor"
  mv "$work/vendor/$name-$version" "$work/vendor/$name"
done <<<"$crates"
for patch in "$root"/patches/gpui/*.patch; do
  (cd "$work" && git apply "$patch") || {
    echo "vendor-gpui: ${patch##*/} does not apply" >&2
    exit 1
  }
done
failed=0
while read -r name _; do
  if [ "$mode" = write ]; then
    rm -rf "$root/vendor/$name"
    cp -R "$work/vendor/$name" "$root/vendor/$name"
  elif ! diff -r "$work/vendor/$name" "$root/vendor/$name"; then
    echo "vendor-gpui: vendor/$name is not $version plus patches/gpui" >&2
    failed=1
  fi
done <<<"$crates"
[ "$failed" = 0 ] || exit 1
count=$(find "$root/patches/gpui" -name '*.patch' | wc -l | tr -d ' ')
echo "vendor-gpui: $mode: four crates at $version plus $count patches"
