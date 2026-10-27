#!/usr/bin/env bash
# Side panels: narrower by default, dragged wider, kept on restart.
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
for tool in ax-values ax-press drag; do swiftc -O -o "$work/$tool" "$root/tests/scripts/$tool.swift"; done
[ "$("$work/ax-values" $$ | head -n 1)" != untrusted ] || { echo "SKIP: needs Accessibility permission"; exit 0; }
home="$work/home"
mkdir -p "$home"
prefs="$home/Library/Application Support/ElyOffice/ely-doc/settings.json"
start() {
  ELY_SMOKE=hold ELY_LOCALE=en HOME="$home" perl -e 'alarm 120; exec @ARGV' "$root/target/debug/ely-doc" >"$work/out" 2>&1 &
  pid=$!
  for _ in $(seq 100); do "$work/ax-values" "$pid" | grep -q 'title=Back' && break; sleep 0.3; done
  osascript -e "tell application \"System Events\" to set frontmost of (first process whose unix id is $pid) to true" >/dev/null
  sleep 1
}
# The window's left edge and Back's center.
edges() {
  local window back
  window=$(osascript -e "tell application \"System Events\" to get position of first window of (first process whose unix id is $pid)" | cut -d, -f1)
  back=$("$work/ax-values" "$pid" | grep -m 1 'title=Back' | sed 's/.*at=//' | cut -d, -f1)
  echo "$window $back"
}
start
read -r left back <<<"$(edges)"
# Back is centered in the left panel: half its width.
[ $(( (back - left) * 2 )) -le 230 ] || fail "the left panel starts at $(( (back - left) * 2 )) points"
y=$(( $(osascript -e "tell application \"System Events\" to get position of first window of (first process whose unix id is $pid)" | cut -d, -f2 | tr -d ' ') + 400 ))
edge=$(( left + (back - left) * 2 - 1 ))
keys() { osascript -e 'tell application "System Events"' -e "$1" -e 'end tell' >/dev/null; }
quit() {
  keys 'keystroke "q" using command down'
  for _ in $(seq 40); do kill -0 "$pid" 2>/dev/null || break; sleep 0.25; done
  ! kill -0 "$pid" 2>/dev/null || fail "Cmd+Q did not quit"
  wait "$pid" 2>/dev/null || true
}
# Waits until the app's AX tree holds $1.
seen() {
  for _ in $(seq 40); do "$work/ax-values" "$pid" | grep -q -- "$1" && return 0; sleep 0.25; done
  return 1
}
# Dragged, then quit at once: the width is still kept.
"$work/drag" "$edge" "$y" $(( edge + 80 )) "$y"
read -r _ wide <<<"$(edges)"
[ $(( wide - back )) -ge 30 ] || fail "the panel did not widen: Back moved $(( wide - back ))"
quit
grep -q '"left": [0-9]' "$prefs" || fail "a quit right after the drag lost the width: $(cat "$prefs" 2>/dev/null)"
start
read -r left again <<<"$(edges)"
[ $(( again - wide )) -le 2 ] && [ $(( wide - again )) -le 2 ] || fail "the width did not come back: $again, not $wide"
quit

# A bad stored width is shown, not silently replaced.
python3 - "$prefs" <<'PY'
import json, sys
path = sys.argv[1]
fields = json.load(open(path))
fields["panels"] = {"left": "broken", "right": 320}
open(path, "w").write(json.dumps(fields))
PY
start
keys 'keystroke "," using command down'
seen "settings cannot be read" || fail "the bad width is not shown"
# A dragged width mends it; the message goes.
osascript -e "tell application \"System Events\" to click button 1 of (first window of (first process whose unix id is $pid) whose name is \"Settings\")" >/dev/null
sleep 0.5
read -r left back <<<"$(edges)"
edge=$(( left + (back - left) * 2 - 1 ))
"$work/drag" "$edge" "$y" $(( edge + 30 )) "$y"
for _ in $(seq 20); do grep -q '"left": [0-9]' "$prefs" 2>/dev/null && break; sleep 0.25; done
grep -q '"left": [0-9]' "$prefs" || fail "the mending width is not kept"
keys 'keystroke "," using command down'
seen "title=Dark" || fail "the settings page did not load"
! "$work/ax-values" "$pid" | grep -q "cannot be read" || fail "the mended width still shows as unreadable"
quit

# A failed save is shown.
printf '{"version":1}' >"$prefs"
start
read -r left back <<<"$(edges)"
edge=$(( left + (back - left) * 2 - 1 ))
rm "$prefs"
mkdir "$prefs"
"$work/drag" "$edge" "$y" $(( edge + 60 )) "$y"
sleep 1
keys 'keystroke "," using command down'
seen "was not saved" || fail "the failed save is not shown"
# Another kind of save succeeding leaves this failure shown.
"$work/ax-press" "$pid" Dark >/dev/null
sleep 1.5
grep -q '"appearance": "dark"' "$home/Library/Application Support/ElyOffice/settings.json" || fail "Dark was not saved"
seen "was not saved" || fail "the panel failure vanished on another save"
# Once the panel save works again, the failure goes.
rmdir "$prefs"
# Out of the way: the settings window covers the edge.
osascript -e "tell application \"System Events\" to click button 1 of (first window of (first process whose unix id is $pid) whose name is \"Settings\")" >/dev/null
sleep 0.5
read -r left back <<<"$(edges)"
edge=$(( left + (back - left) * 2 - 1 ))
"$work/drag" "$edge" "$y" $(( edge + 30 )) "$y"
for _ in $(seq 20); do grep -q '"left": [0-9]' "$prefs" 2>/dev/null && break; sleep 0.25; done
grep -q '"left": [0-9]' "$prefs" || fail "the retried width is not kept"
keys 'keystroke "," using command down'
# The page's own controls first: an empty tree proves nothing.
seen "title=Dark" || fail "the settings page did not load"
! "$work/ax-values" "$pid" | grep -q "was not saved" || fail "the fixed failure still shows"
quit
echo "ok: panels"
