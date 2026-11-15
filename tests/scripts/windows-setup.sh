#!/usr/bin/env bash
# One setup: chosen folder, apps, refusals, one kernel, login.
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
  # The real document apps hide; restore brings them back.
  for name in ElyDoc ElyTable ElyShow; do reg delete "HKCU\\Software\\Ely Office\\$name" /f >/dev/null 2>&1 || true; done
  for name in $names; do
    reg delete "HKCU\\Software\\Ely Office\\$name" /f >/dev/null 2>&1 || true
    reg delete "$entry\\Ely Office $name" /f >/dev/null 2>&1 || true
    rm -f "$APPDATA/Microsoft/Windows/Start Menu/Programs/$name.lnk"
  done
}
trap 'kill $(jobs -p) 2>/dev/null || true; clean; restore' EXIT
clean
rm -rf "$work"
mkdir -p "$work/data"
# ely-login logs here, never in the user's data.
export ELY_DATA_ROOT=$(cygpath -w "$work/data")
login=$(cargo build -q -p ely-login --message-format=json | sed -n 's/.*"executable":"\([^"]*\)".*/\1/p' | tail -n 1 | sed 's/\\\\/\\/g')
[ -n "$login" ] || fail "ely-login did not build"

stages=$work/stages apps=()
for name in $names; do
  slug=$(echo "$name" | sed 's/\([A-Z]\)/-\L\1/g; s/^-//')
  mkdir -p "$stages/$slug"
  echo "$name" >"$stages/$slug/$slug.exe"
  untyped "$name" "$stages/$slug-types.nsh"
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

# An installed suite never moves to another folder.
[ "$(nsis "$work/setup.exe" "/D=$(cygpath -w "$work/elsewhere")")" = 2 ] || fail "the setup moved to another folder"
[ ! -e "$work/elsewhere" ] || fail "the refused setup wrote another folder"

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
reg delete "HKCU\Software\Ely Office\ElyProbe" /f >/dev/null 2>&1 || true
rm -rf "$chosen"

# Two apps share one kernel; the third has none.
shared=$work/shared
mkdir -p "$shared/kernel/program"
echo kernel >"$shared/kernel/program/soffice.txt"
echo id >"$shared/kernel/program/ely-kernel-id"
cp "$(cygpath -u "$login")" "$shared/kernel/"
for name in $names; do
  slug=$(echo "$name" | sed 's/\([A-Z]\)/-\L\1/g; s/^-//')
  cp -r "$stages/$slug" "$stages/$slug-types.nsh" "$shared/"
done
setup "$work/shared.exe" 1 "$shared" ElyProbe:ely-probe:1 ElyProbeTwo:ely-probe-two:1 ElyProbeThree:ely-probe-three:0
share() { nsis "$work/shared.exe" "/D=$(cygpath -w "$chosen")"; }
# linked DIR: DIR is a junction to the kernel.
linked() {
  [ "$(ELY_DIR=$(cygpath -w "$1") powershell -NoProfile -Command '$i=Get-Item $env:ELY_DIR; "$($i.LinkType) $($i.Target)"' | tr -d '\r')" = \
    "Junction $(cygpath -w "$chosen/Kernel")" ]
}
key() { reg add "HKCU\Software\Ely Office\ElyProbe" /v InstallDir /d "$(cygpath -w "$1")" /f >/dev/null; }
# unjoin DIR: only the link goes, never its target.
unjoin() { ELY_DIR=$(cygpath -w "$1") powershell -NoProfile -Command '[IO.Directory]::Delete($env:ELY_DIR)'; }

# A planted login never runs, on either run.
plant() { mkdir -p "$chosen/Kernel"; cp "$(cygpath -u "$login")" "$chosen/Kernel/ely-login.exe"; }
unrun() {
  [ ! -s "$work/data/ely-login.log" ] || fail "$1 ran a planted login"
  ! reg query "$runkey" /v "Ely Office" >/dev/null 2>&1 || fail "$1 starts a planted login"
}
plant
for pass in first second; do
  [ "$(run)" = 0 ] || fail "the $pass install without a kernel failed"
  unrun "the $pass install without a kernel"
done
cp "$chosen/uninstall.exe" "$work/uninstall.exe"
[ "$(nsis "$work/uninstall.exe" "_?=$(cygpath -w "$chosen")")" = 0 ] || fail "the uninstall beside a planted login failed"
unrun "the uninstall"
unsuite
rm -rf "$chosen"
plant
for pass in first second; do
  [ "$(share)" = 2 ] || fail "the $pass kernel took a planted login"
  unrun "the $pass refused kernel"
done
unsuite
rm -rf "$chosen"

# No document app: an app in Kernel's place stays.
mkdir -p "$chosen/Kernel"
echo old >"$chosen/Kernel/ely-probe-three.exe"
printf 'ely-probe-three.exe\r\n' >"$chosen/Kernel/.ely-installed"
reg add "HKCU\Software\Ely Office\ElyProbeThree" /v InstallDir /d "$(cygpath -w "$chosen/Kernel")" /f >/dev/null
plant
[ "$(share)" = 0 ] || fail "the update with an app in the kernel's place failed"
unrun "an app's list in the kernel's place"
grep -q ElyProbeThree "$chosen/Kernel/ely-probe-three.exe" || fail "the update removed an app in the kernel's place"
[ ! -e "$chosen/Kernel/program" ] || fail "the kernel went into an app's folder"
reg delete "HKCU\Software\Ely Office\ElyProbeThree" /f >/dev/null
rm -rf "$chosen/Kernel"

# An app installed where the kernel goes stops the setup.
mkdir -p "$chosen/Kernel"
echo old >"$chosen/Kernel/ely-probe.exe"
printf 'ely-probe.exe\r\n' >"$chosen/Kernel/.ely-installed"
key "$chosen/Kernel"
[ "$(share)" = 2 ] || fail "the kernel took an app's folder"
grep -q old "$chosen/Kernel/ely-probe.exe" || fail "the app in the kernel's place changed"
[ ! -e "$chosen/Kernel/program" ] || fail "the kernel went into an app's folder"
rm -rf "$chosen/Kernel"

# A link to another folder refuses; it is never re-pointed.
old=$chosen/ElyProbe
mkdir -p "$old" "$work/other/program"
echo other >"$work/other/program/soffice.txt"
echo old >"$old/ely-probe.exe"
printf 'ely-probe.exe\r\n' >"$old/.ely-installed"
ELY_LINK=$(cygpath -w "$old/libreoffice") ELY_TO=$(cygpath -w "$work/other") powershell -NoProfile -Command \
  'New-Item -ItemType Junction -Path $env:ELY_LINK -Target $env:ELY_TO | Out-Null'
key "$old"
[ "$(share)" = 2 ] || fail "the setup took a link to another folder"
grep -q old "$old/ely-probe.exe" || fail "the refused app did not come back"
[ "$(ELY_DIR=$(cygpath -w "$old/libreoffice") powershell -NoProfile -Command '(Get-Item $env:ELY_DIR).Target' | tr -d '\r')" = \
  "$(cygpath -w "$work/other")" ] || fail "the link was re-pointed"
grep -q other "$work/other/program/soffice.txt" || fail "the other folder changed"
reg delete "HKCU\Software\Ely Office\ElyProbe" /f >/dev/null
unjoin "$old/libreoffice"
rm -rf "$old" "$work/other"

# An unlisted file in the old kernel refuses the link.
old=$chosen/ElyProbe
mkdir -p "$old/libreoffice/program"
echo old >"$old/ely-probe.exe"
echo old >"$old/libreoffice/program/old.txt"
echo mine >"$old/libreoffice/keep.txt"
printf 'libreoffice\\\r\nlibreoffice\\program\\\r\nely-probe.exe\r\nlibreoffice\\program\\old.txt\r\n' >"$old/.ely-installed"
key "$old"
[ "$(share)" = 2 ] || fail "the setup linked over a file it did not install"
grep -q old "$old/ely-probe.exe" || fail "the refused app did not come back"
grep -q old "$old/libreoffice/program/old.txt" || fail "the old kernel did not come back"
grep -q mine "$old/libreoffice/keep.txt" || fail "the unlisted file went"
rm "$old/libreoffice/keep.txt"
[ "$(share)" = 0 ] || fail "the update from an old kernel failed"
linked "$old/libreoffice" || fail "the old kernel did not give way to the link"
[ ! -e "$old/.ely-old" ] || fail "the backup stays"
reg delete "HKCU\Software\Ely Office\ElyProbe" /f >/dev/null
unjoin "$old/libreoffice"
rm -rf "$old"

[ "$(share)" = 0 ] || fail "the shared install failed"
installed "$chosen/Kernel" || fail "no kernel"
# started: the Run value names the kernel's login.
started() { reg query "$runkey" /v "Ely Office" 2>/dev/null | tr -d '\r' | grep -qF "\"$(cygpath -w "$chosen/Kernel")\\ely-login.exe\""; }
started || fail "no start at login"
grep -q "ely-login .* started" "$work/data/ely-login.log" || fail "the setup did not warm the engines"
for name in ElyProbe ElyProbeTwo; do
  linked "$chosen/$name/libreoffice" || fail "$name does not link to the kernel"
  grep -q kernel "$chosen/$name/libreoffice/program/soffice.txt" || fail "$name cannot read the kernel"
  ! grep -qi libreoffice "$chosen/$name/.ely-installed" || fail "$name lists the kernel as its own"
done
[ ! -e "$chosen/ElyProbeThree/libreoffice" ] || fail "an app without a kernel links one"
[ "$(share)" = 0 ] || fail "the shared update failed"
linked "$chosen/ElyProbe/libreoffice" || fail "the update broke the link"
# A lost list: the legacy walk never enters the link.
rm "$chosen/ElyProbe/.ely-installed"
[ "$(share)" = 0 ] || fail "the update without a list failed"
grep -q kernel "$chosen/Kernel/program/soffice.txt" || fail "a legacy update took the kernel's files"
linked "$chosen/ElyProbe/libreoffice" || fail "a legacy update broke the link"
# A downgrade lists the kernel's files; they stay the kernel's.
printf 'libreoffice\\\r\nlibreoffice\\program\\\r\nlibreoffice\\program\\soffice.txt\r\n' >>"$chosen/ElyProbe/.ely-installed"
[ "$(share)" = 0 ] || fail "the update after a downgrade failed"
grep -q kernel "$chosen/Kernel/program/soffice.txt" || fail "the update took the kernel's files"
printf 'libreoffice\\program\\soffice.txt\r\n' >>"$chosen/ElyProbe/.ely-installed"

# An engine that will not stop changes nothing.
bad=$work/bad
mkdir -p "$bad"
cp "$SYSTEMROOT/System32/findstr.exe" "$bad/ely-doc.exe"
reg add "HKCU\Software\Ely Office\ElyDoc" /v InstallDir /d "$(cygpath -w "$bad")" /f >/dev/null
cp "$chosen/ElyProbe/.ely-installed" "$work/before"
[ "$(share)" = 2 ] || fail "the setup went on past an engine that would not stop"
cmp -s "$work/before" "$chosen/ElyProbe/.ely-installed" || fail "a refused stop changed an app"
grep -q "ERROR.*ely-doc: .*engine stop exited" "$work/data/ely-login.log" || fail "the refused stop is not logged"
"$(cygpath -u "$chosen/Kernel/ely-login.exe")" && fail "a failed engine exited 0"
grep -q "ERROR.*ely-doc: .*engine exited" "$work/data/ely-login.log" || fail "the failed engine is not logged"
reg delete "HKCU\Software\Ely Office\ElyDoc" /f >/dev/null
rm -rf "$bad"

# A refused app: the earlier one went, the kernel stays.
cp "$chosen/uninstall.exe" "$work/uninstall.exe"
rm -f "$work/held"
ELY_HELD=$(cygpath -w "$chosen/ElyProbeTwo/ely-probe-two.exe") ELY_MARK=$(cygpath -w "$work/held") \
  powershell -NoProfile -Command \
  '$f=[IO.File]::Open($env:ELY_HELD,"Open","Read","Read"); New-Item $env:ELY_MARK | Out-Null; Start-Sleep 60' &
holder=$!
until [ -e "$work/held" ]; do
  kill -0 "$holder" 2>/dev/null || fail "the file lock did not take"
  sleep 0.2
done
result=$(nsis "$work/uninstall.exe" "_?=$(cygpath -w "$chosen")")
kill %% 2>/dev/null; wait 2>/dev/null || true
[ "$result" = 2 ] || fail "the uninstall exited $result over a refused app"
[ ! -e "$chosen/ElyProbe" ] || fail "ElyProbe stays"
grep -q kernel "$chosen/Kernel/program/soffice.txt" || fail "removing ElyProbe took the kernel's files"
linked "$chosen/ElyProbeTwo/libreoffice" || fail "ElyProbeTwo lost its kernel"

# No document app left: the kernel goes with the last.
reg delete "HKCU\Software\Ely Office\ElyProbeTwo" /f >/dev/null
unjoin "$chosen/ElyProbeTwo/libreoffice"
rm -rf "$chosen/ElyProbeTwo"
[ "$(share)" = 0 ] || fail "the update with only ElyProbeThree failed"
[ ! -e "$chosen/Kernel" ] || fail "the kernel stays with no document app"
! reg query "$runkey" /v "Ely Office" >/dev/null 2>&1 || fail "the start at login stays without the kernel"
installed "$chosen/ElyProbeThree" || fail "ElyProbeThree is gone"
[ "$(share)" = 0 ] || fail "the reinstall failed"
cp "$chosen/uninstall.exe" "$work/uninstall.exe"
[ "$(nsis "$work/uninstall.exe" "_?=$(cygpath -w "$chosen")")" = 0 ] || fail "the shared uninstall failed"
[ ! -e "$chosen" ] || fail "the chosen folder stays: $(cd "$chosen" && find .)"
rm -rf "$work"
echo "PASS: windows setup"
