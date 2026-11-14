#!/usr/bin/env bash
# Installer upgrades remove only owned files; busy files change nothing.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
. "$root/tests/scripts/windows-common.sh"

work=$root/target/installer-test
key='HKCU\Software\Ely Office\ElyProbe'
clean() {
  reg delete "$key" /f >/dev/null 2>&1 || true
  rm -f "$APPDATA/Microsoft/Windows/Start Menu/Programs/ElyProbe.lnk"
}
trap 'kill $(jobs -p) 2>/dev/null || true; clean; restore' EXIT
clean
rm -rf "$work"
mkdir -p "$work"

# Two fake releases; v2 adds extra.txt and zz.
build() {
  local version=$1 stages=$work/stages-$1
  local stage=$stages/ely-probe
  mkdir -p "$stage/licenses" "$stage/data" "$stage/bin"
  cp /c/Windows/System32/PING.EXE "$stage/ely-probe.exe"
  cp /c/Windows/System32/PING.EXE "$stage/bin/helper.exe"
  echo "$version" >"$stage/ely-probe-worker.exe"
  echo "$version" >"$stage/licenses/LICENSE"
  echo "$version" >"$stage/data/info.txt"
  [ "$version" = 1 ] || { echo "$version" >"$stage/extra.txt"; mkdir -p "$stage/zz"; echo 2 >"$stage/zz/deep.txt"; }
  untyped ElyProbe "$stages/ely-probe-types.nsh"
  setup "$work/setup-$version.exe" "$version" "$stages" ElyProbe:ely-probe
}
build 1
build 2

# install VERSION ROOT [keyed]: the app goes in ROOT/ElyProbe.
install() {
  [ -n "${3:-}" ] || { reg delete "$key" /f >/dev/null 2>&1 || true; unsuite; }
  nsis "$work/setup-$1.exe" "/D=$(cygpath -w "$2")"
}
# Keyed to ROOT's app; a copy of the uninstaller runs.
uninstall() {
  reg add "$key" /v InstallDir /d "$(cygpath -w "$1/ElyProbe")" /f >/dev/null
  cp "$1/uninstall.exe" "$work/uninstall.exe"
  nsis "$work/uninstall.exe" "_?=$(cygpath -w "$1")"
}
snapshot() { (cd "$1" && find . -type f -exec md5sum {} + | sort); }
put() { mkdir -p "$(dirname "$1")" && echo user >"$1"; }
must() { [ -e "$1" ] || fail "$2: $1 is gone"; }
mustnt() { [ ! -e "$1" ] || fail "$2: $1 is still there"; }
# Holds a file open; sharing defaults to read only.
hold() {
  rm -f "$work/held"
  powershell -NoProfile -Command "\$f=[IO.File]::Open('$(cygpath -w "$1")','Open','Read','${2:-Read}'); \
    New-Item '$(cygpath -w "$work/held")' | Out-Null; Start-Sleep 60" &
  until [ -e "$work/held" ]; do sleep 0.2; done
}

# A chosen folder's own files survive install and reinstall.
top=$work/stranger dir=$work/stranger/ElyProbe
own="keep.exe keep.dll keep.json git/keep.txt notes/note.md licenses/my-notes.md"
for f in $own; do put "$dir/$f"; done
put "$top/mine.txt"
[ "$(install 1 "$top")" = 0 ] || fail "stranger install"
[ "$(install 2 "$top" keyed)" = 0 ] || fail "stranger reinstall"
for f in $own; do must "$dir/$f" stranger; done
must "$top/mine.txt" stranger
must "$dir/extra.txt" "stranger reinstall"
grep -q '^2$' "$dir/licenses/LICENSE" || fail "stranger reinstall kept the old licence"

# Older releases are cleaned only where our key points.
legacy() {
  for f in ely-probe.exe libcef.dll chrome_100.pak old.json git/cmd/git.exe locales/en.pak \
    notes/note.md keep.md; do put "$1/$f"; done
}
keyed() { reg add "$key" /v InstallDir /d "$(cygpath -w "$1")" /f >/dev/null; }
# Another install's file in the way: refused, nothing changes.
untouched() {
  local name=$1 top=$2 dir=$2/ElyProbe before
  before=$(snapshot "$dir")
  [ "$(install 2 "$top")" = 2 ] || fail "$name: installer did not refuse"
  [ "$(snapshot "$dir")" = "$before" ] || fail "$name: the folder changed"
  mustnt "$dir/.ely-old" "$name"
}
legacy "$work/unkeyed/ElyProbe"
untouched "unkeyed folder" "$work/unkeyed"
[ "$(install 1 "$work/taken")" = 0 ] || fail "taken setup"
put "$work/taken/ElyProbe/extra.txt"
untouched "user file in the way" "$work/taken"
top=$work/legacy dir=$work/legacy/ElyProbe
legacy "$dir"
keyed "$dir"
[ "$(install 2 "$top" keyed)" = 0 ] || fail "legacy upgrade"
for f in libcef.dll chrome_100.pak old.json git locales; do mustnt "$dir/$f" "legacy upgrade"; done
for f in notes/note.md keep.md extra.txt; do must "$dir/$f" "legacy upgrade"; done

# Busy files: the installer refuses; the old install stays whole.
busy() {
  local name=$1 top=$2 file=$3 dir=$2/ElyProbe before
  before=$(snapshot "$dir")
  hold "$file"
  [ "$(install 2 "$top" keyed)" = 2 ] || fail "$name: installer did not refuse"
  kill %% 2>/dev/null; wait 2>/dev/null || true
  [ "$(snapshot "$dir")" = "$before" ] || fail "$name: the old install changed"
  mustnt "$dir/.ely-old" "$name"
}
top=$work/busy dir=$work/busy/ElyProbe
[ "$(install 1 "$top")" = 0 ] || fail "busy setup"
busy "held program" "$top" "$dir/ely-probe-worker.exe"
busy "held file in a folder" "$top" "$dir/licenses/LICENSE"
top=$work/busy-legacy dir=$work/busy-legacy/ElyProbe
legacy "$dir"
keyed "$dir"
busy "held legacy git" "$top" "$dir/git/cmd/git.exe"

running() {
  local name=$1 top=$2 program=$3 dir=$2/ElyProbe before
  before=$(snapshot "$dir")
  "$program" -n 30 127.0.0.1 >/dev/null &
  sleep 1
  [ "$(install 2 "$top" keyed)" = 2 ] || fail "$name: installer did not refuse"
  kill %% 2>/dev/null; wait 2>/dev/null || true
  [ "$(snapshot "$dir")" = "$before" ] || fail "$name: the old install changed"
  mustnt "$dir/.ely-old" "$name"
}
top=$work/running dir=$work/running/ElyProbe
[ "$(install 1 "$top")" = 0 ] || fail "running setup"
running "running program" "$top" "$dir/ely-probe.exe"
running "running program in a folder" "$top" "$dir/bin/helper.exe"
top=$work/running-legacy dir=$work/running-legacy/ElyProbe
legacy "$dir"
cp /c/Windows/System32/PING.EXE "$dir/git/cmd/git.exe"
keyed "$dir"
running "running legacy git" "$top" "$dir/git/cmd/git.exe"

# A late copy failure puts the old release back.
top=$work/rollback dir=$work/rollback/ElyProbe
[ "$(install 1 "$top")" = 0 ] || fail "rollback setup"
put "$dir/zz/user.txt"
icacls "$(cygpath -w "$dir/zz")" /deny '*S-1-1-0:(WD)' >/dev/null
before=$(snapshot "$dir")
result=$(install 2 "$top" keyed)
icacls "$(cygpath -w "$dir/zz")" /remove:d '*S-1-1-0' >/dev/null
[ "$result" != 0 ] || fail "rollback: blocked copy succeeded"
[ "$(snapshot "$dir")" = "$before" ] || fail "rollback: the old install changed"
mustnt "$dir/.ely-old" rollback

# Paths near the limit upgrade and uninstall too.
long=$(cygpath -w "$work/long")
top=$work/long/$(printf "%$((224 - ${#long} - 1))s" | tr ' ' a) dir=$top/ElyProbe
[ "$(install 1 "$top")" = 0 ] || fail "long path install"
worker=$(cygpath -w "$dir/ely-probe-worker.exe")
[ ${#worker} = 254 ] || fail "long path is ${#worker} long"
[ "$(install 2 "$top" keyed)" = 0 ] || fail "long path upgrade"
must "$dir/extra.txt" "long path upgrade"
[ "$(uninstall "$top")" = 0 ] || fail "long path uninstall"
mustnt "$dir/ely-probe-worker.exe" "long path uninstall"

# A held install list stops uninstall; nothing changes.
unheld() {
  local name=$1 top=$2 share=$3 dir=$2/ElyProbe before result
  before=$(snapshot "$dir")
  hold "$dir/.ely-installed" "$share"
  result=$(uninstall "$top")
  kill %% 2>/dev/null; wait 2>/dev/null || true
  [ "$result" = 2 ] || fail "$name: uninstall did not refuse"
  [ "$(snapshot "$dir")" = "$before" ] || fail "$name: the install changed"
  mustnt "$dir/.ely-old" "$name"
}
[ "$(install 1 "$work/held-list")" = 0 ] || fail "held list setup"
unheld "list held for reading" "$work/held-list" Read
unheld "list held against deletion" "$work/held-list" ReadWrite

# Uninstall removes only what was installed, then the entry.
top=$work/stranger dir=$work/stranger/ElyProbe
[ "$(uninstall "$top")" = 0 ] || fail "uninstall"
for f in $own; do must "$dir/$f" uninstall; done
must "$top/mine.txt" uninstall
for f in ely-probe.exe ely-probe-worker.exe extra.txt licenses/LICENSE data bin zz .ely-installed ../uninstall.exe; do
  mustnt "$dir/$f" uninstall
done
! reg query "$key" >/dev/null 2>&1 || fail "uninstall left the app's key"
! reg query 'HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\Ely Office' >/dev/null 2>&1 ||
  fail "uninstall left the entry"
# An in-app update passes the app's own folder.
top="$work/Ely Office" dir="$work/Ely Office/ElyProbe"
[ "$(install 1 "$top")" = 0 ] || fail "update: setup 1"
# Unkeyed: only the folder maps /D to its parent.
reg delete "$key" /f >/dev/null
ELY_TEST_SETUP=$(cygpath -w "$work/setup-2.exe") ELY_TEST_DIR=$(cygpath -w "$dir") cargo test -q -p ely-platform --test install a_real_setup_installs_over_its_folder -- --exact >"$work/update.log" 2>&1 ||
  { cat "$work/update.log" >&2; fail "update: the after-quit step"; }
grep -q "1 passed" "$work/update.log" || fail "update: the test did not run"
must "$dir/extra.txt" "update"
must "$dir/zz/deep.txt" "update"
[ "$(cat "$dir/ely-probe-worker.exe")" = 2 ] || fail "update: version 1 stays"
mustnt "$dir/.ely-old" "update"
mustnt "$dir/ElyProbe" "update: nested in the app's folder"
rm -rf "$work"
echo "PASS: windows installer"
