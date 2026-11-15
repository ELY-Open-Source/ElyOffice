#!/usr/bin/env bash
# An installed ElyMark fetches a served .pkg, asking first.
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
mkdir -p "$work/Applications" "$work/serve"
ditto "$work/dist/ElyMark.app" "$work/Applications/ElyMark.app"
# The new release: a package the Installer opens.
mkdir -p "$work/payload"
pkgbuild --quiet --root "$work/payload" --identifier ely.test.update --version 9.9.9 "$work/serve/ElyOffice.pkg"
port=$(python3 -c 'import socket; s = socket.socket(); s.bind(("127.0.0.1", 0)); print(s.getsockname()[1])')
sha=$(shasum -a 256 "$work/serve/ElyOffice.pkg" | cut -d' ' -f1)
key="macos-$(uname -m | sed s/arm64/aarch64/)"
printf '{"version": "9.9.9", "assets": {"%s": {"url": "http://127.0.0.1:%s/ElyOffice.pkg", "sha256": "%s"}}}\n' "$key" "$port" "$sha" >"$work/serve/ely-mark.json"
# The test key signs; the app reads its half.
signed() { ELY_UPDATE_KEY=JWPqrMuIaO6Rxz6efh5PJFonrCtZcc0yZnLG5pMUmKk= "$root/packaging/sign.sh" "$work/serve/ely-mark.json" >/dev/null; }
signed
python3 -m http.server "$port" --bind 127.0.0.1 --directory "$work/serve" >"$work/served" 2>&1 &
server=$!
mkdir -p "$work/home"
# The Installer's open, recorded; $work/refuse makes it fail.
mkdir -p "$work/bin"
printf '#!/bin/sh\nprintf "%%s\\n" "$*" >>"%s"\n[ ! -e "%s" ] || { echo "no Installer here" >&2; exit 1; }\n' \
  "$work/opened" "$work/refuse" >"$work/bin/open"
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
# The title bar's button opens the dialog.
dialog() {
  local opened
  opened=$(grep -c "the dialog opens" "$work/out" || true)
  press "$1"
  for _ in $(seq 40); do [ "$(grep -c "the dialog opens" "$work/out" || true)" -gt "$opened" ] && break; sleep 0.25; done
  [ "$(grep -c "the dialog opens" "$work/out" || true)" -gt "$opened" ] || fail "$1 opened no dialog"
  # Until focused, the title bar's button answers.
  local forward
  for _ in $(seq 20); do
    forward=$(grep -c "comes forward" "$work/out" || true)
    press "$2"
    sleep 0.25
    [ "$(grep -c "comes forward" "$work/out" || true)" -gt "$forward" ] || return 0
  done
  fail "the dialog never took $2"
}
offer() {
  dialog "Update Available" Download
  dialog "Restart to update" "Restart to update"
}
press "Check for Updates…"
for _ in $(seq 60); do grep -q "offers 9.9.9" "$work/out" && break; sleep 0.5; done
grep -q "offers 9.9.9" "$work/out" || fail "no offer: $(grep update "$work/out" | tail -3)"
offer
# Unsaved text: the question comes first; cancel installs nothing.
press Cancel
sleep 3
kill -0 "$pid" 2>/dev/null || fail "a cancelled quit still quit"
[ ! -e "$work/opened" ] || fail "the Installer opened before the question"
# A plain quit after the cancel installs nothing either.
osascript -e 'tell application "System Events" to keystroke "q" using command down' >/dev/null
press "Don't Save"
for _ in $(seq 60); do kill -0 "$pid" 2>/dev/null || break; sleep 0.5; done
! kill -0 "$pid" 2>/dev/null || fail "the app did not quit"
sleep 8
[ ! -e "$work/opened" ] || fail "a plain quit opened the Installer"
! pgrep -f "$(basename "$work")/Applications/ElyMark.app/Contents/MacOS/ely-mark" >/dev/null || fail "a plain quit reopened the app"
# Relaunched: check, download again, and Restart opens the Installer.
start
for _ in $(seq 60); do grep -q 'mark: shown as' "$work/out" && break; sleep 0.5; done
press "Check for Updates…"
offer
for _ in $(seq 60); do kill -0 "$pid" 2>/dev/null || break; sleep 0.5; done
! kill -0 "$pid" 2>/dev/null || fail "the old app did not quit"
for _ in $(seq 40); do [ -s "$work/opened" ] && break; sleep 0.5; done
opened=$(cat "$work/opened" 2>/dev/null || true)
case $opened in "-b com.apple.installer "*.pkg) ;; *) fail "the Installer was not asked: $opened" ;; esac
pkg=${opened#-b com.apple.installer }
[ "$(shasum -a 256 "$pkg" | cut -d' ' -f1)" = "$sha" ] || fail "the Installer got another file: $pkg"
binary="$(basename "$work")/Applications/ElyMark.app/Contents/MacOS/ely-mark"
! pgrep -f "$binary" >/dev/null || fail "the app reopened; the Installer's end is the user's"
# A refused open leaves a note the next launch reads.
touch "$work/refuse"
rm -f "$work/opened"
start
for _ in $(seq 60); do grep -q 'mark: shown as' "$work/out" && break; sleep 0.5; done
press "Check for Updates…"
offer
note="$work/home/Library/Application Support/ElyOffice/ely-mark/updates/failed.txt"
for _ in $(seq 60); do [ -e "$note" ] && break; sleep 0.5; done
grep -q "no Installer here" "$note" || fail "no failure note: $(cat "$note" 2>/dev/null)"
start
for _ in $(seq 60); do [ ! -e "$note" ] && break; sleep 0.5; done
[ ! -e "$note" ] || fail "the next launch did not take the note"
echo "ok: ElyMark downloads its update and hands the .pkg to the Installer"
