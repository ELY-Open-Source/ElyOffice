#!/usr/bin/env bash
# ElyDoc recovery: points while editing, copies after crashes.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
[ "$(uname)" = Darwin ] || { echo "SKIP: macOS only"; exit 0; }
work=$(mktemp -d)
# Keeps the run's own status; the app's SIGTERM is expected.
cleanup() {
  local status=$?
  if [ -n "${pid:-}" ]; then
    pkill -9 -P "$pid" 2>/dev/null || true
    kill "$pid" 2>/dev/null || true
    wait "$pid" 2>/dev/null || true
  fi
  [ "$status" = 0 ] && rm -rf "$work" || echo "kept $work" >&2
  exit "$status"
}
trap cleanup EXIT
fail() {
  echo "FAIL: $*" >&2
  exit 1
}

cargo build -q -p ely-lok --bin ely-lok-worker --manifest-path "$root/Cargo.toml"
cargo build -q -p ely-doc --manifest-path "$root/Cargo.toml"
swiftc -O -o "$work/ax-values" "$root/tests/scripts/ax-values.swift"
swiftc -O -o "$work/ax-press" "$root/tests/scripts/ax-press.swift"
[ "$("$work/ax-values" $$ | head -n 1)" != untrusted ] || { echo "SKIP: needs Accessibility permission"; exit 0; }
home="$work/home"
data="$home/Library/Application Support/ElyOffice/ely-doc"
mkdir -p "$home"
keys() { osascript -e 'tell application "System Events"' -e "$1" -e 'end tell' >/dev/null; }
# Opened tabs so far; the log may not exist yet.
opened() { { grep -c "doc: opened" "$data/ely.log" 2>/dev/null || true; } | head -n 1; }
start() {
  local before
  before=$(opened)
  ELY_SMOKE=hold ELY_LOCALE=en HOME="$home" perl -e 'alarm 200; exec @ARGV' "$root/target/debug/ely-doc" >"$work/out" 2>&1 &
  pid=$!
  for _ in $(seq 150); do n=$(opened); [ "${n:-0}" -gt "${before:-0}" ] && break; sleep 0.2; done
  osascript -e "tell application \"System Events\" to set frontmost of (first process whose unix id is $pid) to true" >/dev/null
  sleep 1
}
gone() {
  for _ in $(seq 80); do "$work/ax-values" "$pid" | grep -q -- "$1" || return 0; sleep 0.25; done
  return 1
}
seen() {
  for _ in $(seq 80); do "$work/ax-values" "$pid" | grep -q -- "$1" && return 0; sleep 0.25; done
  return 1
}
start
keys 'keystroke "ZqxRecover typed"'
# A point within the ten-second beat, sealed or not.
for _ in $(seq 60); do find "$data/recovery" -path '*/points/*' -name '*.bin' 2>/dev/null | grep -q . && break; sleep 0.5; done
find "$data/recovery" -path '*/points/*' -name '*.bin' | grep -q . || fail "no recovery point while editing"
# A crash: no quit, no clean mark.
kill -9 "$pid"
wait "$pid" 2>/dev/null || true
# A refused index write is told, then caught up.
index="$data/recovery/index.json"
chmod 555 "$data/recovery"
written=$(stat -f %m "$index")
start
grep -q "recovery: the last run ended Crashed" "$data/ely.log" || fail "the crash was not noticed"
seen "AXTextArea	value=ZqxRecover typed" || fail "the recovered text is not back"
seen "Recovery points cannot be listed" || fail "a refused index write is not told"
chmod 755 "$data/recovery"
for _ in $(seq 60); do [ "$(stat -f %m "$index")" != "$written" ] && break; sleep 0.5; done
[ "$(stat -f %m "$index")" != "$written" ] || fail "the index never caught up"
# A second crash before the copy writes its own point.
pkill -9 -P "$pid" 2>/dev/null || true
kill -9 "$pid"
wait "$pid" 2>/dev/null || true
start
seen "AXTextArea	value=ZqxRecover typed" || fail "a second crash lost the text"
# The engine ends before a new point; the old counts.
pkill -9 -P "$pid" -f ely-lok-worker || fail "no engine worker to stop"
seen "The document engine stopped" || fail "the ended engine is not told"
"$work/ax-values" "$pid" | grep -q "none yet" && fail "the taken-over point is not the last recovery point"
"$work/ax-press" "$pid" "Reopen documents" >/dev/null
gone "The document engine stopped" || fail "the engine does not come back the first time"
seen "AXTextArea	value=ZqxRecover typed" || fail "the text is not back after the first engine restart"
# Unsaved: closing asks.
keys 'keystroke "w" using command down'
seen "Don't Save" || fail "the recovered copy closes without asking"
"$work/ax-press" "$pid" "Cancel" >/dev/null
# The engine alone ends: the window says so and reopens.
touch "$work/mark"
keys 'keystroke " engine"'
for _ in $(seq 60); do
  find "$data/recovery" -path '*/points/*' -name '*.bin' -newer "$work/mark" | grep -q . && break
  sleep 0.5
done
find "$data/recovery" -path '*/points/*' -name '*.bin' -newer "$work/mark" | grep -q . || fail "no point for the new text"
pkill -9 -P "$pid" -f ely-lok-worker || fail "no engine worker to stop"
seen "The document engine stopped" || fail "the ended engine is not told"
"$work/ax-press" "$pid" "Reopen documents" >/dev/null
gone "The document engine stopped" || fail "the engine does not come back"
grep -q "the engine is back, 0 tab(s) lost" "$data/ely.log" || fail "a tab is lost"
seen "AXTextArea	value=.*engine" || fail "the text is not back after the engine restarts"
keys 'keystroke "w" using command down'
seen "Don't Save" || fail "the reopened copy closes without asking"
echo "ok: recovery"
