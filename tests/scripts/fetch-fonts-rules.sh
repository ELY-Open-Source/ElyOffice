#!/usr/bin/env bash
# fetch-fonts.sh: fetch, unzip a member, check, skip what is there.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
fail() {
  echo "FAIL: $*" >&2
  exit 1
}
url() { # A file URL curl reads on every host.
  if command -v cygpath >/dev/null; then echo "file:///$(cygpath -m "$1")"; else echo "file://$1"; fi
}
sum() { shasum -a 256 "$1" | cut -d ' ' -f 1; }

mkdir -p "$work/tree/scripts" "$work/tree/packaging/libreoffice/fonts" "$work/up"
cp "$root/scripts/fetch-fonts.sh" "$work/tree/scripts/"
fetch() { "$work/tree/scripts/fetch-fonts.sh" 2>"$work/err"; }
echo plain >"$work/up/Plain.ttf"
python3 - "$work/up/pack.zip" <<'PY'
import sys, zipfile
with zipfile.ZipFile(sys.argv[1], "w") as z:
    z.writestr("ttf/Packed.ttf", "packed\n")
PY
echo packed >"$work/packed"
list=$work/tree/packaging/libreoffice/fonts/SOURCES.tsv
write() { # member, packed hash, zip hash
  printf 'folder\tfile\turl\tsha256\tupstream_sha256\n' >"$list"
  printf 'One\tPlain.ttf\t%s\t%s\t%s\n' "$(url "$work/up/Plain.ttf")" "$(sum "$work/up/Plain.ttf")" "$(sum "$work/up/Plain.ttf")" >>"$list"
  printf 'Two\tPacked.ttf\t%s#%s\t%s\t%s\n' "$(url "$work/up/pack.zip")" "$1" "$2" "$3" >>"$list"
}
fonts=$work/tree/engines/fonts
zip=$(sum "$work/up/pack.zip")
write ttf/Packed.ttf "$(sum "$work/packed")" "$zip"
fetch >/dev/null || fail "a good list failed: $(cat "$work/err")"
[ "$(cat "$fonts/One/Plain.ttf")" = plain ] || fail "the plain file is wrong"
[ "$(cat "$fonts/Two/Packed.ttf")" = packed ] || fail "the zip member is wrong"
[ -z "$(fetch)" ] || fail "a second run fetched again"

# A spoiled copy of upstream is fetched again.
rm -rf "$fonts/Two"
echo spoiled >"$fonts/.upstream/$zip"
fetch >/dev/null || fail "a spoiled cache stopped it: $(cat "$work/err")"
[ "$(cat "$fonts/Two/Packed.ttf")" = packed ] || fail "the spoiled cache was used"

leftovers() { find "$fonts" -name '*.part' | grep . && fail "$1 left a part file" || true; }
rm -rf "$fonts"
write ttf/Packed.ttf 0000 "$zip"
if fetch >/dev/null; then fail "a wrong font hash passed"; fi
grep -q 'Two/Packed.ttf is' "$work/err" || fail "the wrong hash is not named: $(cat "$work/err")"
[ ! -e "$fonts/Two/Packed.ttf" ] || fail "a wrong font was kept"
leftovers "a wrong font hash"

write ttf/Gone.ttf "$(sum "$work/packed")" "$zip"
if fetch >/dev/null; then fail "a missing member passed"; fi
grep -q 'has no ttf/Gone.ttf' "$work/err" || fail "the missing member is not named: $(cat "$work/err")"
leftovers "a missing member"

rm -rf "$fonts"
write ttf/Packed.ttf "$(sum "$work/packed")" 0000
if fetch >/dev/null; then fail "a wrong upstream hash passed"; fi
grep -q 'pack.zip is' "$work/err" || fail "the wrong upstream is not named: $(cat "$work/err")"
[ ! -e "$fonts/.upstream/0000" ] || fail "a wrong upstream was kept"
leftovers "a wrong upstream"

rm "$work/up/pack.zip"
write ttf/Packed.ttf "$(sum "$work/packed")" "$zip"
if fetch >/dev/null; then fail "a missing upstream passed"; fi
grep -q 'cannot fetch' "$work/err" || fail "the missing upstream is not named: $(cat "$work/err")"
leftovers "a missing upstream"
echo "PASS: fetch-fonts.sh fetches, unzips, checks, refetches and leaves nothing half done"
