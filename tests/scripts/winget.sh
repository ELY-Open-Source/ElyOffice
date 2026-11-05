#!/usr/bin/env bash
# winget.sh writes valid manifests from a release; refuses wrong ones.
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
apps=$(awk -F'\t' '$4 == 1 { print $1 "\t" $2 }' "$root/packaging/apps.tsv")
while IFS=$'\t' read -r slug name; do
  printf '{"version": "9.9.9", "assets": {"windows-x86_64": {"url": "https://x.test/%s.exe", "sha256": "%064d"}}}\n' "$name" 7 >"$release/$slug-windows-x86_64.json"
done <<<"$apps"
url="file://$(cd "$work/release" && pwd -W 2>/dev/null || pwd)"
run() { ELY_WINGET_DOWNLOAD=$url ELY_DIST=$work/dist bash "$root/packaging/winget.sh" "$1"; }
run v9.9.9 >/dev/null || fail "a good release was refused"
while IFS=$'\t' read -r slug name; do
  at=$work/dist/winget/manifests/z/ZacharyZhang/$name/9.9.9
  row=$(awk -F'\t' -v slug="$slug" '$1 == slug' "$root/packaging/apps.tsv")
  grep -qx "  InstallerUrl: https://x.test/$name.exe" "$at/ZacharyZhang.$name.installer.yaml" || fail "$name: wrong installer URL"
  grep -qx "  InstallerSha256: $(printf '%064d' 7)" "$at/ZacharyZhang.$name.installer.yaml" || fail "$name: wrong hash"
  grep -qx "ProductCode: Ely Office $name" "$at/ZacharyZhang.$name.installer.yaml" || fail "$name: product code is not the uninstall key"
  grep -qx "License: $(cut -f5 <<<"$row")" "$at/ZacharyZhang.$name.locale.en-US.yaml" || fail "$name: licence differs from apps.tsv"
  grep -qx "ShortDescription: $(cut -f6 <<<"$row")" "$at/ZacharyZhang.$name.locale.en-US.yaml" || fail "$name: summary differs from apps.tsv"
done <<<"$apps"
# The uninstall key the installer writes.
grep -q '^!define UNINSTALL_KEY "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\Ely Office ${APP_NAME}"' \
  "$root/packaging/windows/installer.nsi" || fail "installer.nsi's uninstall key moved"
if command -v winget >/dev/null; then
  for at in "$work"/dist/winget/manifests/z/ZacharyZhang/*/9.9.9; do
    winget validate --manifest "$(cygpath -w "$at")" >/dev/null || fail "winget rejects $at"
  done
else
  echo "SKIP: no winget; manifests not validated"
fi
cp -r "$release" "$work/release/v9.9.8"
said=$(run v9.9.8 2>&1) && fail "a manifest of another version passed"
case $said in *"is 9.9.9, not v9.9.8"*) ;; *) fail "the version refusal says: $said" ;; esac
rm "$release/ely-mail-windows-x86_64.json"
run v9.9.9 >/dev/null 2>&1 && fail "a missing manifest passed"
echo "ok: winget manifests"
