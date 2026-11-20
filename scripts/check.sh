#!/usr/bin/env bash
# The project's rules: length, guide, comments, banners, line endings.
set -euo pipefail

root=${1:-$(cd "$(dirname "$0")/.." && pwd)}
here=$(cd "$(dirname "$0")/.." && pwd)
# Rules read tracked files: anywhere else, they check none.
if ! prefix=$(git -C "$root" rev-parse --show-prefix 2>/dev/null); then
  echo "FAIL: $root is not in a git checkout; every rule reads tracked files"
  exit 1
elif [ -n "$prefix" ]; then
  echo "FAIL: $root is not the top of its git checkout but $prefix in it; every rule reads tracked files"
  exit 1
elif [ -z "$(git -C "$root" ls-files | head -n 1)" ]; then
  echo "FAIL: $root tracks no files; every rule reads tracked files"
  exit 1
fi
list=$(mktemp)
trap 'rm -f "$list"' EXIT
git -C "$root" ls-files -z -- '*.rs' '*.c' '*.h' '*.sh' '*.py' '*.ts' '*.tsx' '*.js' '*.mjs' '*.css' \
  '*.swift' '*.nsi' '*.ps1' '*.html' ':(exclude,glob)**/vendor/**' ':!reference' ':!patches' >"$list"
failed=0
# awk counts an unterminated last line; wc -l does not.
long=$(cd "$root" && xargs -0 awk 'FNR == 1 && NR > 1 && n > 500 {print name ": " n " lines"}
  {name = FILENAME; n = FNR} END {if (n > 500) print name ": " n " lines"}' <"$list")
if [ -n "$long" ]; then
  echo "FAIL: files over 500 lines:"
  echo "$long"
  failed=1
fi
# Every session reads the whole agent guide.
guide=$root/agents.md
if [ -f "$guide" ] && [ "$(wc -c <"$guide")" -gt 30000 ]; then
  echo "FAIL: agents.md is $(wc -c <"$guide") bytes, over 30000: move detail into docs"
  failed=1
fi
python3 "$here/tests/scripts/comments.py" "$root" || failed=1
python3 "$here/tests/scripts/waits.py" "$root" || failed=1
python3 "$here/tests/scripts/banners.py" "$root" || failed=1
bash "$here/tests/scripts/eol.sh" "$root" || failed=1
[ "$failed" = 0 ] || exit 1
echo "check: every rule holds"
