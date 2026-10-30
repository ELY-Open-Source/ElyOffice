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
doctor() { # env assignments; sets out and code
  code=0
  out=$(env "$@" bash "$root/scripts/doctor.sh" 2>&1) || code=$?
}
line() { grep -E "$1" <<<"$out" || true; }

# A stand-in node of each version.
mkdir -p "$work/bin"
node() { printf '#!/bin/sh\necho v%s\n' "$1" >"$work/bin/node" && chmod +x "$work/bin/node"; }
node 22.18.0
doctor PATH="$work/bin:$PATH"
[[ $(line '^(OLD +)?node ') == "node       v22.18.0" ]] || fail "Node 22.18 fails: $(line '^(OLD +)?node ')"
# Exit codes count only where all else passes.
whole=$code
[ "$whole" = 0 ] || echo "SKIP: exit codes; doctor.sh fails here anyway: $(line '^(MISSING|OLD|BROKEN)' | tr '\n' ' ')"
node 18.19.1
doctor PATH="$work/bin:$PATH"
[[ $(line '^(OLD +)?node ') == "OLD     node 18.19.1, want 22.18 or newer"* ]] || fail "Node 18 passes: $(line '^(OLD +)?node ')"
[ "$whole" != 0 ] || [ "$code" != 0 ] || fail "Node 18 leaves doctor.sh exiting 0"
rm "$work/bin/node"

case $(uname -s) in
  MINGW* | MSYS* | CYGWIN*) ;;
  *)
    echo "PASS: doctor.sh fails a tool too old (fxc and Cygwin are Windows only)"
    exit 0
    ;;
esac
doctor PATH="$PATH"
real=$(line '^fxc ' | sed 's/^fxc  *//')
[ -n "$real" ] || fail "doctor.sh finds no fxc on this machine"
# No caller's fxc folder on any case's PATH.
bare=$(tr ':' '\n' <<<"$PATH" | while IFS= read -r dir; do [ -e "$dir/fxc.exe" ] || printf '%s:' "$dir"; done)
# A registry without the SDK's key.
printf '#!/bin/sh\nexit 1\n' >"$work/bin/reg"
chmod +x "$work/bin/reg"
fxc() { # extra PATH entry, GPUI_FXC_PATH, wanted line, wanted exit
  doctor PATH="$work/bin:${1:+$1:}${bare%:}" GPUI_FXC_PATH="$2"
  [[ $(line '^(MISSING )?fxc') == $3 ]] || fail "PATH ${1:-bare}, GPUI_FXC_PATH ${2:-unset}: $(line '^(MISSING )?fxc')"
  [ "$whole" != 0 ] || [ "$4" = any ] || { [ "$4" = 0 ] && [ "$code" = 0 ]; } || { [ "$4" = 1 ] && [ "$code" != 0 ]; } ||
    fail "PATH ${1:-bare}, GPUI_FXC_PATH ${2:-unset}: exit $code"
}
windows=$(cygpath -m "$real")
fxc "" "$windows" "fxc*$windows" 0
fxc "$(dirname "$real")" "" "fxc*fxc.exe" 0
fxc "" "" "MISSING fxc*" 1
fxc "" "C:/no/fxc.exe" "MISSING fxc*" 1

# Run from Cygwin too, the shell LibreOffice builds in.
cygwin=$(cygpath -u "${ELY_CYGWIN:-C:/cygwin64}")
if [ ! -x "$cygwin/bin/bash.exe" ]; then
  echo "PASS: doctor.sh fails a tool too old, finds fxc by variable, PATH or SDK (SKIP: no Cygwin)"
  exit 0
fi
said=$("$cygwin/bin/bash.exe" -lc "bash '$(cygpath -m "$root")/scripts/doctor.sh'" 2>&1 | grep -E '^(MISSING |OLD +)?(lo-make|autoconf|aclocal) ' || true)
[ "$(grep -c -E '^(lo-make|autoconf|aclocal) ' <<<"$said")" = 3 ] || fail "from Cygwin: $said"
echo "PASS: doctor.sh fails a tool too old, finds fxc by variable, PATH or SDK, and finds Cygwin's tools from Git Bash and Cygwin"
