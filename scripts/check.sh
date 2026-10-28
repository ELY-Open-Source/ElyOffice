#!/usr/bin/env bash
# The project's rules: file length, comments, line endings.
set -euo pipefail

root=${1:-$(cd "$(dirname "$0")/.." && pwd)}
here=$(cd "$(dirname "$0")/.." && pwd)
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
python3 "$here/tests/scripts/comments.py" "$root" || failed=1
bash "$here/tests/scripts/eol.sh" "$root" || failed=1
[ "$failed" = 0 ] || exit 1
echo "check: every rule holds"
