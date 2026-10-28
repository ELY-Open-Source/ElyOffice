#!/usr/bin/env bash
# eol.sh reads whole paths, spaces included.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
git init -q "$work"
cp "$root/.gitattributes" "$work/"
git -C "$work" add .gitattributes
# Puts CRLF bytes in the index, past the LF filter.
crlf() {
  mkdir -p "$(dirname "$work/$1")"
  printf 'one\r\ntwo\r\n' >"$work/$1"
  blob=$(git -C "$work" hash-object -w --no-filters -- "$1")
  git -C "$work" update-index --add --cacheinfo "100644,$blob,$1"
}
crlf "vendor/mail/.My Folder/new/probe.eml"
if ! said=$(bash "$root/tests/scripts/eol.sh" "$work"); then
  echo "FAIL: a vendor path with a space was refused: $said"
  exit 1
fi
crlf "notes vendor/probe.md"
if said=$(bash "$root/tests/scripts/eol.sh" "$work"); then
  echo "FAIL: CRLF outside vendor passed: $said"
  exit 1
fi
case $said in
  *"notes vendor/probe.md"*) ;;
  *)
    echo "FAIL: the bad path was not named whole: $said"
    exit 1
    ;;
esac
git -C "$work" rm -q --cached -- "notes vendor/probe.md"
# A git whose listing fails; the rest is real.
mkdir "$work/bin"
printf '#!/usr/bin/env bash\nfor arg; do [ "$arg" != ls-files ] || exit 128; done\nexec "%s" "$@"\n' \
  "$(command -v git)" >"$work/bin/git"
chmod +x "$work/bin/git"
if said=$(PATH="$work/bin:$PATH" bash "$root/tests/scripts/eol.sh" "$work" 2>&1); then
  echo "FAIL: a failed listing passed: $said"
  exit 1
fi
echo "PASS: eol.sh keeps vendor's bytes, names whole paths, stops on git's failure"
