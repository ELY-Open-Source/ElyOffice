#!/usr/bin/env bash
# bundle.sh shares one kernel only when the copies match.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
[ "$(uname)" = Linux ] || {
  echo "SKIP: Linux only"
  exit 0
}
base=$(mktemp -d)
trap 'rm -rf "$base"' EXIT
# A path that is no regular expression.
work=$base/odd[
mkdir "$work"
fail() {
  echo "FAIL: $*" >&2
  exit 1
}
arch=$(uname -m)

# Only what bundle.sh reads; nfpm records its calls.
plant() {
  local tree=$1 slug name batch
  mkdir -p "$tree/packaging/linux" "$tree/engines" "$tree/dist"
  cp "$root/packaging/linux/bundle.sh" "$tree/packaging/linux/"
  cp "$root/packaging/apps.tsv" "$tree/packaging/"
  echo 'version = "9.9.9"' >"$tree/Cargo.toml"
  printf '#!/bin/sh\necho "$@" >>"%s/nfpm.log"\n' "$tree" >"$tree/engines/nfpm-2.47.0-$arch"
  chmod +x "$tree/engines/nfpm-2.47.0-$arch"
  while IFS=$'\t' read -r slug name _ batch _; do
    [ "$batch" = 1 ] || continue
    local package=$tree/target/package/$slug
    mkdir -p "$package/$name.AppDir/usr/bin"
    echo app >"$package/$name.AppDir/usr/bin/$slug"
    for kind in deb rpm archlinux; do echo "lib-$kind" >"$package/depends.$kind"; done
    case $slug in ely-doc | ely-table | ely-show)
      mkdir -p "$package/$name.AppDir/usr/bin/libreoffice/program" "$package/$name.AppDir/usr/bin/libreoffice/share/template/common"
      echo kernel >"$package/$name.AppDir/usr/bin/libreoffice/program/libmergedlo.so"
      ;;
    esac
  done <"$root/packaging/apps.tsv"
  # ElyShow alone carries the presentation templates.
  mkdir -p "$tree/target/package/ely-show/ElyShow.AppDir/usr/bin/libreoffice/share/template/common/presnt"
}
kernel_of() { echo "$1/target/package/$2/$3.AppDir/usr/bin/libreoffice"; }
bundle() { ELY_DIST=$1/dist "$1/packaging/linux/bundle.sh" >"$1/out.log" 2>&1; }

plant "$work/same"
bundle "$work/same" || fail "equal kernels did not bundle: $(cat "$work/same/out.log")"
[ "$(wc -l <"$work/same/nfpm.log")" = 3 ] || fail "nfpm did not make the three packages"
shared=$work/same/target/package/ely-office/opt
[ -f "$shared/libreoffice/program/libmergedlo.so" ] || fail "the shared kernel is missing"
[ ! -e "$shared/ely-doc/bin/libreoffice" ] || fail "ElyDoc kept a kernel of its own"

plant "$work/differs"
echo other >"$(kernel_of "$work/differs" ely-table ElyTable)/program/libmergedlo.so"
! bundle "$work/differs" || fail "a differing kernel was shared"
grep -q "ely-table's kernel is not ElyShow's" "$work/differs/out.log" || fail "the differing kernel is not named"
[ ! -e "$work/differs/nfpm.log" ] || fail "a package was made after a differing kernel"

# A link to nothing: diff cannot read it.
plant "$work/unread"
library=$(kernel_of "$work/unread" ely-doc ElyDoc)/program/libmergedlo.so
rm "$library"
ln -s /nonexistent/library "$library"
! bundle "$work/unread" || fail "a kernel that could not be read was shared"
grep -q "ely-doc's kernel did not compare" "$work/unread/out.log" || fail "the unread kernel is not named"
[ ! -e "$work/unread/nfpm.log" ] || fail "a package was made after an unread kernel"
echo "linux-bundle-rules: one kernel is shared only when every copy compares equal"
