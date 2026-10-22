#!/usr/bin/env bash
# deb, rpm and Arch packages install cleanly and link.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
dist=${ELY_DIST:-$root/dist}
fail() {
  echo "FAIL: $*" >&2
  exit 1
}
command -v docker >/dev/null || {
  echo "SKIP: no docker"
  exit 0
}
version=$(awk -F'"' '/^version = / { print $2; exit }' "$root/Cargo.toml")
batch=$(awk -F'\t' '$4 == 1 { print $1 }' "$root/packaging/apps.tsv")
# Entry points exist; every ELF file finds its libraries.
check='set -euo pipefail
slug=$1
entry=/opt/ely-office/$slug/bin/$slug
[ -x "/usr/bin/$slug" ] || { echo "no /usr/bin/$slug"; exit 1; }
[ "$(readlink -f "/usr/bin/$slug")" = "$entry" ] || { echo "bad /usr/bin/$slug"; exit 1; }
[ -f "/usr/share/applications/$slug.desktop" ] || { echo "no desktop entry"; exit 1; }
[ -f "/usr/share/icons/hicolor/512x512/apps/$slug.png" ] || { echo "no icon"; exit 1; }
for lib in libvulkan.so.1 libwayland-client.so.0; do
  ldconfig -p | grep "$lib" >/dev/null || { echo "no $lib"; exit 1; }
done
ldd "$entry" >/dev/null || { echo "$entry is not a dynamic executable"; exit 1; }
find "/opt/ely-office/$slug" -type f -print0 >/tmp/files || { echo "cannot list files"; exit 1; }
checked=0
while IFS= read -r -d "" file; do
  magic=$(head -c 4 "$file" | od -An -c | tr -d " ") || { echo "cannot read $file"; exit 1; }
  [ "$magic" = 177ELF ] || continue
  checked=$((checked + 1))
  if ! out=$(ldd "$file" 2>&1); then
    # git-lfs is static Go: it must run.
    [ "${file##*/}" = git-lfs ] && "$file" version >/dev/null && continue
    echo "$file: $out"; exit 1
  fi
  if [[ $out == *"not found"* ]]; then
    echo "$file:"; printf "%s\n" "$out" | grep "not found"; exit 1
  fi
done </tmp/files
[ "$checked" -gt 0 ] || { echo "no ELF files"; exit 1; }
echo "$slug: $checked ELF files link"'
run() { # image, install command, package file, slug
  docker run --rm -v "$dist:/dist:ro" "$1" bash -c "{ $2 /dist/${3##*/}; } >/tmp/install.log 2>&1 ||
    { tail -20 /tmp/install.log; exit 1; }; bash -c '$check' _ $4" ||
    fail "$3 does not install or link in $1"
}
# The oldest glibc served: 2.31.
debian=debian:11@sha256:6f519a81440354a85eb592c5f32109ab80605f6b892455983a6f618bf87fabe9
ubuntu=ubuntu:26.04@sha256:f144425ff09be612d6d9ad965196e9cdc23dae1f42110a8a11a3e9a8198759f7
fedora=fedora:44@sha256:43b29f65a41eb9c35e1cd5323e3bdf3b655c2357a9f4f1ff2f9c2798e5045d80
arch=archlinux@sha256:4e77cf2ea5f410e6f8be5abf93ccf17ce2436e87a138c167208356711a405dbd
for slug in $batch; do
  deb=$dist/${slug}_$version-1_amd64.deb
  rpm=$dist/$slug-$version-1.x86_64.rpm
  pkg=$dist/$slug-$version-1-x86_64.pkg.tar.zst
  for file in "$deb" "$rpm" "$pkg"; do
    [ -f "$file" ] || fail "no $file; run packaging/linux/package.sh $slug"
  done
  run "$debian" "apt-get update -q && apt-get install -y -q" "$deb" "$slug"
  run "$ubuntu" "apt-get update -q && apt-get install -y -q" "$deb" "$slug"
  run "$fedora" "dnf install -y -q" "$rpm" "$slug"
  run "$arch" "pacman -Sy --noconfirm && pacman -U --noconfirm" "$pkg" "$slug"
done
echo "linux-packages: deb, rpm and Arch packages install and link"
