#!/usr/bin/env bash
# apply-lo-patches.sh --check: pinned commit, whole series, nothing applied.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
fail() {
  echo "FAIL: $*" >&2
  exit 1
}
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
# A three-patch series over a one-file tree.
fake=$work/root
mkdir -p "$fake/scripts" "$fake/patches/libreoffice"
cp "$root/scripts/apply-lo-patches.sh" "$fake/scripts/"
tree() { # name; the same pinned commit each time
  mkdir -p "$work/$1"
  git -C "$work/$1" init -q
  git -C "$work/$1" config core.autocrlf false
  printf 'one\ntwo\nthree\n' >"$work/$1/a.txt"
  git -C "$work/$1" add a.txt
  GIT_AUTHOR_DATE="2000-01-01T00:00:00 +0000" GIT_COMMITTER_DATE="2000-01-01T00:00:00 +0000" git -C "$work/$1" -c user.name=t -c user.email=t@t -c commit.gpgsign=false commit -qm pin
}
tree pattern
pin=$(git -C "$work/pattern" rev-parse HEAD)
echo "fetch libreoffice-core https://example.invalid/core.git $pin \\" >"$fake/scripts/fetch-references.sh"
for i in 1 2 3; do
  word=$(sed -n "${i}p" "$work/pattern/a.txt")
  sed -i "${i}s/.*/$word-patched/" "$work/pattern/a.txt"
  git -C "$work/pattern" diff --src-prefix=a/ --dst-prefix=b/ >"$fake/patches/libreoffice/000$i-p$i.patch"
  git -C "$work/pattern" -c user.name=t -c user.email=t@t -c commit.gpgsign=false commit -qam "p$i"
done
check() { # tree; sets status and out
  status=0
  out=$("$fake/scripts/apply-lo-patches.sh" --check "$work/$1" 2>&1) || status=$?
}
upto() { # name, count; the pinned tree with the first count applied
  tree "$1"
  for ((i = 1; i <= $2; i++)); do git -C "$work/$1" apply "$fake/patches/libreoffice/000$i-p$i.patch"; done
}

upto whole 3
check whole
[ "$status" -eq 0 ] || fail "the whole series was refused: $out"
grep -q "holds all 3 patches" <<<"$out" || fail "the whole series is not named: $out"

upto lagging 1
before=$(cat "$work/lagging/a.txt")
check lagging
[ "$status" -ne 0 ] || fail "a tree lacking two patches passed"
grep -q "lacks 2: 0002-p2.patch 0003-p3.patch" <<<"$out" || fail "the missing patches are not named: $out"
[ "$(cat "$work/lagging/a.txt")" = "$before" ] || fail "--check applied patches"

upto moved 3
git -C "$work/moved" -c user.name=t -c user.email=t@t -c commit.gpgsign=false commit -qam moved
check moved
[ "$status" -ne 0 ] || fail "a tree off the pinned commit passed"
grep -q "not the pinned $pin" <<<"$out" || fail "the moved base is not named: $out"

upto edited 3
echo extra >>"$work/edited/a.txt"
check edited
[ "$status" -ne 0 ] || fail "an edited tree passed"
grep -q "matches no prefix" <<<"$out" || fail "the edited tree is not named: $out"

# Without --check it still applies the rest.
"$fake/scripts/apply-lo-patches.sh" "$work/lagging" >/dev/null
check lagging
[ "$status" -eq 0 ] || fail "applying did not complete the series: $out"
echo "lo-series: the whole series passes; a lagging, moved or edited tree is refused and named"
