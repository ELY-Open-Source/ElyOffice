#!/usr/bin/env bash
# vendor's gpui is crates.io's release plus patches/gpui.
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
mode=${1:-check}
case $mode in
  check | write) ;;
  *) echo "usage: vendor-gpui.sh [check|write]" >&2; exit 2 ;;
esac
version=0.3.11
# Each crate with the hash crates.io's index gives it.
crates="gpui-pre 9303c771abf2acff2a96d706f69d9a82088c4770173a07cce88f0002431d887d
gpui-pre-linux 79cac5cbd3485c1e3db8eafd5dc69019188f5cb0a39846946af772efa386f624
gpui-pre-macos d7410cfdaa1fa55a414346cc492d18ce340e3a8b6d0ac3213b984ed690c82b92
gpui-pre-windows 5141a3db59bab761a16ab19529fe4117a86e91922e8fc00af6dfc4829cb1744d"
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
