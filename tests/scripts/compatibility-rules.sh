#!/usr/bin/env bash
# compatibility.py --check fails when the list names another build.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
fail() {
  echo "FAIL: $*" >&2
  exit 1
}
# Only what the script reads.
mkdir -p "$work/scripts" "$work/docs" "$work/packaging" "$work/patches"
cp "$root/Cargo.toml" "$work/"
cp "$root/docs/compatibility.md" "$work/docs/"
cp "$root/packaging/apps.tsv" "$work/packaging/"
cp "$root"/scripts/{compatibility.py,fetch-references.sh,fetch-git-lfs.sh,vendor-gpui.sh} "$work/scripts/"
for folder in libreoffice mupdf gpui; do
  mkdir "$work/patches/$folder"
  for patch in "$root/patches/$folder"/*.patch; do : >"$work/patches/$folder/${patch##*/}"; done
done
check() { python3 "$work/scripts/compatibility.py" --check >"$work/out.log" 2>&1; }

check || fail "the committed list is not this build's: $(cat "$work/out.log")"
# A version bump without the script.
sed -i.bak 's/^version = "\(.*\)"/version = "9.9.9"/' "$work/Cargo.toml"
! check || fail "a list for another version passed"
grep -q "Ely Office 9.9.9" "$work/out.log" || fail "the wanted version is not named"
python3 "$work/scripts/compatibility.py" >/dev/null
check || fail "the rewritten list does not pass"
# A new patch without the script.
: >"$work/patches/libreoffice/9999-new.patch"
! check || fail "a list missing a patch passed"
grep -q "–9999" "$work/out.log" || fail "the new patch range is not named"
# A pin nobody named.
sed -i.bak 's/^fetch mupdf \(\S*\) [0-9a-f]*/fetch mupdf \1 0123456789012345678901234567890123456789/' "$work/scripts/fetch-references.sh"
! check || fail "an unnamed pin passed"
grep -q "name its release" "$work/out.log" || fail "the unnamed pin is not reported"
echo "compatibility-rules: the list fails the check for another version, patch range or pin"
