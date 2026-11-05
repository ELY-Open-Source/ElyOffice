#!/usr/bin/env bash
# macOS releases refuse bash 3.2; doctor.sh names both bashes.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
[ "$(uname -s)" = Darwin ] || {
  echo "SKIP: macOS only"
  exit 0
}
fail() {
  echo "FAIL: $*" >&2
  exit 1
}
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

# ci.sh stops under 3.2 before reading its arguments.
out=$(/bin/bash "$root/scripts/ci.sh" macos v0.0.0 2>&1) && fail "ci.sh ran under /bin/bash"
grep -q "macOS needs bash 5" <<<"$out" || fail "ci.sh under /bin/bash: $out"
# Under bash 5 it goes on, to the tag check.
out=$("$BASH" "$root/scripts/ci.sh" macos v0.0.0 2>&1) && fail "ci.sh took tag v0.0.0"
grep -q "tag v0.0.0 is not v" <<<"$out" || fail "ci.sh under bash $BASH_VERSION: $out"

doctor() { # env assignments; prints doctor.sh's bash lines
  env "$@" "$BASH" "$root/scripts/doctor.sh" 2>&1 | grep -E '^(OLD +|MISSING +)?bash' || true
}
mkdir -p "$work/bin" "$work/home/ely-share/bash-x86_64/bin"
stand() { printf '#!/bin/sh\necho "GNU bash, version %s(1)-release"\n' "$1" >"$work/bin/bash" && chmod +x "$work/bin/bash"; }
stand 3.2.57
grep -qx "OLD     bash 3.2.57, want 5.0 or newer (README, Build)" <<<"$(doctor PATH="$work/bin:$PATH")" ||
  fail "bash 3.2 passes doctor.sh: $(doctor PATH="$work/bin:$PATH")"
stand 5.3.0
grep -q "^bash  *GNU bash, version 5.3.0" <<<"$(doctor PATH="$work/bin:$PATH")" ||
  fail "bash 5.3 fails doctor.sh: $(doctor PATH="$work/bin:$PATH")"
# Rosetta cannot run an arm64-only bash.
arm=$(command -v /opt/homebrew/bin/bash) || fail "no Homebrew bash (brew install bash)"
lipo -archs "$(readlink -f "$arm")" | grep -qx arm64 || fail "$arm is not arm64 only"
ln -s "$arm" "$work/home/ely-share/bash-x86_64/bin/bash"
grep -q "^MISSING bash-x86_64" <<<"$(doctor HOME="$work/home")" ||
  fail "an arm64 bash passes as the Intel one: $(doctor HOME="$work/home")"
x86=$HOME/ely-share/bash-x86_64/bin/bash
[ -x "$x86" ] || fail "no $x86 (scripts/build-bash.sh)"
grep -q "^bash-x86_64 GNU bash, version 5" <<<"$(doctor)" || fail "the Intel bash fails doctor.sh: $(doctor)"
echo "PASS: ci.sh refuses bash 3.2; doctor.sh takes bash 5 and an x86_64 bash 5 only"
