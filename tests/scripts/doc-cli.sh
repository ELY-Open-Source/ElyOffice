#!/usr/bin/env bash
# Another program reads, replaces and saves text open in ElyDoc.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
work=$(mktemp -d)
pid=
cleanup() {
  [ -z "$pid" ] || kill "$pid" 2>/dev/null || true
  [ -z "$pid" ] || wait "$pid" 2>/dev/null || true
  rm -rf "$work"
}
trap cleanup EXIT
fail() {
  echo "FAIL: $*" >&2
  [ ! -f "$work/out" ] || tail -n 20 "$work/out" >&2
  exit 1
}
native() { if command -v cygpath >/dev/null; then cygpath -m "$1"; else printf '%s' "$1"; fi; }

cargo build -q -p ely-lok --bin ely-lok-worker --manifest-path "$root/Cargo.toml"
cargo build -q -p ely-doc --manifest-path "$root/Cargo.toml"
doc=$root/target/debug/ely-doc
export ELY_DATA_ROOT ELY_LOCALE=en ELY_NO_DIALOG=1 ELY_LOG=info
ELY_DATA_ROOT=$(native "$work/data")
file=$(native "$work/note.docx")
cp "$root/tests/fixtures/office/t.docx" "$work/note.docx"
text() { "$doc" text "$@"; }

ELY_SMOKE=hold "$doc" "$file" >"$work/out" 2>&1 &
pid=$!
for _ in $(seq 120); do
  text docs 2>/dev/null | grep -qF note.docx && break
  kill -0 "$pid" 2>/dev/null || fail "ElyDoc exited"
  sleep 1
done
text docs | grep -qF note.docx || fail "note.docx never showed in text docs"
[ "$(text get "$file")" = "BUNDLE TEXT" ] || fail "get read '$(text get "$file")'"
text replace "$file" BUNDLE 'Ely 世界' || fail "replacing"
[ "$(text get "$file")" = "Ely 世界 TEXT" ] || fail "after replace, get read '$(text get "$file")'"
if text replace "$file" zebra x 2>"$work/err"; then fail "a missing word was replaced"; fi
grep -q "nothing replaced" "$work/err" || fail "the refusal says $(cat "$work/err")"
if text get "$work/other.docx" 2>/dev/null; then fail "a file not open was read"; fi
if text replace "$file" BUNDLE 2>/dev/null; then fail "a short command ran"; fi
saved=$(text save "$file") || fail "saving"
[ "$(native "$saved")" = "$file" ] || fail "save answered '$saved', not $file"
# On disk at the answer: crash recovery cannot stand in.
python - "$file" <<'PY' || fail "the saved note.docx lacks the replacement"
import re, sys, zipfile
xml = zipfile.ZipFile(sys.argv[1]).read('word/document.xml').decode()
sys.exit(0 if ''.join(re.findall(r'<w:t[^>]*>([^<]*)</w:t>', xml)) == 'Ely 世界 TEXT' else 1)
PY
echo "ok: ElyDoc answered text docs, get, replace and save from another process; the save was on disk at its answer"
