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
# The kernel stays out of the app.
doc=$(ELY_DIST="$work" "$root/packaging/macos/bundle.sh" ely-doc debug | tail -n 1)
engine="$doc/Contents/Helpers/Ely Engine.app/Contents"
[ -x "$engine/MacOS/ely-lok-worker" ] || fail "ElyDoc has no LOK worker"
[ "$(/usr/libexec/PlistBuddy -c "Print :CFBundleIdentifier" "$engine/Info.plist")" = ely.zacharyzhang.office.doc.engine ] ||
  fail "the worker shares ElyDoc's bundle id"
[ "$(/usr/libexec/PlistBuddy -c "Print :LSUIElement" "$engine/Info.plist")" = true ] || fail "the worker is no UI element"
[ ! -e "$doc/Contents/Frameworks" ] || fail "ElyDoc carries a kernel"
# A kernel with a stale id gets its own written.
lo=$work/lo/LibreOffice.app/Contents
mkdir -p "$lo/Frameworks" "$lo/Resources/gallery" "$lo/Resources/extensions/dict-en"
for file in sofficerc LICENSE LICENSE.html NOTICE CREDITS.fodt; do echo kernel >"$lo/Resources/$file"; done
echo thesaurus >"$lo/Resources/extensions/dict-en/th_en_US_v2.dat"
echo '<oor:component-data/>' >"$lo/Resources/extensions/dict-en/dictionaries.xcu"
printf '%064d\n' 0 >"$lo/Resources/ely-kernel-id"
kernel=$(ELY_LO_APP="$work/lo/LibreOffice.app" "$root/packaging/macos/kernel.sh" "$work/Kernel" | tail -n 1)
[ ! -e "$kernel/Resources/gallery" ] || fail "the kernel kept the gallery"
"$root/scripts/kernel-id.sh" "$kernel" "$work/ely-kernel-id" >/dev/null
[ "$(cat "$kernel/Resources/ely-kernel-id")" = "$(cat "$work/ely-kernel-id")" ] ||
  fail "the kernel kept a stale id"
echo "bundle-macos: valid, signed, starts, no kernel inside; the kernel's own id"
