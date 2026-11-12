#!/usr/bin/env bash
# elfs.sh keeps library paths inside the package (compiled ELFs).
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
[ "$(uname)" = Linux ] || {
  echo "SKIP: Linux only"
  exit 0
}
for tool in cc readelf patchelf; do
  command -v "$tool" >/dev/null || {
    echo "SKIP: no $tool"
    exit 0
  }
done
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
fail() {
  echo "FAIL: $*" >&2
  exit 1
}
printf 'int leaf(void) { return 42; }\n' >"$work/leaf.c"
printf '#include <math.h>\nint leaf(void);\nint main(int c, char **v) { (void)v; return leaf() - 42 + (int)floor(c) - 1; }\n' >"$work/main.c"
cc -s -shared -fPIC -o "$work/libleaf.so" "$work/leaf.c"
cc -s -o "$work/app" "$work/main.c" -L"$work" -lleaf -lm
cc -o "$work/symbols" "$work/main.c" -L"$work" -lleaf -lm
# A fresh AppDir whose app has this library path.
check() { # runpath [binary]
  rm -rf "$work/AppDir" "$work/scratch"
  mkdir -p "$work/AppDir/usr/bin" "$work/AppDir/usr/lib" "$work/scratch"
  cp "$work/libleaf.so" "$work/AppDir/usr/lib/"
  cp "$work/${2:-app}" "$work/AppDir/usr/bin/app"
  patchelf --set-rpath "$1" "$work/AppDir/usr/bin/app"
  [ -z "${link:-}" ] || ln -s "$link" "$work/AppDir/usr/bin/escape"
  bash "$root/packaging/linux/elfs.sh" "$work/AppDir" "$work/scratch" >"$work/out" 2>&1
}
check '$ORIGIN/../lib' || fail "an in-package path is refused: $(cat "$work/out")"
grep -qx 'libm.so.6' "$work/out" || fail "libm is not reported needed: $(cat "$work/out")"
! grep -q 'libleaf' "$work/out" || fail "the bundled library is reported needed"
"$work/AppDir/usr/bin/app" || fail "the packaged app does not load its library"
refused() { # case, runpath, said text [binary]
  ! check "$2" "${4:-}" || fail "$1: elfs.sh passed"
  grep -qF "$3" "$work/out" || fail "$1: said $(cat "$work/out")"
}
refused 'trailing newline' $'$ORIGIN/../lib\n' 'newline in its library path'
refused 'inner newline' $'$ORIGIN\n/../lib' 'newline in its library path'
refused 'absolute path' '/usr/lib' 'not from $ORIGIN'
refused 'outside' '$ORIGIN/../../..' 'outside the package'
refused 'empty entry' '$ORIGIN/../lib:' 'searches the working directory'
refused 'symbols' '$ORIGIN/../lib' 'keeps symbols' symbols
# A link to a sibling whose name ends in newline.
mkdir -p "$work/AppDir"$'\n'
link=$'../../../AppDir\n' refused 'link out' '$ORIGIN/escape' 'outside the package'
refused 'loader token' '$ORIGIN/$LIB' 'a loader token'
refused 'braced token' '$ORIGIN/../${PLATFORM}' 'a loader token'
link=../lib check '$ORIGIN/escape' || fail "an in-package link is refused: $(cat "$work/out")"
echo "PASS: elfs rules"
