#!/usr/bin/env bash
# An installed ElyMark updates from a served release, asking first.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
[ "$(uname)" = Darwin ] || { echo "SKIP: macOS only"; exit 0; }
work=$(mktemp -d)
cleanup() {
  pkill -f "$work/Applications/ElyMark.app" 2>/dev/null || true
  if [ -n "${server:-}" ]; then
    kill "$server" 2>/dev/null || true
    wait "$server" 2>/dev/null || true
  fi
  rm -rf "$work"
}
trap cleanup EXIT
fail() { echo "FAIL: $*" >&2; exit 1; }

ELY_DIST="$work/dist" "$root/packaging/macos/bundle.sh" ely-mark debug >"$work/bundle" 2>&1 || fail "bundle: $(tail -5 "$work/bundle")"
swiftc -O -o "$work/ax-press" "$root/tests/scripts/ax-press.swift"
mkdir -p "$work/Applications" "$work/new" "$work/serve"
ditto "$work/dist/ElyMark.app" "$work/Applications/ElyMark.app"
# The new release: the same app, marked.
ditto "$work/dist/ElyMark.app" "$work/new/ElyMark.app"
touch "$work/new/ElyMark.app/Contents/Resources/ely-update-marker"
codesign --force --deep --sign - "$work/new/ElyMark.app" 2>/dev/null
ln -s /Applications "$work/new/Applications"
hdiutil create -quiet -volname ElyMark -srcfolder "$work/new" -format UDZO "$work/serve/ElyMark.dmg"
port=$(python3 -c 'import socket; s = socket.socket(); s.bind(("127.0.0.1", 0)); print(s.getsockname()[1])')
sha=$(shasum -a 256 "$work/serve/ElyMark.dmg" | cut -d' ' -f1)
key="macos-$(uname -m | sed s/arm64/aarch64/)"
printf '{"version": "9.9.9", "assets": {"%s": {"url": "http://127.0.0.1:%s/ElyMark.dmg", "sha256": "%s"}}}\n' "$key" "$port" "$sha" >"$work/serve/ely-mark.json"
# The test key signs; the app reads its half.
signed() { ELY_UPDATE_KEY=JWPqrMuIaO6Rxz6efh5PJFonrCtZcc0yZnLG5pMUmKk= "$root/packaging/sign.sh" "$work/serve/ely-mark.json" >/dev/null; }
signed
python3 -m http.server "$port" --bind 127.0.0.1 --directory "$work/serve" >"$work/served" 2>&1 &
server=$!
mkdir -p "$work/home"
# The installer's open: says whether the failure note came first.
mkdir -p "$work/bin"
printf '#!/bin/sh\n[ -e "%s" ] && echo before >>"%s" || echo after >>"%s"\nexec /usr/bin/open "$@"\n' \
  "$work/home/Library/Application Support/ElyOffice/ely-mark/updates/failed.txt" "$work/opened" "$work/opened" >"$work/bin/open"
chmod +x "$work/bin/open"
start() {
  PATH="$work/bin:$PATH" HOME="$work/home" ELY_LOG=info ELY_LOCALE=en ELY_NO_DIALOG=1 ELY_UPDATE_MANIFEST="http://127.0.0.1:$port/ely-mark.json" \
    ELY_UPDATE_PUBLIC_KEY=LWfbar3eMGzvVABTvGC618oPPHvgHL/Cu30umm2w1Xg= "$work/Applications/ElyMark.app/Contents/MacOS/ely-mark" >"$work/out" 2>&1 &
  pid=$!
}
start
[ "$("$work/ax-press" "$pid" none)" != untrusted ] || fail "needs Accessibility permission"
# Off by default: nothing is asked at launch.
for _ in $(seq 60); do grep -q "no check at launch" "$work/out" && break; sleep 0.5; done
grep -q "no check at launch" "$work/out" || fail "the launch check ran: $(grep update "$work/out" | tail -3)"
for _ in $(seq 60); do grep -q 'mark: shown as' "$work/out" && break; sleep 0.5; done
# A new untitled page: it takes keys, autosave skips it.
osascript -e "tell application \"System Events\" to set frontmost of (first process whose unix id is $pid) to true" -e 'delay 1' \
  -e 'tell application "System Events" to keystroke "n" using command down' -e 'delay 2' \
  -e 'tell application "System Events" to keystroke "unsaved"' >/dev/null
sleep 3
! grep -q GET "$work/served" || fail "asked without being told: $(cat "$work/served")"
press() {
  for _ in $(seq 60); do
    [ "$("$work/ax-press" "$pid" "$1")" = pressed ] && return 0
    sleep 0.5
  done
  fail "could not press $1: $(grep update "$work/out" | tail -3)"
}
press "Check for Updates…"
for _ in $(seq 60); do grep -q "offers 9.9.9" "$work/out" && break; sleep 0.5; done
grep -q "offers 9.9.9" "$work/out" || fail "no offer: $(grep update "$work/out" | tail -3)"
press Download
press "Restart to update"
# Unsaved text: the question comes first; cancel installs nothing.
press Cancel
sleep 3
kill -0 "$pid" 2>/dev/null || fail "a cancelled quit still quit"
[ ! -e "$work/Applications/ElyMark.app/Contents/Resources/ely-update-marker" ] || fail "installed before the question"
# A plain quit after the cancel installs nothing either.
osascript -e 'tell application "System Events" to keystroke "q" using command down' >/dev/null
press "Don't Save"
for _ in $(seq 60); do kill -0 "$pid" 2>/dev/null || break; sleep 0.5; done
! kill -0 "$pid" 2>/dev/null || fail "the app did not quit"
sleep 8
[ ! -e "$work/Applications/ElyMark.app/Contents/Resources/ely-update-marker" ] || fail "a plain quit installed the update"
! pgrep -f "$(basename "$work")/Applications/ElyMark.app/Contents/MacOS/ely-mark" >/dev/null || fail "a plain quit reopened the app"
# Relaunched: check, download again, and Restart installs.
start
for _ in $(seq 60); do grep -q 'mark: shown as' "$work/out" && break; sleep 0.5; done
press "Check for Updates…"
press Download
press "Restart to update"
for _ in $(seq 60); do kill -0 "$pid" 2>/dev/null || break; sleep 0.5; done
! kill -0 "$pid" 2>/dev/null || fail "the old app did not quit"
# The installer copies once the old app has gone.
for _ in $(seq 120); do [ -e "$work/Applications/ElyMark.app/Contents/Resources/ely-update-marker" ] && break; sleep 0.5; done
[ -e "$work/Applications/ElyMark.app/Contents/Resources/ely-update-marker" ] || fail "the new app is not in place"
# The new app itself, by the path the system resolved.
binary="$(basename "$work")/Applications/ElyMark.app/Contents/MacOS/ely-mark"
for _ in $(seq 40); do pgrep -f "$binary" >/dev/null && break; sleep 0.5; done
fresh=$(pgrep -f "$binary" | head -n 1 || true)
[ -n "$fresh" ] && [ "$fresh" != "$pid" ] || fail "the new app did not open"
[ "$(ls -A "$work/Applications")" = ElyMark.app ] || fail "copies left: $(ls -A "$work/Applications")"
# A broken release: its note is written before the reopen.
pkill -f "$binary" || true
for _ in $(seq 20); do pgrep -f "$binary" >/dev/null || break; sleep 0.5; done
mkdir -p "$work/broken/Other.app"
hdiutil create -quiet -volname Broken -srcfolder "$work/broken" -format UDZO "$work/serve/Broken.dmg"
sha=$(shasum -a 256 "$work/serve/Broken.dmg" | cut -d' ' -f1)
rm -f "$work/opened"
printf '{"version": "9.9.10", "assets": {"%s": {"url": "http://127.0.0.1:%s/Broken.dmg", "sha256": "%s"}}}\n' "$key" "$port" "$sha" >"$work/serve/ely-mark.json"
signed
start
for _ in $(seq 60); do grep -q 'mark: shown as' "$work/out" && break; sleep 0.5; done
press "Check for Updates…"
press Download
press "Restart to update"
note="$work/home/Library/Application Support/ElyOffice/ely-mark/updates/failed.txt"
# Not the old process, which this script started and reaps.
reopened() { pgrep -f "$binary" | grep -vx "$pid" | head -n 1 || true; }
for _ in $(seq 60); do [ -n "$(reopened)" ] && [ -e "$note" ] && break; sleep 0.5; done
grep -q "holds no ElyMark.app" "$note" || fail "no failure note: $(cat "$note" 2>/dev/null)"
[ -n "$(reopened)" ] || fail "the old app did not reopen"
[ "$(head -n 1 "$work/opened")" = before ] || fail "the app reopened before the note was written"
echo "ok: ElyMark downloads, installs and reopens its update"
