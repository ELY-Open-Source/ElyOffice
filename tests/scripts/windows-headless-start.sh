#!/usr/bin/env bash
# ElyDoc without a visible desktop: one error, exit 1.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
case "$(uname -s)" in
MINGW* | MSYS* | CYGWIN*) ;;
*)
  echo "SKIP: not Windows"
  exit 0
  ;;
esac
fail() {
  echo "FAIL: $*" >&2
  exit 1
}
cd "$root"
cargo build -q -p ely-lok --bins
cargo build -q -p ely-doc
data=$(mktemp -d)
trap 'rm -rf "$data"' EXIT
start=$(date +%s)
set +e
ELY_SMOKE=1 ELY_DATA_ROOT=$(cygpath -w "$data") timeout 30 target/debug/ely-doc.exe >"$data/out" 2>&1
code=$?
set -e
took=$(($(date +%s) - start))
if [ "$code" = 0 ]; then
  echo "SKIP: a visible desktop, the window opened"
  exit 0
fi
log=$data/ely-doc/ely.log
[ "$code" = 1 ] || fail "exit $code after ${took}s, not 1; $(tail -3 "$data/out")"
grep -q "the window did not open" "$log" || fail "no start error in the log"
! grep -q "panicked" "$log" || fail "it panicked: $(grep -m1 panicked "$log")"
grep -q "no visible desktop, so no dialog" "$log" || fail "a dialog nobody can see was shown"
echo "PASS: without a visible desktop ElyDoc logs one error and exits 1 in ${took}s"
