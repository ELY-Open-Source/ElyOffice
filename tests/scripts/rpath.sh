#!/usr/bin/env bash
# Usage: rpath.sh <app> <binary>; its @rpath loads resolve inside.
set -euo pipefail

binary=${2:?a Mach-O file in it}
name=${binary#"${1:?the .app}"/}
app=$(realpath "$1")
fail() {
  echo "FAIL: $*" >&2
  exit 1
}
header=$(otool -hv "$binary") || fail "otool cannot read the header of $name"
# Libraries load through their executable's paths.
grep -qw EXECUTE <<<"$header" || exit 0
commands=$(otool -l "$binary") || fail "otool cannot read the load commands of $name"
libraries=$(otool -L "$binary") || fail "otool cannot read the libraries of $name"
here=$(dirname "$binary")
rpaths=$(awk '$2 == "LC_RPATH" { getline; getline; print $2 }' <<<"$commands")
loads=$(awk 'NR > 1 && $1 ~ /^@rpath\// { print $1 }' <<<"$libraries")
while IFS= read -r load; do
  [ -n "$load" ] || continue
  found=
  while IFS= read -r path; do
    path=${path/#@executable_path/$here}
    path=${path/#@loader_path/$here}
    [ -e "$path/${load#@rpath/}" ] || continue
    # Physical paths: no .. or link leaves the app.
    real=$(realpath "$path/${load#@rpath/}") || fail "cannot resolve $path/${load#@rpath/}"
    if [[ $real == "$app"/* ]]; then found=1; fi
  done <<<"$rpaths"
  [ -n "$found" ] || fail "$name loads $load; no path in the app has it"
done <<<"$loads"
