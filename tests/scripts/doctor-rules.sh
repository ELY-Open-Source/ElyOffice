#!/usr/bin/env bash
# doctor.sh: old tools fail; fxc found as gpui finds it.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
fail() {
  echo "FAIL: $*" >&2
  exit 1
}
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
said() { # line pattern; doctor.sh with $work/bin first on PATH
  PATH="$work/bin:$PATH" bash "$root/scripts/doctor.sh" 2>&1 | grep -E "$1" || true
}

# A stand-in node of each version.
mkdir -p "$work/bin"
node() { printf '#!/bin/sh\necho v%s\n' "$1" >"$work/bin/node" && chmod +x "$work/bin/node"; }
node 18.19.1
[[ $(said '^(OLD +)?node ') == "OLD     node 18.19.1, want 22.12 or newer"* ]] || fail "Node 18 passes: $(said '^(OLD +)?node ')"
node 22.12.0
[[ $(said '^(OLD +)?node ') == "node       v22.12.0" ]] || fail "Node 22.12 fails: $(said '^(OLD +)?node ')"
rm "$work/bin/node"

case $(uname -s) in
  MINGW* | MSYS* | CYGWIN*) ;;
  *)
    echo "PASS: doctor.sh fails a tool too old (fxc is Windows only)"
    exit 0
    ;;
esac
real=$(bash "$root/scripts/doctor.sh" | sed -n 's/^fxc  *//p')
[ -n "$real" ] || fail "doctor.sh finds no fxc on this machine"
# No caller's fxc folder on any case's PATH.
bare=$(tr ':' '\n' <<<"$PATH" | while IFS= read -r dir; do [ -e "$dir/fxc.exe" ] || printf '%s:' "$dir"; done)
# A registry without the SDK's key.
printf '#!/bin/sh\nexit 1\n' >"$work/bin/reg"
chmod +x "$work/bin/reg"
fxc() { # extra PATH entry, then GPUI_FXC_PATH
  PATH="$work/bin:${1:+$1:}${bare%:}" GPUI_FXC_PATH=$2 bash "$root/scripts/doctor.sh" 2>&1 | grep -E '^(MISSING )?fxc' || true
}
windows=$(cygpath -m "$real")
[[ $(fxc "" "$windows") == "fxc"*"$windows" ]] || fail "GPUI_FXC_PATH is not taken: $(fxc "" "$windows")"
[[ $(fxc "$(dirname "$real")" "") == "fxc"*fxc.exe ]] || fail "fxc on PATH is not found: $(fxc "$(dirname "$real")" "")"
[[ $(fxc "" "") == "MISSING fxc"* ]] || fail "no SDK key, no variable, no PATH: $(fxc "" "")"
[[ $(fxc "" "C:/no/fxc.exe") == "MISSING fxc"* ]] || fail "a missing GPUI_FXC_PATH counts: $(fxc "" "C:/no/fxc.exe")"
echo "PASS: doctor.sh fails a tool too old, finds fxc by variable, PATH or SDK, and says when none has it"
