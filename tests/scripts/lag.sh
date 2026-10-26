#!/usr/bin/env bash
# Typing lag in a real window: input and tile times.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
[ "$(uname)" = Darwin ] || { echo "SKIP: macOS only"; exit 0; }
app=${1:-ely-doc}
work=$(mktemp -d)
trap 'kill "${pid:-0}" 2>/dev/null || true; rm -rf "$work"' EXIT
fail() {
  echo "FAIL: $*" >&2
  exit 1
}

cargo build -q -p ely-lok --bins -p "$(cargo pkgid --manifest-path "$root/apps/$app/Cargo.toml")" --manifest-path "$root/Cargo.toml"
swiftc -O -o "$work/axv" "$root/tests/scripts/ax-values.swift"
[ "$("$work/axv" $$)" != untrusted ] || { echo "SKIP: needs Accessibility permission"; exit 0; }
ELY_SMOKE=hold ELY_LOG=debug ELY_LOCALE=en HOME="$work" XDG_DATA_HOME="$work/data" \
  perl -e 'alarm 120; exec @ARGV' "$root/target/debug/$app" >"$work/out" 2>"$work/err" &
pid=$!
log() { find "$work" -name ely.log -path "*$app*" 2>/dev/null | head -n 1; }
for _ in $(seq 200); do grep -q "opened Untitled" "$(log)" 2>/dev/null && break; sleep 0.2; done
grep -q "opened Untitled" "$(log)" || fail "no blank tab: $(tail -n 3 "$work/err")"
osascript -e "tell application \"System Events\" to set frontmost of (first process whose unix id is $pid) to true" >/dev/null
sleep 1
start=$(grep -c "lok: input" "$(log)" || true)
# A burst, then keys at a typist's pace.
osascript -e 'tell application "System Events" to keystroke "The quick brown fox jumps over the lazy dog. "' >/dev/null
osascript -e 'tell application "System Events"' \
  -e 'repeat with c in characters of "The quick brown fox jumps over the lazy dog."' \
  -e 'keystroke c' -e 'delay 0.08' -e 'end repeat' -e 'end tell' >/dev/null
sleep 2
# Percentiles of the timed lines after the start.
stats() {
  tail -n +"$2" "$(log)" | grep -E "$1" | sed -E 's/.* ([0-9]+) ms$/\1/' | sort -n |
    awk '{ v[NR] = $1 } END { if (NR == 0) { print "none"; exit } printf "%d samples, p50 %d, p95 %d, max %d ms\n", NR, v[int(NR / 2) + 1], v[int(NR * 0.95) + 1 > NR ? NR : int(NR * 0.95) + 1], v[NR] }'
}
line=$(grep -n "lok: input" "$(log)" | sed -n "$((start + 1))p" | cut -d: -f1)
[ -n "$line" ] || { [ -z "${ELY_LAG_LOG:-}" ] || cp "$(log)" "$ELY_LAG_LOG"; fail "no typed input reached the view"; }
echo "$app inputs: $(stats 'lok: input [0-9]+ waited' "$line")"
echo "$app tiles: $(stats 'lok: tile .* in [0-9]+ ms' "$line")"
echo "$app edit to tile: $(stats 'lok: edit to tile' "$line")"
[ -z "${ELY_LAG_LOG:-}" ] || { cp "$(log)" "$ELY_LAG_LOG"; cp "$work/err" "$ELY_LAG_LOG.err"; }
