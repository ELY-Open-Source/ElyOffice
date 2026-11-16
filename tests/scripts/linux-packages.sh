#!/usr/bin/env bash
# The ely-office deb, rpm and Arch package install and link.
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
find -L "/opt/ely-office/$slug" -type f -print0 >/tmp/files || { echo "cannot list files"; exit 1; }
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
# The oldest glibc served: 2.31.
debian=debian:11@sha256:6f519a81440354a85eb592c5f32109ab80605f6b892455983a6f618bf87fabe9
ubuntu=ubuntu:26.04@sha256:f144425ff09be612d6d9ad965196e9cdc23dae1f42110a8a11a3e9a8198759f7
fedora=fedora:44@sha256:43b29f65a41eb9c35e1cd5323e3bdf3b655c2357a9f4f1ff2f9c2798e5045d80
arch=archlinux@sha256:4e77cf2ea5f410e6f8be5abf93ccf17ce2436e87a138c167208356711a405dbd
# 0.1.16's own ElyTree package, faked: the suite replaces it.
nfpm=$root/engines/nfpm-2.47.0-x86_64
[ -x "$nfpm" ] || fail "no $nfpm; run packaging/linux/package.sh"
old=$(mktemp -d)
trap 'rm -rf "$old"' EXIT
echo old >"$old/old"
printf 'name: ely-tree\narch: amd64\nplatform: linux\nversion: 0.1.15\nrelease: 1\nmaintainer: test\ndescription: the old ElyTree\nlicense: MIT\ncontents:\n  - src: %s/old\n    dst: /opt/ely-office/ely-tree/old\n' "$old" >"$old/nfpm.yaml"
for packager in deb rpm archlinux; do "$nfpm" package --config "$old/nfpm.yaml" --packager "$packager" --target "$old/" >/dev/null; done
# What systemctl --global enable would write.
wants="for slug in ely-doc ely-table ely-show; do [ \"\$(readlink /etc/systemd/user/graphical-session.target.wants/ely-engine@\$slug.service)\" = /usr/lib/systemd/user/ely-engine@.service ] || exit 1; done"
gone="[ -z \"\$(ls /etc/systemd/user/graphical-session.target.wants)\" ]"
# The suite: one package for every app, replacing theirs.
suite() { # image, install command, suite file, query for ElyTree's own package, reinstall, remove
  docker run --rm -v "$dist:/dist:ro" -v "$old:/old:ro" "$1" bash -c "{ $2 /dist/${3##*/}; } >/tmp/install.log 2>&1 ||
    { tail -20 /tmp/install.log; exit 1; }
    ! { $4; } >/dev/null 2>&1 || { echo 'ely-tree keeps its own package'; exit 1; }
    [ \"\$(readlink -f /opt/ely-office/ely-doc/bin/libreoffice)\" = /opt/ely-office/libreoffice ] ||
      { echo 'ElyDoc has a kernel of its own'; exit 1; }
    for slug in $(echo $batch); do bash -c '$check' _ \$slug || exit 1; done
    { $wants; } || { echo 'login does not want the engines'; exit 1; }
    { $5 /dist/${3##*/}; } >/tmp/again.log 2>&1 || { tail -20 /tmp/again.log; exit 1; }
    { $wants; } || { echo 'a reinstall dropped the engines'; exit 1; }
    { $6; } >/tmp/remove.log 2>&1 || { tail -20 /tmp/remove.log; exit 1; }
    { $gone; } || { echo 'the removal left login links'; exit 1; }" ||
    fail "$3 does not install, replace ely-tree, link, enable the engines or remove in $1"
}
for file in "$dist/ely-office_$version-1_amd64.deb" "$dist/ely-office-$version-1.x86_64.rpm" \
  "$dist/ely-office-$version-1-x86_64.pkg.tar.zst"; do
  [ -f "$file" ] || fail "no $file; run packaging/linux/bundle.sh"
done
# dpkg knows a removed deb; pacman -Q answers for providers.
kept="dpkg-query -W -f='\${db:Status-Status}' ely-tree | grep -qx installed"
suite "$debian" "sed -i s#deb.debian.org#archive.debian.org# /etc/apt/sources.list && apt-get update -q &&
  apt-get install -y -q /old/ely-tree_0.1.15-1_amd64.deb && apt-get install -y -q" \
  "ely-office_$version-1_amd64.deb" "$kept" "apt-get install -y -q --reinstall" "apt-get remove -y -q ely-office"
suite "$ubuntu" "apt-get update -q && apt-get install -y -q /old/ely-tree_0.1.15-1_amd64.deb &&
  apt-get install -y -q" "ely-office_$version-1_amd64.deb" "$kept" "apt-get install -y -q --reinstall" "apt-get remove -y -q ely-office"
suite "$fedora" "dnf install -y -q /old/ely-tree-0.1.15-1.x86_64.rpm && dnf install -y -q" \
  "ely-office-$version-1.x86_64.rpm" "rpm -q ely-tree" "rpm -U --replacepkgs" "rpm -e ely-office"
suite "$arch" "pacman -Sy --noconfirm && pacman -U --noconfirm /old/ely-tree-0.1.15-1-x86_64.pkg.tar.zst &&
  yes | pacman -U" "ely-office-$version-1-x86_64.pkg.tar.zst" "pacman -Qq ely-tree | grep -qx ely-tree" \
  "pacman -U --noconfirm" "pacman -R --noconfirm ely-office"
echo "linux-packages: the ely-office deb, rpm and Arch package install, replace the old ElyTree, link, enable the engines at login and remove them"
