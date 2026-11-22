#!/usr/bin/env bash
# The ely-office packages and the one AppImage.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
dist=${ELY_DIST:-$root/dist}
mkdir -p "$dist"
broken() {
  echo "bundle.sh: $*" >&2
  exit 1
}
arch=$(uname -m)
case $arch in
  x86_64) goarch=amd64 ;;
  *) broken "no package for $arch" ;;
esac
nfpm=$root/engines/nfpm-2.47.0-$arch
[ -x "$nfpm" ] || broken "no $nfpm; run package.sh first"
pack=$root/engines/appimagetool-1.9.1-$arch.AppImage
runtime=$root/engines/type2-runtime-20251108-$arch
[ -x "$pack" ] && [ -f "$runtime" ] || broken "no appimagetool or runtime; run package.sh first"
version=$(awk -F'"' '/^version = / { print $2; exit }' "$root/Cargo.toml")
packages=$root/target/package
work=$packages/ely-office
rm -rf "$work"
mkdir -p "$work/opt"

slugs=() kernels=()
entries=$work/entries.yaml
: >"$entries" >"$work/licenses" >"$work/apps.tsv"
while IFS=$'\t' read -r slug name _ batch license _; do
  [ "$batch" = 1 ] || continue
  usr=$packages/$slug/$name.AppDir/usr
  [ -d "$usr" ] || broken "no $usr; run package.sh $slug first"
  # Links, not copies: a kernel alone is 500 MB.
  cp -al "$usr" "$work/opt/$slug"
  slugs+=("$slug")
  printf '%s\t%s\n' "$slug" "$name" >>"$work/apps.tsv"
  [ ! -d "$usr/bin/libreoffice" ] || kernels+=("$slug")
  for kind in deb rpm archlinux; do
    cat "$packages/$slug/depends.$kind" >>"$work/depends.$kind"
  done
  sed 's/ AND /\n/g' <<<"$license" >>"$work/licenses"
  cat >>"$entries" <<ENTRY
  - src: /opt/ely-office/$slug/bin/$slug
    dst: /usr/bin/$slug
    type: symlink
  - src: $packages/$slug/$slug.desktop
    dst: /usr/share/applications/$slug.desktop
  - src: $root/assets/icons/$slug.png
    dst: /usr/share/icons/hicolor/512x512/apps/$slug.png
ENTRY
done <"$root/packaging/apps.tsv"

# One kernel serves ElyDoc, ElyTable and ElyShow.
kernel=$work/opt/libreoffice
mv "$work/opt/ely-show/bin/libreoffice" "$kernel"
for slug in "${kernels[@]}"; do
  own=$work/opt/$slug/bin/libreoffice
  if [ -d "$own" ]; then
    # Relative: diff quotes odd paths. 2: a failed read.
    status=0
    differs=$(cd "$work/opt" && LC_ALL=C diff -rq "$slug/bin/libreoffice" libreoffice) || status=$?
    [ "$status" -le 1 ] || broken "$slug's kernel did not compare with ElyShow's"
    [ -z "$differs" ] || broken "$slug's kernel is not ElyShow's: $differs"
    rm -rf "$own"
  fi
  cat >>"$entries" <<ENTRY
  - src: /opt/ely-office/libreoffice
    dst: /opt/ely-office/$slug/bin/libreoffice
    type: symlink
ENTRY
done

# Login starts each kernel app's idle engine.
unit=/usr/lib/systemd/user/ely-engine@.service
wants=/etc/systemd/user/graphical-session.target.wants
links=$(printf "$wants/ely-engine@%s.service " "${kernels[@]}")
cat >>"$entries" <<ENTRY
  - src: $root/packaging/linux/ely-engine@.service
    dst: $unit
ENTRY
# What systemctl --global enable writes.
printf '#!/bin/sh\nmkdir -p %s || exit 1\nfor link in %s; do ln -sf %s "$link" || exit 1; done\n' \
  "$wants" "$links" "$unit" >"$work/postinstall"
# Each user's idle engines stop, or the files stay.
printf '#!/bin/sh\nkernels="%s"\n' "${kernels[*]}" >"$work/preremove"
cat >>"$work/preremove" <<'STOP'
[ -d /run/systemd/system ] || exit 0
# In the user's manager; systemd 247 lacks --user -M.
as() { runuser -u "$user" -- env XDG_RUNTIME_DIR="$dir" "$@"; }
for dir in /run/user/*; do
  [ -d "$dir" ] || continue
  uid=${dir##*/}
  if ! systemctl is-active --quiet "user@$uid.service"; then
    echo "ely-office: user $uid has no systemd manager; skipped" >&2
    continue
  fi
  user=$(id -nu "$uid") || exit 1
  for slug in $kernels; do
    # Whoever started it: the engine itself stops.
    if ! as systemd-run --user --wait --quiet --collect -p RuntimeMaxSec=60 "/usr/bin/$slug" engine stop; then
      echo "ely-office: $user's $slug idle engine did not stop; removal stops. Retry: runuser -u $user -- env XDG_RUNTIME_DIR=$dir systemd-run --user --wait /usr/bin/$slug engine stop" >&2
      exit 1
    fi
  done
  # The units stop claiming a live engine.
  if ! as systemctl --user stop 'ely-engine@*.service'; then
    echo "ely-office: $user's engine units did not stop; removal stops" >&2
    exit 1
  fi
done
STOP
# An upgrade keeps the unit.
printf '#!/bin/sh\n[ -e %s ] || rm -f %s\n' "$unit" "$links" >"$work/postremove"

license=$(awk '!seen[$0]++' "$work/licenses" | paste -sd'|' | sed 's/|/ AND /g')
names=$(printf '  - %s\n' "${slugs[@]}")
yaml_list() { sort -u "$work/depends.$1" | sed 's/.*/      - "&"/'; }
cat >"$work/nfpm.yaml" <<YAML
name: ely-office
arch: $goarch
platform: linux
version: $version
release: 1
maintainer: ZacharyZhang
vendor: ZacharyZhang
description: Ely Office, every app in one package
homepage: https://github.com/ELY-Open-Source/ElyOffice
license: $license
contents:
  - src: $work/opt
    dst: /opt/ely-office
    type: tree
$(cat "$entries")
scripts:
  postinstall: $work/postinstall
  preremove: $work/preremove
  postremove: $work/postremove
conflicts:
$names
replaces:
$names
provides:
$names
deb:
  compression: xz
rpm:
  compression: xz
overrides:
  deb:
    depends:
$(yaml_list deb)
  rpm:
    depends:
$(yaml_list rpm)
  archlinux:
    depends:
$(yaml_list archlinux)
YAML
for packager in deb rpm archlinux; do
  "$nfpm" package --config "$work/nfpm.yaml" --packager "$packager" --target "$dist/"
done

# The AppImage: the same tree, its links relative.
appdir=$work/ElyOffice.AppDir
cp -al "$work/opt" "$appdir"
[ -s "$appdir/libreoffice/program/ely-kernel-id" ] || broken "the kernel has no ely-kernel-id"
for slug in "${kernels[@]}"; do ln -s ../../libreoffice "$appdir/$slug/bin/libreoffice"; done
cp "$work/apps.tsv" "$appdir/apps.tsv"
# AppRun keeps the worker by this.
sha256sum "$appdir/ely-doc/bin/ely-lok-worker" | cut -c1-16 >"$appdir/ely-worker-id"
cp "$root/packaging/linux/AppRun" "$appdir/AppRun"
cp "$root/packaging/linux/ely-engine@.service" "$appdir/"
cp "$root/assets/icons/ely-doc.png" "$appdir/ely-doc.png"
ln -s ely-doc.png "$appdir/.DirIcon"
cat >"$appdir/ely-office.desktop" <<DESKTOP
[Desktop Entry]
Type=Application
Name=Ely Office
Exec=AppRun
Icon=ely-doc
Categories=Office;
Terminal=false
DESKTOP
image=$dist/ElyOffice-$version-linux-$arch.AppImage
ARCH=$arch "$pack" --appimage-extract-and-run --runtime-file "$runtime" "$appdir" "$image"
"$root/packaging/manifest.sh" ely-office "linux-$arch" "$image"
# 0.1.16 reads its own app's manifest; each names this image.
for slug in "${slugs[@]}"; do "$root/packaging/manifest.sh" "$slug" "linux-$arch" "$image"; done
echo "$image"
