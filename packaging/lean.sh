#!/usr/bin/env bash
# Drops what Ely never uses. Usage: lean.sh <kernel share folder>
set -euo pipefail

share=${1:?the kernel share folder, Resources on macOS}
# Gallery and thesaurus; one kernel serves all three apps.
rm -rf "$share/gallery"
en=$share/extensions/dict-en
rm "$en"/th_en_US_v2.*
awk '/<node oor:name="ThesDic_/ { skip = 1 } !skip { print } skip && /<\/node>/ { skip = 0 }' \
  "$en/dictionaries.xcu" >"$en/dictionaries.xcu.next"
mv "$en/dictionaries.xcu.next" "$en/dictionaries.xcu"
! grep -q 'DICT_THES' "$en/dictionaries.xcu" || {
  echo "lean.sh: the thesaurus is still in $en/dictionaries.xcu" >&2
  exit 1
}
