#!/usr/bin/env bash
# One setup: chosen folder, installed apps only, refusal stops.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
. "$root/tests/scripts/windows-common.sh"

work=$root/target/setup-test
names="ElyProbe ElyProbeTwo ElyProbeThree"
entry='HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall'
# deny Add|Remove RIGHT KEY: this user loses RIGHT there.
deny() {
  ELY_KEY=${3#HKCU\\} ELY_DO=$1 ELY_RIGHT=$2 powershell -NoProfile -Command \
    '$ErrorActionPreference="Stop"; $k=[Microsoft.Win32.Registry]::CurrentUser.OpenSubKey($env:ELY_KEY,"ReadWriteSubTree","ReadPermissions,ChangePermissions")
    $a=$k.GetAccessControl(); $u=[Security.Principal.WindowsIdentity]::GetCurrent().Name
    $r=New-Object Security.AccessControl.RegistryAccessRule($u,$env:ELY_RIGHT,"Deny")
    if ($env:ELY_DO -eq "Add") { $a.AddAccessRule($r) } else { [void]$a.RemoveAccessRule($r) }
    $k.SetAccessControl($a)'
}
clean() {
  deny Remove Delete "$entry\\Ely Office ElyProbe" >/dev/null 2>&1 || true
  deny Remove QueryValues "HKCU\\Software\\Ely Office\\ElyProbe" >/dev/null 2>&1 || true
  for name in $names; do
    reg delete "HKCU\\Software\\Ely Office\\$name" /f >/dev/null 2>&1 || true
    reg delete "$entry\\Ely Office $name" /f >/dev/null 2>&1 || true
    rm -f "$APPDATA/Microsoft/Windows/Start Menu/Programs/$name.lnk"
  done
}
trap 'kill $(jobs -p) 2>/dev/null || true; clean; restore' EXIT
clean
rm -rf "$work"
mkdir -p "$work"

stages=$work/stages apps=()
for name in $names; do
  slug=$(echo "$name" | sed 's/\([A-Z]\)/-\L\1/g; s/^-//')
  mkdir -p "$stages/$slug"
  echo "$name" >"$stages/$slug/$slug.exe"
  untyped "$name" "$stages/$slug-types.nsh"
  "$root/packaging/windows/files.sh" "$stages/$slug" "$slug" >"$stages/$slug-files.txt"
  apps+=("$name:$slug")
done
setup "$work/setup.exe" 1 "$stages" "${apps[@]}"

chosen="$work/O'Neil chosen dir"
run() { nsis "$work/setup.exe" "/D=$(cygpath -w "$chosen")"; }
installed() { [ -f "$1/.ely-installed" ]; }

# Another program's uninstaller stops the setup, even where 0.1.16 was.
mkdir -p "$chosen"
reg add "HKCU\Software\Ely Office" /v Root /d "$(cygpath -w "$chosen")" /f >/dev/null
echo other >"$chosen/uninstall.exe"
[ "$(run)" = 2 ] || fail "the setup took a folder with another uninstaller"
grep -q other "$chosen/uninstall.exe" || fail "another program's uninstaller was overwritten"
[ ! -e "$chosen/ElyProbe" ] || fail "the setup went on after the refusal"
rm "$chosen/uninstall.exe"

# First install: every app, each in its own folder there.
reg add "$entry\\Ely Office ElyProbe" /v DisplayName /d "Ely Office ElyProbe" /f >/dev/null
[ "$(run)" = 0 ] || fail "the first install failed"
for name in $names; do installed "$chosen/$name" || fail "$name is not in the chosen folder"; done
[ -f "$chosen/uninstall.exe" ] || fail "no uninstaller in the chosen folder"
reg query "$entry\\Ely Office" /v UninstallString | grep -qF "$(cygpath -w "$chosen")\\uninstall.exe" ||
  fail "the entry does not run the chosen folder's uninstaller"
! reg query "$entry\\Ely Office ElyProbe" >/dev/null 2>&1 || fail "the single-app entry stays"
reg query "HKCU\Software\Ely Office" /v Root | grep -qF "$(cygpath -w "$chosen")" ||
  fail "the chosen folder is not remembered"

# Update: only installed apps, each where its key points.
moved=$work/moved
mv "$chosen/ElyProbe" "$moved"
reg add "HKCU\Software\Ely Office\ElyProbe" /v InstallDir /d "$(cygpath -w "$moved")" /f >/dev/null
reg delete "HKCU\Software\Ely Office\ElyProbeThree" /f >/dev/null
rm -rf "$chosen/ElyProbeThree"
[ "$(run)" = 0 ] || fail "the update failed"
installed "$moved" || fail "ElyProbe did not update in place"
[ ! -e "$chosen/ElyProbe" ] || fail "ElyProbe went to the chosen folder too"
installed "$chosen/ElyProbeTwo" || fail "ElyProbeTwo is gone"
[ ! -e "$chosen/ElyProbeThree" ] || fail "an app not installed came back"

# 0.1.16 passes an app's folder; the root stays.
[ "$(nsis "$work/setup.exe" "/D=$(cygpath -w "$moved")")" = 0 ] || fail "the update from an app's folder failed"
[ ! -e "$work/uninstall.exe" ] || fail "the update moved the suite's root"
reg query "$entry\\Ely Office" /v UninstallString | grep -qF "$(cygpath -w "$chosen")\\uninstall.exe" ||
  fail "the entry left the chosen folder"

# A refused app stops the setup; later apps never run.
reg add "HKCU\Software\Ely Office\ElyProbeThree" /v InstallDir /d "$(cygpath -w "$chosen/ElyProbeThree")" /f >/dev/null
rm -f "$work/held"
ELY_HELD=$(cygpath -w "$chosen/ElyProbeTwo/ely-probe-two.exe") ELY_MARK=$(cygpath -w "$work/held") \
  powershell -NoProfile -Command \
  '$f=[IO.File]::Open($env:ELY_HELD,"Open","Read","Read"); New-Item $env:ELY_MARK | Out-Null; Start-Sleep 60' &
holder=$!
until [ -e "$work/held" ]; do
  kill -0 "$holder" 2>/dev/null || fail "the file lock did not take"
  sleep 0.2
done
result=$(run)
kill %% 2>/dev/null; wait 2>/dev/null || true
[ "$result" = 2 ] || fail "the setup exited $result over a refused app"
grep -q ElyProbeTwo "$chosen/ElyProbeTwo/ely-probe-two.exe" || fail "the refused app changed"
installed "$moved" || fail "the first app is gone"
[ ! -e "$chosen/ElyProbeThree" ] || fail "the setup went on after a refusal"

# An old entry that will not go stops the setup.
reg add "$entry\\Ely Office ElyProbe" /v UninstallString /d old /f >/dev/null
deny Add Delete "$entry\\Ely Office ElyProbe"
[ "$(run)" = 2 ] || fail "the setup passed over an old entry it could not delete"
reg query "$entry\\Ely Office ElyProbe" >/dev/null 2>&1 || fail "the denied entry is gone"
deny Remove Delete "$entry\\Ely Office ElyProbe"
reg delete "$entry\\Ely Office ElyProbe" /f >/dev/null

# A second setup waits; the first quits holding.
rm -f "$work/held" "$work/code"
ELY_MARK=$(cygpath -w "$work/held") powershell -NoProfile -Command \
  '$m=New-Object Threading.Mutex($true,"Global\ElyOfficeSetup-$env:USERNAME"); New-Item $env:ELY_MARK | Out-Null
  Start-Sleep 60' &
holder=$!
until [ -e "$work/held" ]; do
  kill -0 "$holder" 2>/dev/null || fail "the mutex did not take"
  sleep 0.2
done
rm "$chosen/ElyProbeTwo/ely-probe-two.exe"
(run >"$work/code") &
waiter=$!
# Time to finish, were the setup not waiting.
sleep 5
kill -0 "$waiter" 2>/dev/null || fail "the setup did not wait for the one before"
[ ! -e "$chosen/ElyProbeTwo/ely-probe-two.exe" ] || fail "the setup wrote while another ran"
kill "$holder"
wait "$waiter"
[ "$(cat "$work/code")" = 0 ] || fail "the waiting setup failed"
[ -f "$chosen/ElyProbeTwo/ely-probe-two.exe" ] || fail "the waiting setup did not install"

# Uninstall: installed apps go, others are not listed.
[ "$(run)" = 0 ] || fail "the install before uninstall failed"
reg delete "HKCU\Software\Ely Office\ElyProbeTwo" /f >/dev/null
reg add "$entry\\Ely Office ElyProbeThree" /v DisplayName /d "Ely Office ElyProbeThree" /f >/dev/null
cp "$chosen/uninstall.exe" "$work/uninstall.exe"

# An app key that cannot be read stops the uninstall.
deny Add QueryValues "HKCU\\Software\\Ely Office\\ElyProbe"
[ "$(nsis "$work/uninstall.exe" "_?=$(cygpath -w "$chosen")")" = 2 ] || fail "the uninstall passed an unreadable key"
installed "$moved" || fail "an unreadable app was removed"
reg query "$entry\\Ely Office" >/dev/null 2>&1 || fail "the entry went with an app left"
deny Remove QueryValues "HKCU\\Software\\Ely Office\\ElyProbe"
[ "$(nsis "$work/uninstall.exe" "_?=$(cygpath -w "$chosen")")" = 0 ] || fail "the uninstall failed"
[ ! -e "$moved" ] || fail "ElyProbe stays"
[ ! -e "$chosen/ElyProbeThree" ] || fail "ElyProbeThree stays: the uninstaller's ticks are off"
! reg query "$entry\\Ely Office ElyProbeThree" >/dev/null 2>&1 || fail "a removed app's old entry stays"
installed "$chosen/ElyProbeTwo" || fail "an unlisted app was removed"
[ ! -e "$chosen/uninstall.exe" ] || fail "the uninstaller stays"
! reg query "$entry\\Ely Office" >/dev/null 2>&1 || fail "the entry stays"
rm -rf "$work"
echo "PASS: windows setup"
