#!/usr/bin/env bash
# Usage: apply-lo-patches.sh [--check] <lo-src> — applies the missing patches.
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
check=
[ "${1:-}" != --check ] || { check=1; shift; }
src=${1:?usage: apply-lo-patches.sh [--check] <lo-src>}
patches=("$root"/patches/libreoffice/*.patch)
files=$(sed -n 's|^+++ b/||p' "${patches[@]}" | sort -u)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

# Whether $src is HEAD plus the first $1 patches.
holds() {
  local n=$1 base=$work/base f i
  rm -rf "$base"
  mkdir -p "$base"
  git -C "$base" init -q
  # LF, as every tree here; Windows defaults convert.
  git -C "$base" config core.autocrlf false
  for f in $files; do
    if git -C "$src" cat-file -e "HEAD:$f" 2>/dev/null; then
      mkdir -p "$base/$(dirname "$f")"
      git -C "$src" show "HEAD:$f" >"$base/$f"
    fi
  done
  for ((i = 0; i < n; i++)); do
    git -C "$base" apply "${patches[$i]}" 2>/dev/null || return 1
  done
  for f in $files; do
    if [ -e "$base/$f" ]; then
      cmp -s "$base/$f" "$src/$f" || return 1
    elif [ -e "$src/$f" ]; then
      return 1
    fi
  done
}

# --check: the pinned commit plus the whole series, applying nothing.
if [ -n "$check" ]; then
  pin=$(sed -n 's/^fetch libreoffice-core [^ ]* \([0-9a-f]\{40\}\).*/\1/p' "$root/scripts/fetch-references.sh")
  [ -n "$pin" ] || { echo "apply-lo-patches.sh: no libreoffice-core pin in fetch-references.sh" >&2; exit 1; }
  head=$(git -C "$src" rev-parse HEAD)
  [ "$head" = "$pin" ] || { echo "apply-lo-patches.sh: $src is at $head, not the pinned $pin" >&2; exit 1; }
  if holds "${#patches[@]}"; then
    echo "apply-lo-patches.sh: $src holds all ${#patches[@]} patches"
    exit 0
  fi
fi
n=${#patches[@]}
while [ "$n" -ge 0 ] && ! holds "$n"; do
  n=$((n - 1))
done
if [ "$n" -lt 0 ]; then
  echo "apply-lo-patches.sh: $src matches no prefix of the patch series" >&2
  exit 1
fi
echo "apply-lo-patches.sh: $n of ${#patches[@]} patches already in $src"
if [ -n "$check" ]; then
  missing=()
  for ((i = n; i < ${#patches[@]}; i++)); do missing+=("${patches[$i]##*/}"); done
  echo "apply-lo-patches.sh: $src lacks ${#missing[@]}: ${missing[*]}" >&2
  exit 1
fi
for ((i = n; i < ${#patches[@]}; i++)); do
  git -C "$src" apply "${patches[$i]}"
  echo "apply-lo-patches.sh: applied ${patches[$i]##*/}"
done
