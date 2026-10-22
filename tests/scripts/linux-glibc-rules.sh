#!/usr/bin/env bash
# linux-glibc.sh refuses new glibc versions and ABI tags.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
for tool in cc readelf; do
  command -v "$tool" >/dev/null || {
    echo "SKIP: no $tool"
    exit 0
  }
done
[ "$(uname)" = Linux ] || {
  echo "SKIP: Linux only"
  exit 0
}
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
fail() {
  echo "FAIL: $*" >&2
  exit 1
}
# A tree holding only what linux-glibc.sh reads.
tree=$work/tree
mkdir -p "$tree/tests/scripts" "$tree/target/alma8/release" "$tree/engines/lo-src/instdir/program" "$tree/engines/git/bin"
cp "$root/tests/scripts/linux-glibc.sh" "$tree/tests/scripts/"
# No start files: only exit@GLIBC_2.2.5 is needed.
printf '#include <stdlib.h>\nstatic int x;\nint *p = &x;\nvoid _start(void) { exit(*p); }\n' >"$work/old.c"
cc -nostartfiles -fPIE -pie -o "$work/old" "$work/old.c"
cc -nostartfiles -fPIE -pie -Wl,-z,pack-relative-relocs -o "$work/relr" "$work/old.c"
# arc4random came in glibc 2.36.
printf '#include <stdlib.h>\nvoid _start(void) { exit((int)(arc4random() & 0)); }\n' >"$work/new.c"
cc -nostartfiles -fPIE -pie -o "$work/new" "$work/new.c"
for name in doc table show pdf mail tree mark lok-worker pdf-worker; do
  cp "$work/old" "$tree/target/alma8/release/ely-$name"
done
cp "$work/old" "$tree/engines/lo-src/instdir/program/libmergedlo.so"
cp "$work/old" "$tree/engines/git/bin/git"
expect() { # pass or fail, the said text, case
  if bash "$tree/tests/scripts/linux-glibc.sh" >"$work/out" 2>&1; then got=pass; else got=fail; fi
  [ "$got" = "$1" ] || fail "$3: linux-glibc.sh gave $got, want $1 ($(cat "$work/out"))"
  grep -qF "$2" "$work/out" || fail "$3: said $(cat "$work/out"), want $2"
}
expect pass "need GLIBC_2.31" "only GLIBC_2.2.5"
cp "$work/relr" "$tree/target/alma8/release/ely-mail"
expect fail "needs GLIBC_ABI_DT_RELR" "packed relocations"
cp "$work/new" "$tree/target/alma8/release/ely-mail"
expect fail "ely-mail needs GLIBC 2.36, above 2.31" "a symbol from glibc 2.36"
echo "linux-glibc-rules: new versions and ABI tags fail"
