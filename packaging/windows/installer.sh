#!/usr/bin/env bash
# Builds dist/ElyOffice-<version>-windows-x86_64-setup.exe with the first batch.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
dist=${ELY_DIST:-$root/dist}
version=$(awk -F'"' '/^version = / { print $2; exit }' "$root/Cargo.toml")
makensis=${MAKENSIS:-"/c/Program Files (x86)/NSIS/makensis.exe"}
[ -x "$makensis" ] || {
  echo "installer.sh: no makensis at $makensis; install NSIS" >&2
  exit 1
}
stages=$root/target/stage
apps=$stages/windows-apps.nsh
mkdir -p "$stages" "$dist"
batch=$(awk -F'\t' '$4 == 1 { print $1 }' "$root/packaging/apps.tsv")
echo '!macro ElyAll M' >"$apps"
for slug in $batch; do
  name=$(awk -F'\t' -v slug="$slug" '$1 == slug { print $2 }' "$root/packaging/apps.tsv")
  stage=$stages/$slug
  "$root/packaging/stage.sh" "$slug" "$stage"
  # Explorer's New copies these; the install list owns them.
  for ext in $(awk -F'\t' -v slug="$slug" '$1 == slug { print $2 }' "$root/packaging/types.tsv"); do
    [ ! -e "$root/packaging/windows/new/new.$ext" ] || cp "$root/packaging/windows/new/new.$ext" "$stage/"
  done
  "$root/packaging/windows/types.sh" "$slug" >"$stages/$slug-types.nsh"
  "$root/packaging/windows/files.sh" "$stage" "$slug" >"$stages/$slug-files.txt"
  printf '  !insertmacro ${M} %s %s\n' "$name" "$slug" >>"$apps"
done
echo '!macroend' >>"$apps"
out=$dist/ElyOffice-$version-windows-x86_64-setup.exe
"$makensis" -V2 -INPUTCHARSET UTF8 -DVERSION="$version" -DAPPS="$(cygpath -w "$apps")" \
  -DSTAGES="$(cygpath -w "$stages")" -DICONS="$(cygpath -w "$root/assets/icons")" \
  -DOUTFILE="$(cygpath -w "$out")" "$(cygpath -w "$root/packaging/windows/installer.nsi")"
echo "$out"
# 0.1.16 reads its own app's manifest; each names this setup.
for slug in $batch; do "$root/packaging/manifest.sh" "$slug" windows-x86_64 "$out"; done
