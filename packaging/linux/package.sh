#!/usr/bin/env bash
# Usage: package.sh [--desktop] <slug>; AppImage, deb, rpm, Arch.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
desktop_only=
[ "${1:-}" = --desktop ] && {
  desktop_only=1
  shift
}
slug=${1:?app slug, such as ely-doc}
dist=${ELY_DIST:-$root/dist}

row=$(awk -F'\t' -v slug="$slug" '$1 == slug { print $2 "\t" $3 }' "$root/packaging/apps.tsv")
[ -n "$row" ] || {
  echo "package.sh: unknown app $slug" >&2
  exit 1
}
name=${row%%$'\t'*}
categories=${row##*$'\t'}
# Ely is MIT; bundled engines keep their licences.
case $slug in
  ely-doc | ely-table | ely-show) license="MIT AND MPL-2.0" ;;
  ely-pdf) license="MIT AND AGPL-3.0-or-later" ;;
  ely-tree) license="MIT AND GPL-2.0-only" ;;
  *) license=MIT ;;
esac
# The types it opens, from packaging/types.tsv; none drops the line.
mimes=$(awk -F'\t' -v slug="$slug" '$1 == slug && !seen[$3]++ { printf "%s;", $3 }' "$root/packaging/types.tsv")
desktop_entry() {
  sed -e "s/@NAME@/$name/" -e "s/@SLUG@/$slug/g" -e "s/@CATEGORIES@/$categories/" -e "s|@MIME@|$mimes|" \
    -e '/^MimeType=$/d' "$root/packaging/linux/app.desktop.in"
}

if [ -n "$desktop_only" ]; then
  desktop_entry
  exit 0
fi

[ "$(uname)" = Linux ] || {
  echo "package.sh: Linux only" >&2
  exit 1
}
command -v patchelf >/dev/null || {
  echo "package.sh: no patchelf; run in packaging/linux/container.sh" >&2
  exit 1
}
arch=$(uname -m)
case $arch in
  x86_64)
    goarch=amd64
    nfpm_arch=x86_64
    pack_sha=ed4ce84f0d9caff66f50bcca6ff6f35aae54ce8135408b3fa33abfc3cb384eb0
    deploy_sha=c20cd71e3a4e3b80c3483cef793cda3f4e990aca14014d23c544ca3ce1270b4d
    runtime_sha=2fca8b443c92510f1483a883f60061ad09b46b978b2631c807cd873a47ec260d
    nfpm_sha=0660ca602b2d2d2ae4781a06c692b3eeb9d437ffea05b831d76e41f4a3188783
    ;;
  aarch64)
    goarch=arm64
    nfpm_arch=arm64
    pack_sha=f0837e7448a0c1e4e650a93bb3e85802546e60654ef287576f46c71c126a9158
    deploy_sha=620095110d693282b8ebeb244a95b5e911cf8f65f76c88b4b47d16ae6346fcff
    runtime_sha=00cbdfcf917cc6c0ff6d3347d59e0ca1f7f45a6df1a428a0d6d8a78664d87444
    nfpm_sha=1c0f5f2999b9a974bfb04fdb0cc3306096de530ac5dbb25d739cc5f5219c919c
    ;;
  *)
    echo "package.sh: no packaging tools for $arch" >&2
    exit 1
    ;;
esac
icon=$root/assets/icons/$slug.png
[ -f "$icon" ] || {
  echo "package.sh: $icon is missing" >&2
  exit 1
}

fetch_tool() { # file url sha256
  if [ ! -f "$1" ]; then
    mkdir -p "$root/engines"
    curl -fsSL -o "$1" "$2"
  fi
  echo "$3  $1" | sha256sum -c - >/dev/null
  chmod +x "$1"
}
pack=$root/engines/appimagetool-1.9.1-$arch.AppImage
deploy=$root/engines/linuxdeploy-1-alpha-20251107-1-$arch.AppImage
runtime=$root/engines/type2-runtime-20251108-$arch
nfpm_tar=$root/engines/nfpm_2.47.0_Linux_$nfpm_arch.tar.gz
fetch_tool "$pack" "https://github.com/AppImage/appimagetool/releases/download/1.9.1/appimagetool-$arch.AppImage" "$pack_sha"
fetch_tool "$deploy" "https://github.com/linuxdeploy/linuxdeploy/releases/download/1-alpha-20251107-1/linuxdeploy-$arch.AppImage" "$deploy_sha"
fetch_tool "$runtime" "https://github.com/AppImage/type2-runtime/releases/download/20251108/runtime-$arch" "$runtime_sha"
fetch_tool "$nfpm_tar" "https://github.com/goreleaser/nfpm/releases/download/v2.47.0/nfpm_2.47.0_Linux_$nfpm_arch.tar.gz" "$nfpm_sha"
nfpm=$root/engines/nfpm-2.47.0-$arch
if [ ! -x "$nfpm" ]; then
  tar -xzf "$nfpm_tar" -O nfpm >"$nfpm.part"
  chmod +x "$nfpm.part"
  mv "$nfpm.part" "$nfpm"
fi

stage=$root/target/stage/$slug
"$root/packaging/stage.sh" "$slug" "$stage"
work=$root/target/package/$slug
rm -rf "$work"
mkdir -p "$work"
desktop_entry >"$work/$slug.desktop"
appdir=$work/$name.AppDir
bin=$appdir/usr/bin
mkdir -p "$bin"
cp -R "$stage"/. "$bin/"
deployed=(--executable "$bin/$slug")
for helper in ely-lok-worker ely-pdf-worker; do
  if [ -f "$bin/$helper" ]; then
    deployed+=(--executable "$bin/$helper")
  fi
done

# linuxdeploy bundles libraries and fails on a missing one.
"$deploy" --appimage-extract-and-run --appdir "$appdir" "${deployed[@]}" \
  --desktop-file "$work/$slug.desktop" --icon-file "$icon"

mkdir -p "$dist"
version=$(awk -F'"' '/^version = / { print $2; exit }' "$root/Cargo.toml")
out=$dist/$name-$version-linux-$arch.AppImage
ARCH=$arch "$pack" --appimage-extract-and-run --runtime-file "$runtime" "$appdir" "$out"
echo "$out"

# Libraries the AppDir needs and does not bundle.
broken() {
  echo "package.sh: $*" >&2
  exit 1
}
find "$appdir" \( -type f -o -type l \) -name '*.so*' -print0 >"$work/libs.list" || broken "cannot list $appdir"
find "$appdir" -type f -print0 >"$work/files.list" || broken "cannot list $appdir"
real=$(realpath "$appdir")
declare -A bundled=()
while IFS= read -r -d '' file; do
  bundled[${file##*/}]=1
done <"$work/libs.list"
# gpui loads these by name.
needed=(libvulkan.so.1 libwayland-client.so.0)
while IFS= read -r -d '' file; do
  magic=$(head -c 4 "$file" | od -An -c | tr -d ' ') || broken "cannot read $file"
  [ "$magic" = '177ELF' ] || continue
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
    target=$(realpath -m "$(dirname "$file")$rest") || broken "cannot resolve $path for $file"
    [[ $target == "$real" || $target == "$real"/* ]] ||
      broken "${file#"$appdir"/} loads from $path, outside the package"
  done
  while IFS= read -r lib; do
    [ -n "${bundled[$lib]:-}" ] || needed+=("$lib")
  done < <(sed -n 's/.*Shared library: \[\(.*\)\]/\1/p' <<<"$dynamic")
done <"$work/files.list"
deb=() pacman=() rpm=()
while IFS= read -r lib; do
  row=$(awk -F'\t' -v lib="$lib" '$1 == lib' "$root/packaging/linux/depends.tsv")
  [ -n "$row" ] || {
    broken "$lib is in no row of packaging/linux/depends.tsv"
  }
  IFS=$'\t' read -r _ deb_name pacman_name <<<"$row"
  deb+=("$deb_name")
  pacman+=("$pacman_name")
  rpm+=("$lib()(64bit)")
done < <(printf '%s\n' "${needed[@]}" | sort -u)
yaml_list() { printf '      - "%s"\n' "$@" | sort -u; }

# deb, rpm and Arch: the AppDir's usr under /opt/ely-office.
cat >"$work/nfpm.yaml" <<EOF
name: $slug
arch: $goarch
platform: linux
version: $version
release: 1
maintainer: ZacharyZhang
vendor: ZacharyZhang
description: $name, part of Ely Office
homepage: https://git.zacharyzhang.com/ZacharyZhang-NY/ElyOffice
license: $license
contents:
  - src: $appdir/usr
    dst: /opt/ely-office/$slug
    type: tree
  - src: /opt/ely-office/$slug/bin/$slug
    dst: /usr/bin/$slug
    type: symlink
  - src: $work/$slug.desktop
    dst: /usr/share/applications/$slug.desktop
  - src: $icon
    dst: /usr/share/icons/hicolor/512x512/apps/$slug.png
overrides:
  deb:
    depends:
$(yaml_list "${deb[@]}")
  rpm:
    depends:
$(yaml_list "${rpm[@]}")
  archlinux:
    depends:
$(yaml_list "${pacman[@]}")
EOF
for packager in deb rpm archlinux; do
  "$nfpm" package --config "$work/nfpm.yaml" --packager "$packager" --target "$dist/"
done
