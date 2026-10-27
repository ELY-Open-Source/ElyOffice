#!/usr/bin/env bash
# A second ElyDoc hands its file over and ends.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
[ "$(uname)" = Darwin ] || { echo "SKIP: macOS only"; exit 0; }
work=$(mktemp -d)
cleanup() {
  if [ -n "${pid:-}" ]; then
    # Its worker writes into the folder too.
    pkill -9 -P "$pid" 2>/dev/null || true
    kill "$pid" 2>/dev/null || true
    wait "$pid" 2>/dev/null || true
  fi
  rm -rf "$work"
}
trap cleanup EXIT
fail() { echo "FAIL: $*" >&2; exit 1; }

cargo build -q -p ely-lok --bin ely-lok-worker --manifest-path "$root/Cargo.toml"
cargo build -q -p ely-doc --manifest-path "$root/Cargo.toml"
run() { HOME="$work/home" ELY_LOG=info ELY_LOCALE=en ELY_NO_DIALOG=1 ELY_SMOKE=hold exec "$root/target/debug/ely-doc" "$@"; }
run >"$work/first" 2>&1 &
pid=$!
for _ in $(seq 90); do grep -q "lok: first tile drawn" "$work/first" && break; sleep 1; done
grep -q "lok: first tile drawn" "$work/first" || fail "the first copy did not start: $(tail -5 "$work/first")"
data="$work/home/Library/Application Support/ElyOffice/ely-doc"
[ -d "$data/work" ] || fail "no work folder"
touch "$data/work/kept"
cp "$root/tests/fixtures/office/t.docx" "$work/handed.docx"
(run "$work/handed.docx") >"$work/second" 2>&1 || fail "the second copy failed: $(tail -3 "$work/second")"
for _ in $(seq 30); do grep -q "opened handed.docx as tab" "$work/first" && break; sleep 1; done
grep -q "another copy handed over 1 files" "$work/first" || fail "no hand-over: $(tail -5 "$work/first")"
grep -q "opened handed.docx as tab" "$work/first" || fail "the first copy did not open it: $(tail -5 "$work/first")"
[ -e "$data/work/kept" ] || fail "the second copy wiped the first's work folder"
kill -0 "$pid" || fail "the first copy died"
echo "ok: a second ElyDoc hands its file to the first, which keeps its data"
