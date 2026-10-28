#!/usr/bin/env bash
# check.sh counts lines in code only, and reports every rule.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
fail() {
  echo "FAIL: $*" >&2
  exit 1
}
git init -q "$work"
git -C "$work" config core.autocrlf false
cp "$root/.gitattributes" "$work/"
lines() { mkdir -p "$(dirname "$work/$2")" && seq "$1" | sed 's/^/let a = 1; \/\/ /' >"$work/$2"; }
check() {
  git -C "$work" add -A
  said=$(bash "$root/scripts/check.sh" "$work" 2>&1)
}
lines 500 "src/full.rs"
lines 600 "vendor/big.rs"
lines 600 "docs/notes.md"
lines 600 "src/with space/data.txt"
lines 600 "site/video/vendor/big.js"
# The same lines, the last without its LF.
bare() { lines "$1" "$2" && printf '%s' "$(cat "$work/$2")" >"$work/$2"; }
bare 500 "src/bare.rs"
check || fail "files within the rules failed: $said"
bare 501 "src/bare.rs"
! check || fail "501 lines without a final LF passed"
case $said in *"src/bare.rs: 501 lines"*) ;; *) fail "the unterminated file was not named: $said" ;; esac
bare 500 "src/bare.rs"
lines 501 "src/with space/long.rs"
! check || fail "a 501-line .rs passed"
case $said in *"src/with space/long.rs: 501 lines"*) ;; *) fail "the long file was not named: $said" ;; esac
rm "$work/src/with space/long.rs"
mkdir -p "$work/crates/a"
printf '// One two three four five six seven eight nine ten eleven.\n' >"$work/crates/a/a.rs"
! check || fail "a long comment passed check.sh"
echo "ok: check rules"
