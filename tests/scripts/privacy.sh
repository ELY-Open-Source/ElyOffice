#!/usr/bin/env bash
# Logs and the diagnostics bundle keep document text out (§18.2).
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
[ "$(uname)" = Darwin ] || { echo "SKIP: macOS only"; exit 0; }
work=$(mktemp -d)
# Keeps the run's own status; the app's SIGTERM is expected.
cleanup() {
  local status=$?
  if [ -n "${pid:-}" ]; then
    kill "$pid" 2>/dev/null || true
    wait "$pid" 2>/dev/null || true
  fi
  rm -rf "$work"
  exit "$status"
}
trap cleanup EXIT
fail() {
  echo "FAIL: $*" >&2
  exit 1
}

cargo build -q -p ely-lok --bin ely-lok-worker --manifest-path "$root/Cargo.toml"
cargo build -q -p ely-doc --manifest-path "$root/Cargo.toml"
swiftc -O -o "$work/axv" "$root/tests/scripts/ax-values.swift"
swiftc -O -o "$work/press" "$root/tests/scripts/ax-press.swift"
[ "$("$work/axv" $$ | head -n 1)" != untrusted ] || { echo "SKIP: needs Accessibility permission"; exit 0; }
home="$work/home"
mkdir -p "$home/Documents"
ELY_SMOKE=hold ELY_LOCALE=en ELY_LOG=debug HOME="$home" \
  perl -e 'alarm 240; exec @ARGV' "$root/target/debug/ely-doc" >"$work/out" 2>"$work/err" &
pid=$!
log="$home/Library/Application Support/ElyOffice/ely-doc/ely.log"
for _ in $(seq 200); do grep -q "doc: opened" "$log" 2>/dev/null && break; sleep 0.2; done
grep -q "doc: opened Untitled 1" "$log" || fail "no blank tab: $(tail -n 3 "$work/err")"
keys() { osascript -e 'tell application "System Events"' -e "$1" -e 'end tell' >/dev/null; }
press() {
  for _ in $(seq 40); do
    [ "$("$work/press" "$pid" "$@")" = pressed ] && { sleep 0.8; return 0; }
    sleep 0.25
  done
  fail "could not press $*"
}
osascript -e "tell application \"System Events\" to set frontmost of (first process whose unix id is $pid) to true" >/dev/null
sleep 0.5
marker=ZqxPrivate42
keys "keystroke \"$marker typed\""
keys 'key code 36'
sleep 2
grep -q "ely_lok::worker\]" "$log" || fail "the worker's log does not reach the app log"
! grep -q "$marker" "$log" || fail "the log holds typed text: $(grep -o ".\{0,60\}$marker" "$log" | head -n 1)"

# The bundle: parts shown, home hidden, no text.
keys 'keystroke "," using command down'
press Diagnostics
for _ in $(seq 40); do "$work/axv" "$pid" | grep -q "This app's log" && break; sleep 0.25; done
"$work/axv" "$pid" | grep -q "This app's log" || fail "the parts are not shown before export"
press "Export…" AXButton
sleep 3
press Save AXButton
bundle="$home/Documents/ElyDoc-diagnostics.txt"
for _ in $(seq 40); do [ -s "$bundle" ] && break; sleep 0.25; done
[ -s "$bundle" ] || fail "no bundle written"
grep -q "ely_lok::worker\]" "$bundle" || fail "the bundle lacks the worker's lines"
! grep -q "$home" "$bundle" || fail "the bundle shows the home folder"
! grep -q "$marker" "$bundle" || fail "the bundle holds typed text"
echo "ok: privacy"
