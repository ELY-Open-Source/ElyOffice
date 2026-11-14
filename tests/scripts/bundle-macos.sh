#!/usr/bin/env bash
# The macOS bundle is valid, signed, and starts.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
fail() {
  echo "FAIL: $*" >&2
  exit 1
}

[ "$(uname)" = Darwin ] || {
  echo "SKIP bundle-macos: macOS only"
  exit 0
}
version=$(awk -F'"' '/^version = / { print $2; exit }' "$root/Cargo.toml")
app=$(ELY_DIST="$work" "$root/packaging/macos/bundle.sh" ely-pdf debug | tail -n 1)
[ "$app" = "$work/ElyPDF.app" ] || fail "bundle printed $app"
plutil -lint "$app/Contents/Info.plist" >/dev/null || fail "Info.plist is not valid"
plist() { /usr/libexec/PlistBuddy -c "Print :$1" "$app/Contents/Info.plist"; }
[ "$(plist CFBundleIdentifier)" = ely.zacharyzhang.office.pdf ] || fail "bundle id: $(plist CFBundleIdentifier)"
[ "$(plist CFBundleName)" = ElyPDF ] || fail "bundle name: $(plist CFBundleName)"
[ "$(plist CFBundleShortVersionString)" = "$version" ] || fail "version: $(plist CFBundleShortVersionString)"
codesign --verify --deep --strict "$app" || fail "signature does not verify"
out=$(ELY_SMOKE=1 HOME="$work" perl -e 'alarm 60; exec @ARGV' "$app/Contents/MacOS/ely-pdf" 2>/dev/null) ||
  fail "the bundled binary did not exit cleanly"
[ "$out" = "ready ElyPDF" ] || fail "bundled binary printed '$out'"
if ELY_DIST="$work" "$root/packaging/macos/bundle.sh" ely-nope debug >/dev/null 2>&1; then
  fail "an unknown app built"
fi
# A kernel with a stale id gets its own written.
kernel=$work/kernel/LibreOffice.app/Contents
mkdir -p "$kernel/Frameworks" "$kernel/Resources"
for file in sofficerc LICENSE LICENSE.html NOTICE CREDITS.fodt; do echo kernel >"$kernel/Resources/$file"; done
printf '%064d\n' 0 >"$kernel/Resources/ely-kernel-id"
doc=$(ELY_DIST="$work" ELY_LO_APP="$work/kernel/LibreOffice.app" "$root/packaging/macos/bundle.sh" ely-doc debug | tail -n 1)
"$root/scripts/kernel-id.sh" "$kernel" "$work/ely-kernel-id" >/dev/null
[ "$(cat "$doc/Contents/Resources/ely-kernel-id")" = "$(cat "$work/ely-kernel-id")" ] ||
  fail "the bundled kernel kept a stale id"
echo "bundle-macos: valid, signed, starts, its kernel's own id"
