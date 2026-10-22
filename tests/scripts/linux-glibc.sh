#!/usr/bin/env bash
# Linux builds run on Debian 11 and Ubuntu 20.04.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
# Their glibc, and GCC 10's libstdc++.
limits=(GLIBC_2.31 GLIBCXX_3.4.28 CXXABI_1.3.12)
fail() {
  echo "FAIL: $*" >&2
  exit 1
}
for tool in readelf; do
  command -v "$tool" >/dev/null || {
    echo "SKIP: no $tool"
    exit 0
  }
done
release=$root/target/alma8/release
lo=$root/engines/lo-src/instdir/program
required=("$release"/ely-{doc,table,show,pdf,mail,tree,mark,lok-worker,pdf-worker} "$lo/libmergedlo.so"
  "$root/engines/git/bin/git")
for file in "${required[@]}"; do
  [ -f "$file" ] || fail "no $file; build in packaging/linux/container.sh"
  magic=$(head -c 4 "$file" | od -An -c | tr -d ' ') || fail "cannot read $file"
  [ "$magic" = '177ELF' ] || fail "$file is not an ELF file"
done
list=$(mktemp)
trap 'rm -f "$list"' EXIT
find -L "$release"/ely-{doc,table,show,pdf,mail,tree,mark,lok-worker,pdf-worker} "$lo" "$root/engines/git" \
  -type f -print0 >"$list" || fail "cannot list every file"
checked=0
while IFS= read -r -d '' file; do
  magic=$(head -c 4 "$file" | od -An -c | tr -d ' ') || fail "cannot read $file"
  [ "$magic" = '177ELF' ] || continue
  checked=$((checked + 1))
  headers=$(readelf -lW "$file") || fail "readelf cannot read $file"
  # Static binaries, like git-lfs, need no glibc.
  grep -q ' DYNAMIC ' <<<"$headers" || continue
  # Every version it needs, ABI tags included.
  versions=$(readelf -VW "$file") || fail "readelf cannot read the versions of $file"
  needs=$(awk '/^Version needs section/ { on = 1; next } /^Version / { on = 0 }
    on { for (i = 1; i < NF; i++) if ($i == "Name:") print $(i + 1) }' <<<"$versions")
  for limit in "${limits[@]}"; do
    name=${limit%_*} most=${limit##*_}
    while IFS= read -r version; do
      [[ $version == "${name}"_* ]] || continue
      number=${version#"${name}"_}
      [[ $number =~ ^[0-9.]+$ ]] || fail "${file#"$root"/} needs $version, which $limit lacks"
      [ "$(printf '%s\n%s\n' "$number" "$most" | sort -V | tail -n 1)" = "$most" ] ||
        fail "${file#"$root"/} needs $name $number, above $most"
    done <<<"$needs"
  done
done <"$list"
echo "linux: $checked ELF files need ${limits[*]} at most"
