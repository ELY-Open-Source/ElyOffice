#!/usr/bin/env bash
# ElyPDF: a locked file keeps sealed points past engine ends.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
[ "$(uname)" = Darwin ] || { echo "SKIP: macOS only"; exit 0; }
work=$(mktemp -d)
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

cargo build -q -p ely-pdf-worker -p ely-pdf --manifest-path "$root/Cargo.toml"
for tool in ax-values ax-press; do swiftc -O -o "$work/$tool" "$root/tests/scripts/$tool.swift"; done
[ "$("$work/ax-values" $$ | head -n 1)" != untrusted ] || { echo "SKIP: needs Accessibility permission"; exit 0; }
home="$work/home"
data="$home/Library/Application Support/ElyOffice/ely-pdf"
mkdir -p "$home"
cp "$root/tests/fixtures/pdf/aes128.pdf" "$work/locked.pdf"
keys() { osascript -e 'tell application "System Events"' -e "$1" -e 'end tell' >/dev/null; }
seen() {
  for _ in $(seq 80); do "$work/ax-values" "$pid" | grep -q -- "$1" && return 0; sleep 0.25; done
  return 1
}
opened() { { grep -c "ely-pdf: opened" "$data/ely.log" 2>/dev/null || true; } | head -n 1; }
start() {
  ELY_SMOKE=hold ELY_LOCALE=en ELY_NO_DIALOG=1 HOME="$home" perl -e 'alarm 300; exec @ARGV' \
    "$root/target/debug/ely-pdf" "$@" >"$work/out" 2>&1 &
  pid=$!
  for _ in $(seq 40); do "$work/ax-values" "$pid" | grep -q AXWindow && break; sleep 0.25; done
  osascript -e "tell application \"System Events\" to set frontmost of (first process whose unix id is $pid) to true" >/dev/null
  sleep 1
}
# The open-password dialog, answered; then the tab is there.
unlock() {
  local before
  before=$(opened)
  seen "is locked" || fail "no password asked ($1)"
  keys 'keystroke "owner"'
  keys 'key code 36'
  for _ in $(seq 80); do [ "$(opened)" -gt "${before:-0}" ] && break; sleep 0.25; done
  [ "$(opened)" -gt "${before:-0}" ] || fail "the locked file did not open ($1)"
}
# A new sealed point, and never a plain one.
sealed() {
  touch "$work/mark"
  for _ in $(seq 100); do
    find "$data/recovery" -name '*.enc' -newer "$work/mark" 2>/dev/null | grep -q . && break
    sleep 0.25
  done
  find "$data/recovery" -name '*.enc' -newer "$work/mark" | grep -q . || fail "no sealed point ($1)"
  [ -z "$(find "$data/recovery" -name '*.bin')" ] || fail "a plain point of a locked file ($1)"
}
start "$work/locked.pdf"
unlock "the file"
osascript -e "tell application \"System Events\" to tell (first process whose unix id is $pid) to click menu item \"Rotate Pages Right\" of menu \"Pages\" of menu bar 1" >/dev/null
sealed "while editing"
# The idle engine ends: told; the copy asks again.
pkill -9 -P "$pid" -f ely-pdf-worker || fail "no worker to stop"
seen "The document engine stopped" || fail "an idle engine's end is not told"
"$work/ax-press" "$pid" "Reopen documents" >/dev/null
unlock "the copy after the engine ended"
[ -n "$(find "$data/recovery" -name '*.enc')" ] || fail "the old points went before a new one"
grep -q "the engine is back, 0 tab(s) lost" "$data/ely.log" || fail "a tab is lost"
sealed "for the reopened copy"
echo "ok: pdf recovery"
