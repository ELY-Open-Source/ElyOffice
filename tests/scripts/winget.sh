#!/usr/bin/env bash
# winget.sh writes one manifest from a release; refuses wrong ones.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
fail() {
  echo "FAIL: $*" >&2
  exit 1
}
release=$work/release/v9.9.9
mkdir -p "$release"
printf '{"version": "9.9.9", "assets": {"windows-x86_64": {"url": "https://x.test/setup.exe", "sha256": "%064d"}}}\n' 7 >"$release/ely-office-windows-x86_64.json"
url="file://$(cd "$work/release" && pwd -W 2>/dev/null || pwd)"
run() { ELY_WINGET_DOWNLOAD=$url ELY_DIST=$work/dist bash "$root/packaging/winget.sh" "$1"; }
run v9.9.9 >/dev/null || fail "a good release was refused"
at=$work/dist/winget/manifests/z/ZacharyZhang/ElyOffice/9.9.9
[ "$(ls "$work/dist/winget/manifests/z/ZacharyZhang")" = ElyOffice ] || fail "more than one package"
installer=$at/ZacharyZhang.ElyOffice.installer.yaml locale=$at/ZacharyZhang.ElyOffice.locale.en-US.yaml
grep -qx "  InstallerUrl: https://x.test/setup.exe" "$installer" || fail "wrong installer URL"
grep -qx "  InstallerSha256: $(printf '%064d' 7)" "$installer" || fail "wrong hash"
# The uninstall key the setup writes.
grep -qx "ProductCode: Ely Office" "$installer" || fail "product code is not the uninstall key"
grep -qF '"${UNINSTALL}\Ely Office" "UninstallString"' "$root/packaging/windows/installer.nsi" ||
  fail "installer.nsi's uninstall key moved"
for license in $(awk -F'\t' '$4 == 1 { print $5 }' "$root/packaging/apps.tsv" | sed 's/ AND /\n/g' | sort -u); do
  grep -q "^License: .*$license" "$locale" || fail "the licence lacks $license"
done
if command -v winget >/dev/null; then
  winget validate --manifest "$(cygpath -w "$at")" >/dev/null || fail "winget rejects $at"
else
  echo "SKIP: no winget; manifest not validated"
fi
cp -r "$release" "$work/release/v9.9.8"
said=$(run v9.9.8 2>&1) && fail "a manifest of another version passed"
case $said in *"is 9.9.9, not v9.9.8"*) ;; *) fail "the version refusal says: $said" ;; esac
# 0.1.16's per-app manifests do not count.
mv "$release/ely-office-windows-x86_64.json" "$release/ely-doc-windows-x86_64.json"
run v9.9.9 >/dev/null 2>&1 && fail "a missing suite manifest passed"
echo "ok: winget manifest"
