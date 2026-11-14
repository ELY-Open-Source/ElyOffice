#!/usr/bin/env bash
# Prints what an install owns: files.sh <stage>
set -euo pipefail

stage=${1:?staged folder from packaging/stage.sh}
cd "$stage"
[ -z "$(find . -type l)" ] || {
  echo "files.sh: links in $stage" >&2
  exit 1
}
# Folders first, parents before children; then files.
{
  find . -mindepth 1 -type d | sort | sed 's|$|/|'
  find . -type f | sort
} | sed 's|^\./||' | while IFS= read -r path; do
  case $path in *\\* | *[[:cntrl:]]*)
    echo "files.sh: unsafe name $path" >&2
    exit 1
    ;;
  esac
  printf '%s\r\n' "${path//\//\\}"
done
