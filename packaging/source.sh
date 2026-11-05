#!/usr/bin/env bash
# Usage: source.sh <dist>; the source of what this commit builds.
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
dist=${1:?the folder to write into}
version=$(awk -F'"' '/^version = / { print $2; exit }' "$root/Cargo.toml")
# Binaries match a commit only when the tree is clean.
changed=$(git -C "$root" status --porcelain --untracked-files=all)
[ -z "$changed" ] || {
  echo "source.sh: uncommitted changes; commit before a release:" >&2
  echo "$changed" >&2
  exit 1
}
commit=$(git -C "$root" rev-parse --short=12 HEAD)
mkdir -p "$dist"
# Our source as published: the committed tree.
ours=$dist/ElyOffice-$version-$commit-source.tar.gz
git -C "$root" archive --format=tar.gz --prefix="ElyOffice-$version/" -o "$ours" HEAD
echo "$ours"
# Copyleft upstream as shipped: patched MuPDF (AGPL), Git (GPLv2).
upstream=$dist/ElyOffice-$version-$commit-upstream-source.tar.gz
[ -f "$root/engines/mupdf-src/COPYING" ] || {
  echo "source.sh: no engines/mupdf-src; run scripts/build-mupdf.sh" >&2
  exit 1
}
[ -f "$root/reference/git/COPYING" ] || {
  echo "source.sh: no reference/git; run scripts/fetch-references.sh" >&2
  exit 1
}
tar -czf "$upstream" --exclude .git --exclude .DS_Store -C "$root/engines" mupdf-src -C "$root/reference" git
echo "$upstream"
# Libraries LibreOffice's builds fetch, every platform's; hashes checked.
external=$dist/ElyOffice-$version-$commit-libreoffice-external-source.tar
built=$root/engines/lo-src/external/tarballs
pack=$(mktemp -d)
trap 'rm -rf "$pack"' EXIT
mkdir "$pack/libreoffice-external"
cp "$root/packaging/lo-tarballs.tsv" "$pack/libreoffice-external/"
while IFS=$'\t' read -r name hash url; do
  file=$pack/libreoffice-external/$name
  if [ -f "$built/$name" ]; then
    ln -s "$built/$name" "$file"
  else
    echo "source.sh: fetching $url"
    curl -fsSL -o "$file" "$url" || {
      echo "source.sh: cannot fetch $url" >&2
      exit 1
    }
  fi
  got=$(shasum -a 256 "$file" | cut -d ' ' -f 1)
  [ "$got" = "$hash" ] || {
    echo "source.sh: $name has sha256 $got, not $hash" >&2
    exit 1
  }
done <"$root/packaging/lo-tarballs.tsv"
tar -chf "$external" -C "$pack" libreoffice-external
echo "$external"
