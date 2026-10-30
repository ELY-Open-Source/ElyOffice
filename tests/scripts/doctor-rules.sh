#!/usr/bin/env bash
# doctor.sh finds fxc as gpui's build does: variable, PATH, SDK.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
fail() {
  echo "FAIL: $*" >&2
  exit 1
}
case $(uname -s) in
  MINGW* | MSYS* | CYGWIN*) ;;
  *)
    echo "SKIP: fxc is Windows only"
    exit 0
    ;;
esac
real=$(bash "$root/scripts/doctor.sh" | sed -n 's/^fxc  *//p')
[ -n "$real" ] || fail "doctor.sh finds no fxc on this machine"
where.exe fxc.exe >/dev/null 2>&1 && fail "fxc is on PATH; the cases need it off"
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
# A registry without the SDK's key.
printf '#!/bin/sh\nexit 1\n' >"$work/reg"
chmod +x "$work/reg"
said() { # extra PATH entry, then GPUI_FXC_PATH
  PATH="$work${1:+:$1}:$PATH" GPUI_FXC_PATH=$2 bash "$root/scripts/doctor.sh" 2>&1 | grep -E '^(MISSING )?fxc' || true
}
windows=$(cygpath -m "$real")
[[ $(said "" "$windows") == "fxc"*"$windows" ]] || fail "GPUI_FXC_PATH is not taken: $(said "" "$windows")"
[[ $(said "$(dirname "$real")" "") == "fxc"*fxc.exe ]] || fail "fxc on PATH is not found: $(said "$(dirname "$real")" "")"
[[ $(said "" "") == "MISSING fxc"* ]] || fail "no SDK key, no variable, no PATH: $(said "" "")"
[[ $(said "" "C:/no/fxc.exe") == "MISSING fxc"* ]] || fail "a missing GPUI_FXC_PATH counts: $(said "" "C:/no/fxc.exe")"
echo "PASS: doctor.sh finds fxc by variable, PATH or SDK, and says when none has it"
