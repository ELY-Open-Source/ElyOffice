#!/usr/bin/env bash
# aur.sh writes ely-office-bin from a release; refuses wrong ones.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
fail() {
  echo "FAIL: $*" >&2
  exit 1
}
command -v zstd >/dev/null || {
  echo "SKIP: no zstd"
  exit 0
}
# A release Arch package: only its metadata matters here.
pack() {
  local tag=$1 name=$2 ver=$3 dir=$work/pkg
  rm -rf "$dir"
  mkdir -p "$dir" "$work/release/$tag"
  printf 'pkgname = %s\npkgver = %s\npkgdesc = Ely Office, every app in one package\nurl = https://x.test\nlicense = MIT AND MPL-2.0\ndepend = glibc>=2.28\ndepend = zlib\nprovides = ely-doc\nprovides = ely-mark\n' \
    "$name" "$ver" >"$dir/.PKGINFO"
  printf 'post_install() {\n  true\n}\n' >"$dir/.INSTALL"
  (cd "$dir" && tar -cf - .PKGINFO .INSTALL) | zstd -q -o "$work/release/$tag/ely-office-${tag#v}-1-x86_64.pkg.tar.zst"
}
url="file://$(cd "$work" && pwd -W 2>/dev/null || pwd)/release"
run() { ELY_AUR_DOWNLOAD=$url/$1 ELY_DIST=$work/dist bash "$root/packaging/aur.sh" "$@"; }
pack v9.9.9 ely-office 9.9.9-1
run v9.9.9 >/dev/null || fail "a good release was refused"
out=$work/dist/aur/ely-office-bin
build=$out/PKGBUILD info=$out/.SRCINFO
file=ely-office-9.9.9-1-x86_64.pkg.tar.zst
sum=$(sha256sum "$work/release/v9.9.9/$file" | cut -d' ' -f1)
grep -qx "pkgver='9.9.9'" "$build" || fail "wrong pkgver"
grep -qx "license=('MIT' 'MPL-2.0')" "$build" || fail "licences not split"
grep -qx "depends=('glibc>=2.28' 'zlib')" "$build" || fail "depends not the package's"
grep -qx "conflicts=('ely-office' 'ely-doc' 'ely-mark')" "$build" || fail "conflicts miss a name"
# The real release, not the test's mirror.
grep -qF "$file::https://github.com/ELY-Open-Source/ElyOffice/releases/download/v9.9.9/$file" "$build" ||
  fail "source is not the release"
grep -qx "sha256sums=('$sum')" "$build" || fail "wrong hash"
grep -qx "	sha256sums = $sum" "$info" || fail ".SRCINFO lacks the hash"
grep -qx "pkgname = ely-office-bin" "$info" || fail ".SRCINFO lacks the name"
cmp -s "$out/ely-office-bin.install" "$work/pkg/.INSTALL" || fail "install script is not the package's"
bash -n "$build" || fail "PKGBUILD does not parse"
pack v9.9.8 ely-office 9.9.9-1
said=$(run v9.9.8 2>&1) && fail "a package of another version passed"
case $said in *"is 9.9.9-1, not 9.9.8-1"*) ;; *) fail "the version refusal says: $said" ;; esac
pack v9.9.7 ely-doc 9.9.7-1
said=$(run v9.9.7 2>&1) && fail "a single-app package passed"
case $said in *"not ely-office"*) ;; *) fail "the name refusal says: $said" ;; esac
run v9.9.9 --pull >/dev/null 2>&1 && fail "an unknown option passed"
echo "ok: AUR ely-office-bin"
