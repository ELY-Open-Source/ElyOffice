#!/usr/bin/env bash
# Usage: elfs.sh <appdir> <scratch>; prints libraries it needs unbundled.
set -euo pipefail

appdir=${1:?AppDir}
work=${2:?scratch directory}
broken() {
  echo "elfs.sh: $*" >&2
  exit 1
}
find "$appdir" \( -type f -o -type l \) -name '*.so*' -print0 >"$work/libs.list" || broken "cannot list $appdir"
find "$appdir" -type f -print0 >"$work/files.list" || broken "cannot list $appdir"
# Resolves $1 into `resolved`; keeps a final newline.
resolve() {
  resolved=$(realpath -m -- "$1" && printf x) || broken "cannot resolve $1"
  resolved=${resolved%x}
  resolved=${resolved%$'\n'}
}
resolve "$appdir"
real=$resolved
declare -A bundled=()
while IFS= read -r -d '' file; do
  bundled[${file##*/}]=1
done <"$work/libs.list"
needed=()
while IFS= read -r -d '' file; do
  magic=$(head -c 4 "$file" | od -An -c | tr -d ' ') || broken "cannot read $file"
  [ "$magic" = '177ELF' ] || continue
  sections=$(readelf -SW "$file") || broken "readelf cannot read $file"
  ! grep -qE ' \.(symtab|debug_)' <<<"$sections" || broken "${file#"$appdir"/} keeps symbols or debug info"
  dynamic=$(readelf -dW "$file") || broken "readelf cannot read $file"
  # Library paths resolve inside the package.
  records=$(awk '/\((RUNPATH|RPATH)\)/ { n++ } END { print n + 0 }' <<<"$dynamic") || broken "cannot count paths of $file"
  [ "$records" -le 1 ] || broken "${file#"$appdir"/} has $records library path records"
  lines=()
  if [ "$records" -eq 1 ]; then
    # The raw string; readelf's text can hide a newline.
    path=$(patchelf --print-rpath "$file" && printf x) || broken "patchelf cannot read $file"
    path=${path%x}
    path=${path%$'\n'}
    [[ $path != *$'\n'* ]] || broken "${file#"$appdir"/} has a newline in its library path"
    lines=("$path")
  fi
  entries=()
  for line in "${lines[@]}"; do
    # An empty entry searches the working directory.
    [[ $line != :* && $line != *: && $line != *::* && -n $line ]] ||
      broken "${file#"$appdir"/} searches the working directory: $line"
    IFS=: read -r -a parts <<<"$line"
    entries+=("${parts[@]}")
  done
  for path in "${entries[@]}"; do
    case $path in
      '$ORIGIN' | '$ORIGIN/'*) rest=${path#'$ORIGIN'} ;;
      '${ORIGIN}' | '${ORIGIN}/'*) rest=${path#'${ORIGIN}'} ;;
      *) broken "${file#"$appdir"/} loads from $path, not from \$ORIGIN" ;;
    esac
    # The loader expands $LIB and $PLATFORM; this check cannot.
    [[ $rest != *'$'* ]] || broken "${file#"$appdir"/} loads from $path, a loader token after \$ORIGIN"
    resolve "${file%/*}$rest"
    [[ $resolved == "$real" || $resolved == "$real"/* ]] ||
      broken "${file#"$appdir"/} loads from $path, outside the package"
  done
  while IFS= read -r lib; do
    [ -n "${bundled[$lib]:-}" ] || needed+=("$lib")
  done < <(sed -n 's/.*Shared library: \[\(.*\)\]/\1/p' <<<"$dynamic")
done <"$work/files.list"
[ "${#needed[@]}" -eq 0 ] || printf '%s\n' "${needed[@]}" | sort -u
