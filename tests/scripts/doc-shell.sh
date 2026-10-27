#!/usr/bin/env bash
# ElyDoc opens a blank tab; Cmd+N works at once.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
[ "$(uname)" = Darwin ] || { echo "SKIP: macOS only"; exit 0; }
work=$(mktemp -d)
trap 'kill "${pid:-0}" 2>/dev/null || true; rm -rf "$work"' EXIT
fail() {
  echo "FAIL: $*" >&2
  exit 1
}

cargo build -q -p ely-lok --bin ely-lok-worker --manifest-path "$root/Cargo.toml"
cargo build -q -p ely-doc --manifest-path "$root/Cargo.toml"
swiftc -O -o "$work/axv" "$root/tests/scripts/ax-values.swift"
[ "$("$work/axv" $$)" != untrusted ] || { echo "SKIP: needs Accessibility permission"; exit 0; }
ELY_SMOKE=hold ELY_LOCALE=en HOME="$work" XDG_DATA_HOME="$work/data" \
  perl -e 'alarm 120; exec @ARGV' "$root/target/debug/ely-doc" >"$work/out" 2>"$work/err" &
pid=$!
log() { find "$work" -path '*/ElyOffice/ely-doc/ely.log' 2>/dev/null | head -n 1; }
for _ in $(seq 200); do grep -q "doc: opened" "$(log)" 2>/dev/null && break; sleep 0.2; done
grep -q "doc: opened Untitled 1" "$(log)" || fail "no blank tab: $(tail -n 3 "$work/err")"
tree() { "$work/axv" "$pid"; }
seen=
for _ in 1 2 3 4 5; do
  seen=$(tree)
  grep -q "Untitled 1" <<<"$seen" && break
  sleep 1
done
grep -q "Untitled 1" <<<"$seen" || fail "the tab is not in the AX tree"
osascript -e "tell application \"System Events\" to set frontmost of (first process whose unix id is $pid) to true" \
  -e 'delay 0.5' -e 'tell application "System Events" to keystroke "n" using command down' >/dev/null
for _ in 1 2 3 4 5; do
  seen=$(tree)
  grep -q "New Document" <<<"$seen" && break
  sleep 1
done
grep -q "New Document" <<<"$seen" || fail "Cmd+N at startup opened no New dialog"
echo "ok: doc-shell"
