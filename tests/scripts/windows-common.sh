# Probe apps through the unified setup; sourced by tests.
case "$(uname -s)" in MINGW* | MSYS* | CYGWIN*) ;; *) echo "SKIP: not Windows"; exit 0 ;; esac
makensis=${MAKENSIS:-"/c/Program Files (x86)/NSIS/makensis.exe"}
[ -x "$makensis" ] || { echo "SKIP: no makensis"; exit 0; }
export MSYS2_ARG_CONV_EXCL='*'
fail() { echo "FAIL: $*" >&2; exit 1; }

# The setup's own entry, folder and login, saved for restore.
runkey='HKCU\Software\Microsoft\Windows\CurrentVersion\Run'
saved=$(mktemp -d)
for key in 'Software\Ely Office' 'Software\Microsoft\Windows\CurrentVersion\Uninstall\Ely Office' "${runkey#HKCU\\}"; do
  ! reg query "HKCU\\$key" >/dev/null 2>&1 || reg export "HKCU\\$key" "$(cygpath -w "$saved/$(echo "$key" | tr '\\ ' '__').reg")" /y >/dev/null
done
# Forgets the suite, so /D picks a new root.
unsuite() {
  reg delete 'HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\Ely Office' /f >/dev/null 2>&1 || true
  reg delete 'HKCU\Software\Ely Office' /v Root /f >/dev/null 2>&1 || true
}
restore() {
  local bad=
  [ ! -e "$LOCALAPPDATA/Programs/Ely Office/ElyProbe" ] || { echo "FAIL: a probe landed in the real Ely Office folder" >&2; bad=1; }
  unsuite
  reg delete "$runkey" /v "Ely Office" /f >/dev/null 2>&1 || true
  for file in "$saved"/*.reg; do
    [ ! -e "$file" ] || reg import "$(cygpath -w "$file")" >/dev/null 2>&1 || { echo "FAIL: could not restore $file; it stays" >&2; bad=1; }
  done
  [ -z "$bad" ] || exit 1
  rm -rf "$saved"
}

# nsis EXE LAST: NSIS wants /D= or _?= last, unquoted.
nsis() {
  # Git Bash quotes spaced arguments; PowerShell passes them raw.
  ELY_EXE=$(cygpath -w "$1") ELY_LAST=$2 powershell -NoProfile -Command \
    'exit (Start-Process -FilePath $env:ELY_EXE -ArgumentList "/S $env:ELY_LAST" -Wait -PassThru).ExitCode' &&
    echo 0 || echo $?
}

# setup OUT VERSION STAGES NAME:SLUG...; stage, list, types per slug.
setup() {
  local out=$1 version=$2 stages=$3 app name slug shares
  shift 3
  # A one-file kernel, unless the test stages its own.
  [ -d "$stages/kernel" ] || { mkdir -p "$stages/kernel/program"; echo id >"$stages/kernel/program/ely-kernel-id"; }
  "$root/packaging/windows/files.sh" "$stages/kernel" >"$stages/kernel-files.txt"
  echo '!macro ElyAll M' >"$stages/apps.nsh"
  for app in "$@"; do
    IFS=: read -r name slug shares <<<"$app"
    cp "$root/assets/icons/ely-mark.ico" "$stages/$slug/$slug.ico"
    "$root/packaging/windows/files.sh" "$stages/$slug" >"$stages/$slug-files.txt"
    printf '  !insertmacro ${M} %s %s %s\n' "$name" "$slug" "${shares:-0}" >>"$stages/apps.nsh"
  done
  echo '!macroend' >>"$stages/apps.nsh"
  "$makensis" -V1 -INPUTCHARSET UTF8 -DVERSION="$version" -DAPPS="$(cygpath -w "$stages/apps.nsh")" \
    -DSTAGES="$(cygpath -w "$stages")" \
    -DOUTFILE="$(cygpath -w "$out")" "$(cygpath -w "$root/packaging/windows/installer.nsi")"
}

# untyped NAME FILE: types macros that write nothing.
untyped() { printf '!macro ElyTypesInstall%s\n!macroend\n!macro ElyTypesUninstall%s\n!macroend\n' "$1" "$1" >"$2"; }
