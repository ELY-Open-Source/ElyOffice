#!/usr/bin/env bash
# Every tracked text file is LF; vendor keeps upstream bytes.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
bad=$(git -C "$root" ls-files --eol | awk '($1 == "i/crlf" || $1 == "i/mixed") && $NF !~ /^vendor\// {print $NF}')
if [ -n "$bad" ]; then
  echo "FAIL: CRLF in the index:"
  echo "$bad"
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
