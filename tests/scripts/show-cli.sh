#!/usr/bin/env bash
# Another program reads, replaces and saves text open in ElyShow.
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
cargo build -q -p ely-show --manifest-path "$root/Cargo.toml"
show=$root/target/debug/ely-show
export ELY_DATA_ROOT ELY_LOCALE=en ELY_NO_DIALOG=1 ELY_LOG=info
ELY_DATA_ROOT=$(native "$work/data")
file=$(native "$work/deck.pptx")
cp "$root/tests/fixtures/office/t.pptx" "$work/deck.pptx"
text() { "$show" text "$@"; }
# Slide n of `get`, slides split by form feeds.
slide() {
  text get "$file" | python -c 'import sys; print(sys.stdin.read().split("\f")[int(sys.argv[1])].strip())' "$1"
}

ELY_SMOKE=hold "$show" "$file" >"$work/out" 2>&1 &
pid=$!
for _ in $(seq 120); do
  text docs 2>/dev/null | grep -qF deck.pptx && break
  kill -0 "$pid" 2>/dev/null || fail "ElyShow exited"
  sleep 1
done
text docs | grep -qF deck.pptx || fail "deck.pptx never showed in text docs"
[ "$(text get "$file" | tr -cd '\f' | wc -c)" = 8 ] || fail "get gave no nine slides"
[ "$(slide 3)" = "01" ] || fail "slide 4 read '$(slide 3)'"
text replace "$file" 01 'Ely 世界' || fail "replacing"
[ "$(slide 3)" = "Ely 世界" ] || fail "after replace, slide 4 read '$(slide 3)'"
if text replace "$file" zebra x 2>"$work/err"; then fail "a missing word was replaced"; fi
grep -q "nothing replaced" "$work/err" || fail "the refusal says $(cat "$work/err")"
if text get "$work/other.pptx" 2>/dev/null; then fail "a file not open was read"; fi
if text replace "$file" 01 2>/dev/null; then fail "a short command ran"; fi
saved=$(text save "$file") || fail "saving"
[ "$(native "$saved")" = "$file" ] || fail "save answered '$saved', not $file"
# On disk at the answer: crash recovery cannot stand in.
python - "$file" <<'PY' || fail "the saved deck.pptx lacks the replacement"
import re, sys, zipfile
z = zipfile.ZipFile(sys.argv[1])
said = [''.join(re.findall(r'<a:t>([^<]*)</a:t>', z.read(n).decode())) for n in z.namelist() if re.match(r'ppt/slides/slide\d+\.xml$', n)]
sys.exit(0 if 'Ely 世界' in said and '01' not in said else 1)
PY
echo "ok: ElyShow answered text docs, get, replace and save from another process; the save was on disk at its answer"
