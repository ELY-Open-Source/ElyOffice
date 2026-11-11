#!/usr/bin/env bash
# Installed types open, leave others' alone, and fail loudly.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
makensis=${MAKENSIS:-"/c/Program Files (x86)/NSIS/makensis.exe"}
case "$(uname -s)" in MINGW* | MSYS* | CYGWIN*) ;; *) echo "SKIP: not Windows"; exit 0 ;; esac
[ -x "$makensis" ] || { echo "SKIP: no makensis"; exit 0; }
export MSYS2_ARG_CONV_EXCL='*'

work=$root/target/types-test
c='Software\Classes'
# A made-up app and type keep real types out.
class="$c\\ElyOffice.ElyProbe" app="$c\\Applications\\ely-probe.exe" listing="$c\\.elyprobe\\OpenWithProgids"
start="$c\\Applications\\ely-probe-start.exe"
command="$class\\shell\\open\\command"
# A made-up scheme for Default Apps.
url="$c\\ElyOffice.ElyProbe.Url" caps='Software\ElyOffice\ElyProbe\Capabilities' listed_apps='Software\RegisteredApplications'
keys=("$class" "$start" "$c\\.elyprobe" "$c\\Other.ElyProbe" "$app" "$c\\Directory\\shell\\ElyOffice.ElyProbe" "$url" "$c\\elyprobe" 'Software\ElyOffice\ElyProbe'
  'Software\Ely Office\ElyProbe' 'Software\Microsoft\Windows\CurrentVersion\Uninstall\Ely Office ElyProbe')
# Set-Acl wants write access, which the rule denies.
undeny() {
  ELY_KEY=$1 powershell -NoProfile -Command '$k = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey($env:ELY_KEY, [Microsoft.Win32.RegistryKeyPermissionCheck]::ReadWriteSubTree, [Security.AccessControl.RegistryRights]"ReadPermissions, ChangePermissions"); if ($k) { $a = $k.GetAccessControl(); $a.PurgeAccessRules([Security.Principal.WindowsIdentity]::GetCurrent().User); $k.SetAccessControl($a); $k.Close() }'
}
deny() { # key [rights]
  # reg add would blank an existing default.
  reg query "HKCU\\$1" >/dev/null 2>&1 || reg add "HKCU\\$1" /f >/dev/null
  ELY_KEY=$1 ELY_RIGHTS=${2:-SetValue, CreateSubKey} powershell -NoProfile -Command '$k = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey($env:ELY_KEY, [Microsoft.Win32.RegistryKeyPermissionCheck]::ReadWriteSubTree, [Security.AccessControl.RegistryRights]"ReadPermissions, ChangePermissions"); $a = $k.GetAccessControl(); $a.AddAccessRule((New-Object Security.AccessControl.RegistryAccessRule([Security.Principal.WindowsIdentity]::GetCurrent().User, [Security.AccessControl.RegistryRights]$env:ELY_RIGHTS, "Deny"))); $k.SetAccessControl($a); $k.Close()' ||
    fail "the deny rule on $1 did not take"
}
clean() {
  for key in "$class" "$app" "$listing" "$command" "$url" "$c\\.elyprobe" "$c\\.elyprobe\\ShellNew"; do undeny "$key"; done
  for key in "${keys[@]}"; do reg delete "HKCU\\$key" /f >/dev/null 2>&1 || true; done
  reg delete "HKCU\\$listed_apps" /v ElyProbe /f >/dev/null 2>&1 || true
  rm -f "$APPDATA/Microsoft/Windows/Start Menu/Programs/ElyProbe.lnk"
  # The shell caches names by exe path.
  ELY_WORK=$(cygpath -w "$work") powershell -NoProfile -Command '$k = "HKCU:\Software\Classes\Local Settings\Software\Microsoft\Windows\Shell\MuiCache"; if (Test-Path $k) { (Get-Item $k).Property | Where-Object { $_ -like "$env:ELY_WORK*" } | ForEach-Object { Remove-ItemProperty $k -Name $_ } }'
  ! reg query "HKCU\\$class" >/dev/null 2>&1 || { echo "FAIL: the probe class stays" >&2; exit 1; }
}
fail() { echo "FAIL: $*" >&2; exit 1; }
trap clean EXIT
clean
rm -rf "$work"
mkdir -p "$work"

# The real types.sh, over a table naming only the probe.
tree=$work/tree
mkdir -p "$tree/packaging/windows/new"
cp "$root/packaging/windows/types.sh" "$tree/packaging/windows/"
printf 'ely-probe\tElyProbe\tUtility;\t1\tMIT\n' >"$tree/packaging/apps.tsv"
printf 'ely-probe\telyprobe\tapplication/x-elyprobe\nely-probe\t/\tinode/directory\nely-probe\telyprobe:\tx-scheme-handler/elyprobe\n' >"$tree/packaging/types.tsv"
: >"$tree/packaging/windows/new/new.elyprobe"
"$tree/packaging/windows/types.sh" ely-probe >"$work/types.nsh"
stage=$work/stage
mkdir -p "$stage"
cp /c/Windows/System32/PING.EXE "$stage/ely-probe.exe"
cp /c/Windows/System32/PING.EXE "$stage/ely-probe-start.exe"
: >"$stage/new.elyprobe"
"$root/packaging/windows/files.sh" "$stage" ely-probe >"$work/files.txt"
"$makensis" -V1 -INPUTCHARSET UTF8 -DAPP_NAME=ElyProbe -DAPP_SLUG=ely-probe -DVERSION=1 \
  -DSTAGE="$(cygpath -w "$stage")" -DICON="$(cygpath -w "$root/assets/icons/ely-mark.ico")" \
  -DTYPES="$(cygpath -w "$work/types.nsh")" -DLIST="$(cygpath -w "$work/files.txt")" \
  -DOUTFILE="$(cygpath -w "$work/setup.exe")" "$(cygpath -w "$root/packaging/windows/installer.nsi")"
# The macros alone, writing the key they blame.
cat >"$work/probe.nsi" <<'NSI'
!include LogicLib.nsh
!include "${TYPES}"
Name probe
OutFile "${OUT}"
RequestExecutionLevel user
SilentInstall silent
Section
  StrCpy $INSTDIR "${DIR}"
  !ifdef UNDO
    !insertmacro ElyTypesUninstall
  !else
    !insertmacro ElyTypesInstall
  !endif
  FileOpen $4 "${SAID}" w
  FileWrite $4 "$2"
  FileClose $4
SectionEnd
NSI
blamed() { # dir [UNDO]
  local defines=(-DTYPES="$(cygpath -w "$work/types.nsh")" -DDIR="$(cygpath -w "$1")"
    -DSAID="$(cygpath -w "$work/said.txt")" -DOUT="$(cygpath -w "$work/probe.exe")")
  [ $# -lt 2 ] || defines+=(-DUNDO)
  "$makensis" -V1 "${defines[@]}" "$(cygpath -w "$work/probe.nsi")" >/dev/null
  "$work/probe.exe"
  cat "$work/said.txt"
}

install() { "$work/setup.exe" /S "/D=$(cygpath -w "$1")" && echo 0 || echo $?; }
uninstall() { cp "$1/uninstall.exe" "$work/uninstall.exe" && "$work/uninstall.exe" /S "_?=$(cygpath -w "$1")" && echo 0 || echo $?; }
# reg prints a placeholder for a missing value.
value() {
  ELY_KEY=$1 ELY_NAME=${2:-} powershell -NoProfile -Command '$k = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey($env:ELY_KEY); if ($k) { $k.GetValue($env:ELY_NAME) }' | tr -d '\r'
}
opens() { echo "\"$(cygpath -w "$1")\\ely-probe-start.exe\" ely-probe ElyProbe \"%1\""; }
listed() { reg query "HKCU\\$listing" /v ElyOffice.ElyProbe >/dev/null 2>&1; }
# The name the shell shows for a class's open command.
named() {
  ELY_CLASS=$1 ELY_SIG='[DllImport("shlwapi.dll", CharSet = CharSet.Unicode)] public static extern int AssocQueryStringW(int flags, int str, string assoc, string extra, System.Text.StringBuilder outs, ref int size);' \
    powershell -NoProfile -Command 'Add-Type -Namespace Ely -Name Assoc -MemberDefinition $env:ELY_SIG; $b = New-Object Text.StringBuilder 260; $n = 260; [void][Ely.Assoc]::AssocQueryStringW(0, 4, $env:ELY_CLASS, "open", $b, [ref]$n); $b.ToString()' | tr -d '\r'
}

# A handlerless type: listed, New-able, and gone after uninstall.
a=$work/a
[ "$(install "$a")" = 0 ] || fail "fresh install"
[ "$(value "$class\\shell\\open\\command")" = "$(opens "$a")" ] || fail "fresh: no open command"
[ "$(value "$c\\Directory\\shell\\ElyOffice.ElyProbe\\command")" = "$(opens "$a")" ] || fail "fresh: no folder command"
listed || fail "fresh: Open With misses it"
[ "$(named ElyOffice.ElyProbe)" = ElyProbe ] || fail "fresh: Open With names $(named ElyOffice.ElyProbe), not ElyProbe"
[ "$(value "$c\\.elyprobe")" = ElyOffice.ElyProbe ] || fail "fresh: New has no class"
[ "$(value "$c\\.elyprobe\\ShellNew" FileName)" = "$(cygpath -w "$a")\\new.elyprobe" ] || fail "fresh: no New"
[ "$(value "$url\\shell\\open\\command")" = "$(opens "$a")" ] || fail "fresh: links do not open"
[ "$(value "$caps\\URLAssociations" elyprobe)" = ElyOffice.ElyProbe.Url ] || fail "fresh: no URL association"
[ "$(value "$listed_apps" ElyProbe)" = "$caps" ] || fail "fresh: Default Apps misses it"
! reg query "HKCU\\$c\\elyprobe" >/dev/null 2>&1 || fail "fresh: the scheme was taken over"

# Moved: the old folder's uninstall leaves the new one working.
b=$work/b
[ "$(install "$b")" = 0 ] || fail "second install"
[ "$(value "$c\\.elyprobe\\ShellNew" FileName)" = "$(cygpath -w "$b")\\new.elyprobe" ] || fail "moved: New kept the old folder"
[ "$(uninstall "$a")" = 0 ] || fail "old uninstall"
[ "$(value "$class\\shell\\open\\command")" = "$(opens "$b")" ] || fail "moved: the old uninstall took the open command"
[ "$(value "$c\\Directory\\shell\\ElyOffice.ElyProbe\\command")" = "$(opens "$b")" ] || fail "moved: the old uninstall took the folder menu"
listed || fail "moved: the old uninstall took Open With"
[ "$(value "$start" FriendlyAppName)" = ElyProbe ] || fail "moved: the old uninstall took the launcher's name"
[ "$(value "$url\\shell\\open\\command")" = "$(opens "$b")" ] || fail "moved: the old uninstall took links"
[ "$(value "$listed_apps" ElyProbe)" = "$caps" ] || fail "moved: the old uninstall took Default Apps"
[ "$(uninstall "$b")" = 0 ] || fail "last uninstall"
for key in "$class" "$c\\Directory\\shell\\ElyOffice.ElyProbe" "$c\\.elyprobe\\ShellNew"; do
  ! reg query "HKCU\\$key" >/dev/null 2>&1 || fail "uninstall left $key"
done
! listed || fail "uninstall left Open With"
! reg query "HKCU\\$start" >/dev/null 2>&1 || fail "uninstall left the launcher's name"
[ -z "$(value "$c\\.elyprobe")" ] || fail "uninstall left the class"
for key in "$url" 'Software\ElyOffice\ElyProbe'; do
  ! reg query "HKCU\\$key" >/dev/null 2>&1 || fail "uninstall left $key"
done
[ -z "$(value "$listed_apps" ElyProbe)" ] || fail "uninstall left Default Apps"

# Another app's type: its default and New stay.
reg add "HKCU\\$c\\Other.ElyProbe\\shell\\open\\command" /ve /d 'C:\Other\other.exe "%1"' /f >/dev/null
reg add "HKCU\\$c\\.elyprobe" /ve /d Other.ElyProbe /f >/dev/null
reg add "HKCU\\$c\\.elyprobe\\ShellNew" /v FileName /d 'C:\Other\original.elyprobe' /f >/dev/null
d=$work/d
[ "$(install "$d")" = 0 ] || fail "install beside another"
[ "$(value "$c\\.elyprobe")" = Other.ElyProbe ] || fail "another: the default changed"
[ "$(value "$c\\.elyprobe\\ShellNew" FileName)" = 'C:\Other\original.elyprobe' ] || fail "another: New changed"
listed || fail "another: Open With misses it"
[ "$(uninstall "$d")" = 0 ] || fail "uninstall beside another"
[ "$(value "$c\\.elyprobe\\ShellNew" FileName)" = 'C:\Other\original.elyprobe' ] || fail "another: uninstall took New"
[ "$(value "$c\\Other.ElyProbe\\shell\\open\\command")" = 'C:\Other\other.exe "%1"' ] || fail "another: its class went"
clean

# A key that cannot be written fails the install, named.
deny "$class"
[ "$(install "$work/e")" != 0 ] || fail "denied class: the install reported success"
! listed || fail "denied class: Open With names an unusable class"
[ "$(blamed "$work/e")" = "HKCU\\$class" ] || fail "denied class: blamed $(cat "$work/said.txt")"
clean
deny "$url"
[ "$(install "$work/u")" != 0 ] || fail "denied link class: the install reported success"
[ -z "$(value "$listed_apps" ElyProbe)" ] || fail "denied link class: Default Apps names it"
clean
for key in "$app" "$listing" "$url"; do
  deny "$key"
  [ "$(blamed "$work/f")" = "HKCU\\$key" ] || fail "denied $key: blamed $(cat "$work/said.txt")"
  clean
done

# Uninstall fails, named, when it cannot delete or check.
for denied in "$listing|SetValue" "$listing|SetValue, QueryValues" "$app|Delete, EnumerateSubKeys" "$command|QueryValues" \
  "$c\\.elyprobe|SetValue, QueryValues" "$c\\.elyprobe\\ShellNew|SetValue, QueryValues"; do
  key=${denied%%|*} rights=${denied#*|} g=$work/g
  [ "$(install "$g")" = 0 ] || fail "install before denying $rights on $key"
  deny "$key" "$rights"
  [ "$(uninstall "$g")" != 0 ] || fail "$rights on $key: the uninstall reported success"
  [ "$(blamed "$g" UNDO)" = "HKCU\\$key" ] || fail "$rights on $key: blamed $(cat "$work/said.txt")"
  undeny "$key"
  [ "$(value "$command")" = "$(opens "$g")" ] || fail "$rights on $key: the class went while still named"
  clean
done
# Types only: an unreadable command keeps the launcher's name.
g=$work/h
[ "$(install "$g")" = 0 ] || fail "install before the types-only check"
reg delete "HKCU\\$url" /f >/dev/null
reg delete "HKCU\\$c\\Directory\\shell\\ElyOffice.ElyProbe" /f >/dev/null
deny "$command" QueryValues
[ "$(uninstall "$g")" != 0 ] || fail "types only: the uninstall reported success"
undeny "$command"
[ "$(value "$start" FriendlyAppName)" = ElyProbe ] || fail "types only: the launcher's name went while still named"
clean
rm -rf "$work"
echo "PASS: windows types"
