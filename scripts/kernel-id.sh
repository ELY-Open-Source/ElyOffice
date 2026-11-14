#!/usr/bin/env bash
# kernel-id.sh <kernel> <id-file>: hash the kernel.
set -euo pipefail

kernel=${1:?the kernel folder, such as engines/lo-src/instdir}
file=${2:?where the id goes, named ely-kernel-id}
[ "$(basename "$file")" = ely-kernel-id ] || {
  echo "kernel-id.sh: $file is not named ely-kernel-id" >&2
  exit 1
}
# A failed run leaves no id.
rm -f "$file"
[ -d "$kernel" ] || {
  echo "kernel-id.sh: no folder $kernel" >&2
  exit 1
}
if command -v sha256sum >/dev/null; then
  hash=(sha256sum)
else
  hash=(shasum -a 256)
fi
names=$(mktemp)
sums=$(mktemp)
file=$(cd "$(dirname "$file")" && pwd)/ely-kernel-id
part=$file.part
trap 'rm -f "$names" "$sums" "$part"' EXIT
cd "$kernel"
# Sorted files and links, the id itself excluded.
find . -type f ! -name 'ely-kernel-id*' -print0 | LC_ALL=C sort -z >"$names"
[ -s "$names" ] || {
  echo "kernel-id.sh: no files in $kernel" >&2
  exit 1
}
xargs -0 "${hash[@]}" <"$names" >"$sums"
find . -type l -print0 | LC_ALL=C sort -z | while IFS= read -r -d '' link; do
  target=$(readlink "$link")
  printf 'link %s -> %s\n' "$link" "$target"
done >>"$sums"
sum=$("${hash[@]}" <"$sums")
id=${sum%% *}
printf '%s\n' "$id" >"$part"
echo "kernel-id.sh: $file is $id"
mv "$part" "$file"
