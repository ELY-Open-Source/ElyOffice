#!/usr/bin/env bash
# A LibreOffice tree configured at another path is refused.
set -euo pipefail
root=$(cd "$(dirname "$0")/../.." && pwd)
check=$root/scripts/lo-configured.sh
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
src=$work/lo-src
mkdir -p "$src"
real=$(cd "$src" && pwd -P)
case $(uname) in
  CYGWIN*) real=$(cygpath -m "$real") ;;
esac
"$check" "$src" || { echo "FAIL: an unconfigured tree was refused"; exit 1; }
printf 'export SRCDIR=%s\n' "$real" >"$src/config_host.mk"
"$check" "$src" || { echo "FAIL: a tree configured in place was refused"; exit 1; }
ln -s "$src" "$work/link"
"$check" "$work/link" || { echo "FAIL: a link to the tree was refused"; exit 1; }
printf 'export SRCDIR=C:/ely/ElyOffice/engines/lo-src\n' >"$src/config_host.mk"
if out=$("$check" "$src" 2>&1); then
  echo "FAIL: a moved tree was accepted"
  exit 1
fi
grep -q "configured at 'C:/ely/ElyOffice/engines/lo-src', not '$real'" <<<"$out" || { echo "FAIL: the refusal names no paths: $out"; exit 1; }
echo "PASS: a tree configured elsewhere is refused, naming both paths"
