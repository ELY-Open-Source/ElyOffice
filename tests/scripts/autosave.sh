#!/usr/bin/env bash
# ElyDoc autosave: off, then on from settings; outside changes wait.
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

cargo build -q -p ely-lok --bin ely-lok-worker --manifest-path "$root/Cargo.toml"
cargo build -q -p ely-doc --manifest-path "$root/Cargo.toml"
for tool in ax-values ax-press; do swiftc -O -o "$work/$tool" "$root/tests/scripts/$tool.swift"; done
[ "$("$work/ax-values" $$ | head -n 1)" != untrusted ] || { echo "SKIP: needs Accessibility permission"; exit 0; }
home="$work/home"
data="$home/Library/Application Support/ElyOffice/ely-doc"
mkdir -p "$home"
cp "$root/tests/fixtures/office/t.docx" "$work/a.docx"
cp "$work/a.docx" "$work/a.orig"
keys() { osascript -e 'tell application "System Events"' -e "$1" -e 'end tell' >/dev/null; }
seen() {
  for _ in $(seq 80); do "$work/ax-values" "$pid" | grep -q -- "$1" && return 0; sleep 0.25; done
  return 1
}
ELY_SMOKE=hold ELY_LOCALE=en ELY_NO_DIALOG=1 HOME="$home" perl -e 'alarm 240; exec @ARGV' \
  "$root/target/debug/ely-doc" "$work/a.docx" >"$work/out" 2>&1 &
pid=$!
for _ in $(seq 150); do grep -q "opened a.docx" "$data/ely.log" 2>/dev/null && break; sleep 0.2; done
grep -q "opened a.docx" "$data/ely.log" || fail "a.docx did not open"
osascript -e "tell application \"System Events\" to set frontmost of (first process whose unix id is $pid) to true" >/dev/null
sleep 1
# Off: edits stay unsaved.
keys 'keystroke "Zqx"'
sleep 6
cmp -s "$work/a.docx" "$work/a.orig" || fail "saved while autosave is off"
# On from the Documents settings.
keys 'keystroke "," using command down'
seen "title=Documents" || fail "no settings window"
"$work/ax-press" "$pid" Documents >/dev/null
seen "title=Save automatically" || fail "no autosave switch"
"$work/ax-press" "$pid" "Save automatically" AXCheckBox >/dev/null
for _ in $(seq 50); do grep -q '"autosave": true' "$data/settings.json" 2>/dev/null && break; sleep 0.2; done
grep -q '"autosave": true' "$data/settings.json" || fail "the switch is not kept"
osascript -e "tell application \"System Events\" to tell (first process whose unix id is $pid) to perform action \"AXRaise\" of window \"ElyDoc\"" >/dev/null
sleep 1
# The pending edit now saves once typing rests.
for _ in $(seq 40); do cmp -s "$work/a.docx" "$work/a.orig" || break; sleep 0.25; done
cmp -s "$work/a.docx" "$work/a.orig" && fail "no autosave with the switch on"
grep -q "autosaving tab" "$data/ely.log" || fail "the save was not the autosave"
# Changed elsewhere: the file waits for the user.
sleep 1
cp "$work/a.orig" "$work/a.docx"
keys 'keystroke "Y"'
sleep 6
cmp -s "$work/a.docx" "$work/a.orig" || fail "autosave wrote over an outside change"
echo "ok: autosave"
