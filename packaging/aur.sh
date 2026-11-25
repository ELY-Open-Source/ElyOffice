#!/usr/bin/env bash
# aur.sh <tag> [--push]: ely-office-bin from that release's Arch package.
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
tag=${1:?the release tag}
push=
case ${2:-} in
  --push) push=1 ;;
  '') ;;
  *)
    echo "aur.sh: unknown option ${2}" >&2
    exit 1
    ;;
esac
command -v zstd >/dev/null || {
  echo "aur.sh: no zstd" >&2
  exit 1
}
release=https://github.com/ELY-Open-Source/ElyOffice/releases/download/$tag
# ELY_AUR_DOWNLOAD replaces the release URL; tests use it.
download=${ELY_AUR_DOWNLOAD:-$release}
version=${tag#v}
file=ely-office-$version-1-x86_64.pkg.tar.zst
out=${ELY_DIST:-$root/dist}/aur/ely-office-bin
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
curl -fsSL -o "$work/$file" "$download/$file"
zstd -dcq "$work/$file" | tar -xf - -C "$work" .PKGINFO .INSTALL
rm -rf "$out"
mkdir -p "$out"
cp "$work/.INSTALL" "$out/ely-office-bin.install"
python3 - "$version" "$file" "$release/$file" "$work/$file" "$work/.PKGINFO" "$out" <<'EOF'
import hashlib, os, sys
version, file, url, package, info, out = sys.argv[1:]
fields = {}
for line in open(info, encoding="utf-8"):
    if " = " in line:
        key, value = line.rstrip("\n").split(" = ", 1)
        fields.setdefault(key, []).append(value)
if fields["pkgname"] != ["ely-office"]:
    sys.exit(f"aur.sh: {file} holds {fields['pkgname']}, not ely-office")
if fields["pkgver"] != [version + "-1"]:
    sys.exit(f"aur.sh: {file} is {fields['pkgver'][0]}, not {version}-1")
digest = hashlib.sha256(open(package, "rb").read()).hexdigest()
licenses = fields["license"][0].split(" AND ")
names = ["ely-office"] + fields["provides"]
values = [
    ("pkgdesc", [fields["pkgdesc"][0]]),
    ("pkgver", [version]),
    ("pkgrel", ["1"]),
    ("url", [fields["url"][0]]),
    ("install", ["ely-office-bin.install"]),
    ("arch", ["x86_64"]),
    ("license", licenses),
    ("depends", fields["depend"]),
    ("provides", names),
    ("conflicts", names),
    ("noextract", [file]),
    ("options", ["!strip", "!debug"]),
    ("source", [f"{file}::{url}"]),
    ("sha256sums", [digest]),
]
quoted = [item for _, items in values for item in items if "'" in item]
if quoted:
    sys.exit(f"aur.sh: a quote in {quoted[0]!r} breaks the PKGBUILD")
plain = {"pkgver", "pkgrel", "url", "install", "pkgdesc"}
build = ["# Maintainer: ZacharyZhang", "# Written by packaging/aur.sh; edit that.", "pkgname=ely-office-bin"]
for key, items in values:
    if key in plain:
        build.append(f"{key}='{items[0]}'")
    else:
        build.append(f"{key}=(" + " ".join(f"'{item}'" for item in items) + ")")
build += [
    "",
    "package() {",
    f"  bsdtar -xf \"$srcdir/{file}\" -C \"$pkgdir\" \\",
    "    --exclude .PKGINFO --exclude .MTREE --exclude .BUILDINFO --exclude .INSTALL",
    "}",
]
srcinfo = ["pkgbase = ely-office-bin"]
srcinfo += [f"\t{key} = {item}" for key, items in values for item in items]
srcinfo += ["", "pkgname = ely-office-bin"]
for name, lines in (("PKGBUILD", build), (".SRCINFO", srcinfo)):
    with open(os.path.join(out, name), "w", encoding="utf-8", newline="\n") as text:
        text.write("\n".join(lines) + "\n")
EOF
echo "$out"
[ -n "$push" ] || exit 0

# AUR makes the repository on the first push.
clone=$work/aur
git clone -q ssh://aur@aur.archlinux.org/ely-office-bin.git "$clone"
cp "$out/PKGBUILD" "$out/.SRCINFO" "$out/ely-office-bin.install" "$clone/"
git -C "$clone" add PKGBUILD .SRCINFO ely-office-bin.install
if git -C "$clone" diff --cached --quiet; then
  echo "aur.sh: the AUR already has $version"
  exit 0
fi
git -C "$clone" commit -q -m "ely-office-bin $version"
git -C "$clone" push -q origin HEAD:master
echo "aur.sh: pushed ely-office-bin $version"
