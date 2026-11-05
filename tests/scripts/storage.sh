#!/usr/bin/env bash
# Storage: sizes shown; log cleared now, copies at restart.
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

cargo build -q -p ely-mark --manifest-path "$root/Cargo.toml"
for tool in ax-values ax-press; do swiftc -O -o "$work/$tool" "$root/tests/scripts/$tool.swift"; done
[ "$("$work/ax-values" $$ | head -n 1)" != untrusted ] || { echo "SKIP: needs Accessibility permission"; exit 0; }
home="$work/home"
data="$home/Library/Application Support/ElyOffice/ely-mark"
mkdir -p "$home"
keys() { osascript -e 'tell application "System Events"' -e "$1" -e 'end tell' >/dev/null; }
seen() {
  for _ in $(seq 60); do "$work/ax-values" "$pid" | grep -q -- "$1" && return 0; sleep 0.25; done
  return 1
}
start() {
  ELY_SMOKE=hold ELY_LOCALE=en HOME="$home" perl -e 'alarm 400; exec @ARGV' "$root/target/debug/ely-mark" >"$work/out" 2>&1 &
  pid=$!
  for _ in $(seq 40); do "$work/ax-values" "$pid" | grep -q AXWindow && break; sleep 0.25; done
  osascript -e "tell application \"System Events\" to set frontmost of (first process whose unix id is $pid) to true" >/dev/null
  sleep 1
}
# The KB the page shows for row $1.
kb() { "$work/ax-values" "$pid" | grep -m 1 "title=$1" | sed 's/.*help=//; s/ KB.*//'; }
# What the page should count under $1, KB rounded up.
disk_kb() { find "$1" -type f -exec stat -f %z {} + | awk '{s += $1} END {print int((s + 1023) / 1024)}'; }
quit() {
  keys 'keystroke "q" using command down'
  for _ in $(seq 40); do kill -0 "$pid" 2>/dev/null || break; sleep 0.25; done
  wait "$pid" 2>/dev/null || true
}
# A crash: the session's copies stay on disk.
crash() {
  kill -9 "$pid"
  wait "$pid" 2>/dev/null || true
}
mkdir -p "$data"
# A log long enough that clearing it shows.
head -c 8192 /dev/zero | tr '\0' 'x' >"$data/ely.log"
start
keys 'keystroke "," using command down'
seen "title=Storage" || fail "no settings window"
"$work/ax-press" "$pid" Storage >/dev/null
seen "title=Recovery copies" || fail "the storage page lists no copies"
shown=$(kb "This app's log")
[ "${shown:-0}" -ge 8 ] || fail "the log shows ${shown:-nothing} KB, not at least 8"
# Each clear asks first; nothing goes before the yes.
"$work/ax-press" "$pid" Clear AXButton >/dev/null
seen "AXButton.*title=Delete[[:space:]]" || fail "clearing the log did not ask"
[ "$(wc -c <"$data/ely.log")" -gt 4096 ] || fail "the log went before the yes"
"$work/ax-press" "$pid" Delete AXButton >/dev/null
sleep 1
[ "$(wc -c <"$data/ely.log")" -lt 4096 ] || fail "the log was not cleared"
# Ten points kept per document, once twelve were made.
"$work/ax-press" "$pid" 10 >/dev/null
for _ in $(seq 20); do grep -q '"keep": 10' "$data/settings.json" 2>/dev/null && break; sleep 0.25; done
grep -q '"keep": 10' "$data/settings.json" || fail "the recovery limit is not kept"
osascript -e "tell application \"System Events\" to click button 1 of (first window of (first process whose unix id is $pid) whose name is \"Settings\")" >/dev/null
sleep 1
# An untitled document: never autosaved, so it keeps recovery points.
tabs=$(grep -c "as tab" "$data/ely.log" || true)
keys 'keystroke "n" using command down'
for _ in $(seq 40); do [ "$(grep -c "as tab" "$data/ely.log")" -gt "$tabs" ] && break; sleep 0.25; done
# Source mode takes typing in an empty file.
keys 'keystroke "2" using command down'
sleep 2
# Twelve points written, one edit each.
written() { grep -c "recovery: a point at" "$data/ely.log" || true; }
for n in $(seq 12); do
  before=$(written)
  keys "keystroke \"edit $n \""
  for _ in $(seq 100); do [ "$(written)" -gt "$before" ] && break; sleep 0.25; done
  [ "$(written)" -gt "$before" ] || fail "edit $n wrote no point"
done
points=$(find "$data/recovery" -path '*/points/*.bin' | wc -l | tr -d ' ')
[ "$points" -eq 10 ] || fail "$points recovery points kept, not 10"
# Two picks while the file is locked: both are kept.
keys 'keystroke "," using command down'
seen "title=Storage" || fail "no settings window after the points"
"$work/ax-press" "$pid" Storage >/dev/null
seen "title=256" || fail "no space choices"
shown=$(kb "Recovery copies")
[ "$shown" = "$(disk_kb "$data/recovery")" ] || fail "recovery copies show ${shown:-nothing} KB, not $(disk_kb "$data/recovery")"
[ "$shown" -gt 0 ] || fail "recovery copies show nothing"
# Copies exist now, so their clear is offered.
"$work/ax-press" "$pid" "Clear at next start" AXButton >/dev/null
seen "AXButton.*title=Delete[[:space:]]" || fail "clearing the copies did not ask"
[ ! -e "$data/.clear-recovery" ] || fail "the copies were marked before the yes"
"$work/ax-press" "$pid" Delete AXButton >/dev/null
seen "Clears at next start" || fail "the copies clear is not shown as pending"
[ -e "$data/.clear-recovery" ] || fail "the copies clear was not marked"
python3 -c 'import fcntl, sys, time; f = open(sys.argv[1], "a"); fcntl.flock(f, fcntl.LOCK_EX); time.sleep(5)' "$data/.settings.lock" &
sleep 0.3
"$work/ax-press" "$pid" 20 >/dev/null
# Another page and back: the reread keeps the waiting pick.
"$work/ax-press" "$pid" General >/dev/null
sleep 0.5
"$work/ax-press" "$pid" Storage >/dev/null
seen "title=256" || fail "no space choices after coming back"
sleep 0.5
"$work/ax-press" "$pid" 256 >/dev/null
sleep 6
grep -q '"keep": 20' "$data/settings.json" && grep -q '"megabytes": 256' "$data/settings.json" || fail "one pick undid the other: $(grep -A3 recovery "$data/settings.json")"
# Settings closed and opened again between two locked picks.
python3 -c 'import fcntl, sys, time; f = open(sys.argv[1], "a"); fcntl.flock(f, fcntl.LOCK_EX); time.sleep(6)' "$data/.settings.lock" &
sleep 0.3
"$work/ax-press" "$pid" 50 >/dev/null
osascript -e "tell application \"System Events\" to click button 1 of (first window of (first process whose unix id is $pid) whose name is \"Settings\")" >/dev/null
sleep 0.5
keys 'keystroke "," using command down'
seen "title=Storage" || fail "no settings window the second time"
"$work/ax-press" "$pid" Storage >/dev/null
seen "title=256" || fail "no space choices the second time"
"$work/ax-press" "$pid" 32 >/dev/null
sleep 7
grep -q '"keep": 50' "$data/settings.json" && grep -q '"megabytes": 32' "$data/settings.json" || fail "reopening lost a pick: $(grep -A3 recovery "$data/settings.json")"
# A failed limit save stays told after a clear succeeds.
mv "$data/.settings.lock" "$work/lock"
mkdir "$data/.settings.lock"
"$work/ax-press" "$pid" 10 >/dev/null
seen "Not done" || fail "the failed limit save is not told"
rmdir "$data/.settings.lock"
mv "$work/lock" "$data/.settings.lock"
"$work/ax-press" "$pid" Clear AXButton >/dev/null
seen "AXButton.*title=Delete[[:space:]]" || fail "clearing the log did not ask the second time"
"$work/ax-press" "$pid" Delete AXButton >/dev/null
sleep 1
"$work/ax-values" "$pid" | grep -q "Not done" || fail "a clear hid the failed limit save"
"$work/ax-press" "$pid" 20 >/dev/null
for _ in $(seq 40); do grep -q '"keep": 20' "$data/settings.json" && break; sleep 0.25; done
grep -q '"keep": 20' "$data/settings.json" || fail "the retried limit is not kept"
sleep 0.5
"$work/ax-values" "$pid" | grep -q "Not done" && fail "the failure stays after a good save"
# After a crash only the mark clears the copies.
crash
[ -n "$(find "$data/recovery" -type f)" ] || fail "the crash left no copies"
touch "$data/recovery/ely-marker"
start
[ ! -e "$data/recovery/ely-marker" ] || fail "the copies were not cleared at the start"
[ ! -e "$data/.clear-recovery" ] || fail "the mark was not spent"
quit
# A failed clear stops the start; the mark stays.
mkdir -p "$data/recovery/locked"
touch "$data/recovery/locked/kept" "$data/.clear-recovery"
chmod 555 "$data/recovery/locked"
status=0
ELY_SMOKE=hold ELY_LOCALE=en ELY_NO_DIALOG=1 HOME="$home" perl -e 'alarm 60; exec @ARGV' "$root/target/debug/ely-mark" >"$work/out" 2>&1 || status=$?
chmod 755 "$data/recovery/locked"
[ "$status" = 1 ] || fail "a failed clear started anyway ($status)"
grep -q "could not be deleted" "$work/out" || fail "the failed clear is not told: $(tail -3 "$work/out")"
[ -e "$data/.clear-recovery" ] && [ -e "$data/recovery/locked/kept" ] || fail "the failed clear lost its mark or data"
start
[ ! -e "$data/.clear-recovery" ] || fail "the clear was not tried again"
[ ! -e "$data/recovery/locked/kept" ] || fail "the clear left the data"
quit
echo "ok: storage"
