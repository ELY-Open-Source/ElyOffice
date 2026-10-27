#!/usr/bin/env bash
# ElyShow: on macOS the show covers menu bar and Dock.
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
cargo build -q -p ely-show --manifest-path "$root/Cargo.toml"
osascript -e 'tell application "System Events" to get name of first process' >/dev/null 2>&1 ||
  { echo "SKIP: needs Accessibility permission"; exit 0; }
cp "$root/tests/fixtures/office/t.pptx" "$work/deck.pptx"
mkdir -p "$work/home"
HOME="$work/home" ELY_LOG=info ELY_LOCALE=en ELY_NO_DIALOG=1 "$root/target/debug/ely-show" "$work/deck.pptx" >"$work/out" 2>&1 &
pid=$!
# The screen the show fills, in points.
screen=$(swift -e 'import AppKit; let f = NSScreen.screens[0].frame; print(Int(f.width), Int(f.height))')
windows() {
  osascript -e "tell application \"System Events\" to tell (first process whose unix id is $pid)" \
    -e 'set out to ""' \
    -e 'repeat with w in windows' \
    -e 'set {x, y} to position of w' \
    -e 'set {a, b} to size of w' \
    -e 'set out to out & x & " " & y & " " & a & " " & b & linefeed' \
    -e 'end repeat' -e 'return out' -e 'end tell' 2>/dev/null || true
}
for _ in $(seq 60); do
  grep -q "lok: tile" "$work/out" 2>/dev/null && [ -n "$(windows)" ] && break
  sleep 1
done
osascript -e "tell application \"System Events\" to set frontmost of (first process whose unix id is $pid) to true" -e 'delay 1' \
  -e 'tell application "System Events" to key code 96' >/dev/null
for _ in $(seq 20); do
  windows | grep -qx "0 0 $screen" && break
  sleep 1
done
windows | grep -qx "0 0 $screen" || fail "no show window covers the screen ($screen): $(windows | tr '\n' ';')"
# Menu bar and Dock hide; neither draws over the slide.
bars=$(swift -e 'import CoreGraphics; let l = CGWindowListCopyWindowInfo(.optionOnScreenOnly, kCGNullWindowID) as! [[String: Any]]; for w in l where ["Menubar", "Dock"].contains(w["kCGWindowName"] as? String ?? "") || (w["kCGWindowOwnerName"] as? String) == "Dock" && (w["kCGWindowLayer"] as? Int ?? 0) > 0 { print(w["kCGWindowName"] as? String ?? "Dock") }')
[ -z "$bars" ] || fail "still on screen over the show: $bars"
osascript -e 'tell application "System Events" to key code 53' >/dev/null
echo "ok: the show covers the whole screen"
