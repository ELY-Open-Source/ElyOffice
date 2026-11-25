#!/usr/bin/env bash
# vendor's gpui is crates.io's release plus patches/gpui.
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
mode=${1:-check}
case $mode in
  check | write) ;;
  *) echo "usage: vendor-gpui.sh [check|write]" >&2; exit 2 ;;
esac
version=0.3.14
# Each crate with the hash crates.io's index gives it.
crates="gpui-pre ab22c23c1304a0b3f34f64568f59a4ce2abbfab3727273a90c66b37924d31089
gpui-pre-linux 9515d52d8378c5dda95b3b2536d3d05ea2431bc25d1d8bb4cdeb32f21090f4a1
gpui-pre-macos 23eeb64494000782e17b0ae0ec2c402ba7d76e5918a6af4759e60094fc8d6dcf
gpui-pre-windows 8badbaa4129a88e4d2fdaa1615fc61b2a59d4ebb2ac36e5de51f75598f0dd992"
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
mkdir "$work/vendor"
# Inside another repository git apply skips every path.
git init -q "$work"
# Patched files keep upstream's line endings.
git -C "$work" config core.autocrlf false
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
