#!/usr/bin/env bash
# rpath.sh fails on loads outside the app and unread binaries.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
fail() {
  echo "FAIL: $*" >&2
  exit 1
}
# A stand-in otool: replies sit beside the binary.
mkdir -p "$work/bin"
cat >"$work/bin/otool" <<'STUB'
#!/usr/bin/env bash
case $1 in
  -hv) reply=$2.header ;;
  -l) reply=$2.commands ;;
  -L) reply=$2.libraries ;;
esac
[ -f "$reply" ] || exit 1
cat "$reply"
STUB
chmod +x "$work/bin/otool"
app=$work/Ely.app
binary=$app/Contents/MacOS/ely
reset() { # header word, rpath
  rm -rf "$app" "$work/outside"
  mkdir -p "$app/Contents/MacOS" "$app/Contents/Frameworks" "$work/outside"
  : >"$binary"
  echo "MH_MAGIC_64 X86_64 ALL 0x00 $1 19 1928 NOUNDEFS DYLDLINK TWOLEVEL PIE" >"$binary.header"
  printf 'Load command 12\n          cmd LC_RPATH\n      cmdsize 48\n         path %s (offset 12)\n' "$2" >"$binary.commands"
  printf '%s:\n\t@rpath/libswiftCoreMedia.dylib (compatibility version 1.0.0)\n\t/usr/lib/libSystem.B.dylib (compatibility version 1.0.0)\n' "$binary" >"$binary.libraries"
}
expect() { # pass or fail, case
  if PATH=$work/bin:$PATH "$root/tests/scripts/rpath.sh" "$app" "$binary" 2>"$work/err"; then got=pass; else got=fail; fi
  [ "$got" = "$1" ] || fail "$2: rpath.sh gave $got, want $1 ($(cat "$work/err"))"
}

reset EXECUTE @executable_path/../Frameworks
expect fail "a load no path finds"
touch "$app/Contents/Frameworks/libswiftCoreMedia.dylib"
expect pass "a load in Frameworks"
reset EXECUTE "$work/outside"
touch "$work/outside/libswiftCoreMedia.dylib"
expect fail "a load found outside the app"
reset EXECUTE @executable_path/../../../outside
touch "$work/outside/libswiftCoreMedia.dylib"
expect fail "a path that climbs out of the app"
reset EXECUTE @executable_path/../Frameworks
touch "$work/outside/libswiftCoreMedia.dylib"
ln -s "$work/outside/libswiftCoreMedia.dylib" "$app/Contents/Frameworks/libswiftCoreMedia.dylib"
# Git Bash copies instead of linking.
if [ -L "$app/Contents/Frameworks/libswiftCoreMedia.dylib" ]; then
  expect fail "a link out of the app"
else
  echo "SKIP: no symbolic links here"
fi
reset DYLIB @executable_path/../Frameworks
expect pass "a library uses its executable's paths"
# No @rpath load: only the unread reply can fail.
for reply in header commands libraries; do
  reset EXECUTE @executable_path/../Frameworks
  printf '%s:\n\t/usr/lib/libSystem.B.dylib (compatibility version 1.0.0)\n' "$binary" >"$binary.libraries"
  expect pass "an executable without @rpath loads"
  rm "$binary.$reply"
  expect fail "otool fails on the $reply"
  grep -q "otool cannot read the .*$reply of" "$work/err" || fail "otool fails on the $reply: said $(cat "$work/err")"
done
echo "rpath-rules: unresolved loads and unread binaries fail"
