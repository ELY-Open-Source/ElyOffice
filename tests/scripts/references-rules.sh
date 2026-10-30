#!/usr/bin/env bash
# fetch-references.sh: unknown names fail; names fetch only those.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
fail() {
  echo "FAIL: $*" >&2
  exit 1
}
quiet() { git -C "$1" config user.name ely && git -C "$1" config user.email ely@example.com && git -C "$1" config commit.gpgsign false; }

# A local upstream for omarchy, with its sparse folders.
up=$work/up
git init -q "$up" && quiet "$up"
mkdir -p "$up/bin" "$up/themes" "$up/default/themed"
touch "$up/bin/a" "$up/themes/a" "$up/default/themed/a"
git -C "$up" add -A && git -C "$up" commit -qm up
rev=$(git -C "$up" rev-parse HEAD)
mkdir -p "$work/root/scripts"
sed -E "s|^fetch omarchy [^ ]+ [0-9a-f]+ |fetch omarchy file://$up $rev |" "$root/scripts/fetch-references.sh" >"$work/root/scripts/fetch-references.sh"
grep -q "file://$up $rev" "$work/root/scripts/fetch-references.sh" || fail "the copy kept omarchy's real source"
script=$work/root/scripts/fetch-references.sh

# A MuPDF checkout whose submodule's source is gone.
mupdf=$work/root/reference/mupdf
git init -q "$mupdf" && quiet "$mupdf"
printf '[submodule "gone"]\n\tpath = gone\n\turl = file://%s/nowhere\n' "$work" >"$mupdf/.gitmodules"
git -C "$mupdf" add .gitmodules
git -C "$mupdf" update-index --add --cacheinfo "160000,$rev,gone"
git -C "$mupdf" commit -qm mupdf
before=$(git -C "$mupdf" status --porcelain --ignore-submodules=none)

said=$(bash "$script" omarchy 2>&1) || fail "omarchy alone fails: $said"
[ "$(git -C "$work/root/reference/omarchy" rev-parse HEAD)" = "$rev" ] || fail "omarchy is not at its pin"
[ ! -e "$mupdf/gone/.git" ] && [ "$(git -C "$mupdf" status --porcelain --ignore-submodules=none)" = "$before" ] ||
  fail "fetching omarchy touched mupdf"
! git -C "$mupdf" submodule update --init --depth 1 --recursive >/dev/null 2>&1 || fail "the gone submodule updates, so the case proves nothing"

for names in "mupdf-typo" "omarchy mupdf-typo" "mupdf-typo omarchy"; do
  rm -rf "$work/root/reference/omarchy"
  # shellcheck disable=SC2086
  if said=$(bash "$script" $names 2>&1); then fail "$names passed: $said"; fi
  [[ $said == *"no upstream named mupdf-typo"* ]] || fail "$names: $said"
  [ ! -e "$work/root/reference/omarchy" ] || fail "$names fetched omarchy before failing"
done
echo "PASS: unknown names fail before any fetch; a name fetches only that upstream"
