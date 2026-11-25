#!/usr/bin/env bash
# Another program edits text, notes and slides, then saves.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
# Its real path: macOS links /var to /private/var.
work=$(cd "$(mktemp -d)" && pwd -P)
pid=
. "$root/tests/scripts/lok-workers.sh"
cleanup() {
  [ -z "$pid" ] || kill "$pid" 2>/dev/null || true
  [ -z "$pid" ] || wait "$pid" 2>/dev/null || true
  # Its worker writes into the folder until it ends.
  if [ -f "$work/out" ] && ! lok_workers_gone "$work/out"; then
    rm -rf "$work" || true
    exit 1
  fi
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
  text get "$file" | python -c 'import sys; sys.stdout.buffer.write(sys.stdin.buffer.read().split(b"\f")[int(sys.argv[1])].strip())' "$1"
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
slides() { "$show" slide "$@"; }
# Field KEY of slide N, or the slide count.
listed() {
  slides list "$file" | python -c 'import json, sys; s = json.load(sys.stdin); print(len(s) if sys.argv[1] == "count" else s[int(sys.argv[1])][sys.argv[2]])' "$@"
}
[ "$(listed count)" = 9 ] || fail "slide list gave $(listed count) slides"
slides add "$file" --layout title-only || fail "adding a slide"
[ "$(listed 9 layout)" = title-only ] || fail "the added slide is $(listed 9 layout)"
slides move "$file" 10 1 || fail "moving a slide"
[ "$(listed 0 layout)" = title-only ] || fail "the moved slide is not first"
slides delete "$file" 1 || fail "deleting a slide"
slides duplicate "$file" 1 || fail "duplicating a slide"
[ "$(listed count)" = 10 ] || fail "the copy is missing"
slides delete "$file" 2 || fail "deleting the copy"
[ "$(listed count)" = 9 ] && [ "$(slide 3)" = "Ely 世界" ] || fail "the deck did not come back to nine slides"
rc=0; slides delete "$file" 99 2>"$work/err" || rc=$?
[ "$rc" = 1 ] && grep -q "does not exist" "$work/err" || fail "deleting slide 99 gave $rc: $(cat "$work/err")"
rc=0; slides add "$file" --layout nope 2>"$work/err" || rc=$?
[ "$rc" = 2 ] && grep -q "unknown layout" "$work/err" || fail "a bad layout gave $rc: $(cat "$work/err")"
# Text and notes by slide; a break starts a paragraph.
slides add "$file" --layout title-content || fail "adding a titled slide"
text set "$file" 10 title Plan || fail "setting a title"
printf 'One\r\nTwo' | text set "$file" 10 body - || fail "setting a body from standard input"
[ "$(listed 9 title)" = Plan ] || fail "the title is $(listed 9 title)"
[ "$(slide 9)" = $'Plan\nOne\nTwo' ] || fail "slide 10 read '$(slide 9)'"
text replace "$file" Plan $'Plan\nB' || fail "replacing with a line break"
[ "$(slide 9)" = $'Plan\nB\nOne\nTwo' ] || fail "after the break, slide 10 read '$(slide 9)'"
notes() { "$show" notes "$@"; }
notes set "$file" 10 $'Say hi\nthen go' || fail "setting notes"
[ "$(notes get "$file" 10)" = $'Say hi\nthen go' ] || fail "the notes read '$(notes get "$file" 10)'"
rc=0; text set "$file" 10 side x 2>"$work/err" || rc=$?
[ "$rc" = 2 ] && grep -q "must be title or body" "$work/err" || fail "a bad box gave $rc: $(cat "$work/err")"
rc=0; notes get "$file" 99 2>"$work/err" || rc=$?
[ "$rc" = 1 ] && grep -q "does not exist" "$work/err" || fail "notes of slide 99 gave $rc: $(cat "$work/err")"
# A picture on slide 10; refusals change nothing.
printf '<svg xmlns="http://www.w3.org/2000/svg" width="40pt" height="20pt"><rect width="40" height="20"/></svg>' > "$work/dot.svg"
"$show" image insert "$file" 10 "$(native "$work/dot.svg")" --width 4cm || fail "inserting a picture"
rc=0; "$show" image insert "$file" 10 "$(native "$work/gone.svg")" 2>/dev/null || rc=$?
[ "$rc" = 2 ] || fail "a missing picture exited $rc, not 2"
printf 'not a picture' > "$work/fake.png"
if "$show" image insert "$file" 10 "$(native "$work/fake.png")" 2>"$work/err"; then fail "a file that is not a picture went in"; fi
grep -q 'cannot read as an image' "$work/err" || fail "the picture refusal says $(cat "$work/err")"
rc=0; "$show" image insert "$file" 99 "$(native "$work/dot.svg")" 2>"$work/err" || rc=$?
[ "$rc" = 1 ] && grep -q "does not exist" "$work/err" || fail "a picture on slide 99 gave $rc: $(cat "$work/err")"
saved=$(text save "$file" | python -c 'import json, sys; sys.stdout.write(json.load(sys.stdin))') || fail "saving"
[ "$(native "$saved")" = "$file" ] || fail "save answered '$saved', not $file"
# On disk at the answer: crash recovery cannot stand in.
python - "$file" <<'PY' || fail "the saved deck.pptx lacks the replacement or picture"
import re, sys, zipfile
z = zipfile.ZipFile(sys.argv[1])
said = [''.join(re.findall(r'<a:t>([^<]*)</a:t>', z.read(n).decode())) for n in z.namelist() if re.match(r'ppt/slides/slide\d+\.xml$', n)]
pictures = [n for n in z.namelist() if re.match(r'ppt/slides/slide10\.xml$', n) and '<p:pic' in z.read(n).decode()]
sys.exit(0 if 'Ely 世界' in said and '01' not in said and pictures else 1)
PY
echo "ok: ElyShow answered text docs, get, replace (with a line break), set, notes get and set, slide list, add, move, duplicate, delete, image insert and save from another process; the save was on disk at its answer"
