#!/usr/bin/env bash
# Every tracked text file is LF; vendor keeps upstream bytes.
set -euo pipefail

root=${1:-$(cd "$(dirname "$0")/../.." && pwd)}
list=$(mktemp)
trap 'rm -f "$list"' EXIT
# A failed listing stops here; NUL needs a file.
git -C "$root" ls-files --eol -z >"$list"
bad=
# Records end in NUL; the path follows the first tab.
while IFS= read -r -d '' record; do
  info=${record%%$'\t'*}
  path=${record#*$'\t'}
  case ${info%% *} in
    i/crlf | i/mixed) [[ $path == vendor/* ]] || bad+="$path"$'\n' ;;
  esac
done <"$list"
if [ -n "$bad" ]; then
  echo "FAIL: CRLF in the index:"
  printf '%s' "$bad"
  exit 1
fi
probe=$(git -C "$root" check-attr text eol -- probe.rs | tr '\n' ' ')
case $probe in
  *"text: auto"*"eol: lf"*) ;;
  *)
    echo "FAIL: new files are not normalized to LF: $probe"
    exit 1
    ;;
esac
echo "PASS: tracked text is LF; new files will be too"
