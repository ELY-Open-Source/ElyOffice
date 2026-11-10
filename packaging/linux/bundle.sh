#!/usr/bin/env bash
# One deb, rpm and Arch package holding every first-batch app.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
dist=${ELY_DIST:-$root/dist}
broken() {
  echo "bundle.sh: $*" >&2
  exit 1
}
arch=$(uname -m)
case $arch in
  x86_64) goarch=amd64 ;;
  aarch64) goarch=arm64 ;;
  *) broken "no package for $arch" ;;
esac
nfpm=$root/engines/nfpm-2.47.0-$arch
[ -x "$nfpm" ] || broken "no $nfpm; run package.sh first"
version=$(awk -F'"' '/^version = / { print $2; exit }' "$root/Cargo.toml")
packages=$root/target/package
work=$packages/ely-office
rm -rf "$work"
mkdir -p "$work/opt"

slugs=() kernels=()
entries=$work/entries.yaml
: >"$entries" >"$work/licenses"
while IFS=$'\t' read -r slug name _ batch license _; do
  [ "$batch" = 1 ] || continue
  usr=$packages/$slug/$name.AppDir/usr
  [ -d "$usr" ] || broken "no $usr; run package.sh $slug first"
  # Links, not copies: a kernel alone is 500 MB.
  cp -al "$usr" "$work/opt/$slug"
  slugs+=("$slug")
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
    # ElyShow's holds the others' and its own templates.
    status=0
    extra=$(grep -vxF "Only in libreoffice/share/template/common: presnt" <<<"$differs") || status=$?
    [ "$status" -le 1 ] || broken "$slug's kernel differences did not filter"
    [ -z "$extra" ] || broken "$slug's kernel is not ElyShow's: $extra"
    rm -rf "$own"
  fi
  cat >>"$entries" <<ENTRY
  - src: /opt/ely-office/libreoffice
    dst: /opt/ely-office/$slug/bin/libreoffice
    type: symlink
ENTRY
done

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
