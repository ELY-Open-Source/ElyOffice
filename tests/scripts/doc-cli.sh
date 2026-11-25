#!/usr/bin/env bash
# Another program reads, replaces and saves text open in ElyDoc.
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
saved=$(text save "$file" | python -c 'import json, sys; sys.stdout.write(json.load(sys.stdin))') || fail "saving"
[ "$(native "$saved")" = "$file" ] || fail "save answered '$saved', not $file"
# On disk at the answer: crash recovery cannot stand in.
python - "$file" <<'PY' || fail "the saved note.docx lacks the replacement"
import re, sys, zipfile
xml = zipfile.ZipFile(sys.argv[1]).read('word/document.xml').decode()
sys.exit(0 if ''.join(re.findall(r'<w:t[^>]*>([^<]*)</w:t>', xml)) == 'Ely 世界 TEXT' else 1)
PY
echo "ok: ElyDoc answered text docs, get, replace and save from another process; the save was on disk at its answer"
printf 'line one\r\nline two\n' | text append "$file" - || fail "appending from standard input"
[ "$(text get "$file")" = $'Ely 世界 TEXT\nline one\nline two' ] || fail "after append, get read '$(text get "$file")'"
text append "$file" Heading --style 'Heading 1' || fail "appending a heading"
text get "$file" --json | python -c '
import json, sys
got = json.load(sys.stdin)
sys.exit(0 if got[-1] == {"style": "Heading 1", "text": "Heading"} and len(got) == 4 else 1)' || fail "get --json read $(text get "$file" --json)"
if text append "$file" x --style Nope 2>"$work/err"; then fail "a missing style was used"; fi
grep -q "Nope: no such paragraph style" "$work/err" || fail "the style refusal says $(cat "$work/err")"
text replace "$file" 'line one' $'one $1\ntwo &' || fail "replacing with a break"
[ "$(text get "$file")" = $'Ely 世界 TEXT\none $1\ntwo &\nline two\nHeading' ] || fail "after the break, get read '$(text get "$file")'"
echo "ok: ElyDoc appended from standard input, with a style, listed styles as JSON and split a paragraph"
"$doc" format "$file" 'line two' bold=on color=#B00020 style='Heading 1' || fail "formatting"
text get "$file" --json | python -c '
import json, sys
got = json.load(sys.stdin)
sys.exit(0 if {"style": "Heading 1", "text": "line two"} in got else 1)' || fail "after format, get --json read $(text get "$file" --json)"
if "$doc" format "$file" zebra bold=on 2>"$work/err"; then fail "a missing word was formatted"; fi
grep -q 'no match for "zebra"' "$work/err" || fail "the format refusal says $(cat "$work/err")"
rc=0; "$doc" format "$file" Heading wrap=on 2>/dev/null || rc=$?
[ "$rc" = 2 ] || fail "an unknown property exited $rc, not 2"
echo "ok: ElyDoc formatted every match from another process; a missing word and an unknown property were refused"
printf 'Item\tQty\r\nTea\t3\n' | "$doc" table insert "$file" - --header --after 'two &' || fail "inserting a table from standard input"
"$doc" table insert "$file" '[["end"]]' || fail "inserting a JSON table at the end"
text get "$file" --json | python -c '
import json, sys
got = [(p["style"], p["text"]) for p in json.load(sys.stdin)]
at = got.index(("Table Heading", "Item"))
want = [("Table Heading", "Qty"), ("Table Contents", "Tea"), ("Table Contents", "3")]
sys.exit(0 if got[at - 1][1] == "two &" and got[at + 1:at + 4] == want and got[at + 4][1] == "line two"
         and [text for _, text in got[-2:]] == ["end", ""] else 1)' || fail "after table insert, get --json read $(text get "$file" --json)"
if "$doc" table insert "$file" '[["a"]]' --after zebra 2>"$work/err"; then fail "a table went after a missing word"; fi
grep -q 'no match for "zebra"' "$work/err" || fail "the table refusal says $(cat "$work/err")"
rc=0; printf 'a\tb\nc\n' | "$doc" table insert "$file" - 2>/dev/null || rc=$?
[ "$rc" = 2 ] || fail "uneven rows exited $rc, not 2"
"$doc" table insert "$file" '[[""]]' || fail "inserting one empty cell"
"$doc" table insert "$file" '[[" "]]' --after 'two &' || fail "inserting one blank cell after a match"
echo "ok: ElyDoc inserted tables after a match and at the end, empty cells too; a missing word and uneven rows were refused"
printf '<svg xmlns="http://www.w3.org/2000/svg" width="40pt" height="20pt"><rect width="40" height="20"/></svg>' > "$work/dot.svg"
count() { text get "$file" --json | python -c 'import json, sys; print(len(json.load(sys.stdin)))'; }
before=$(count)
"$doc" image insert "$file" "$work/dot.svg" --width 2cm || fail "inserting a picture at the end"
"$doc" image insert "$file" "$work/dot.svg" --after 'two &' || fail "inserting a picture after a match"
[ "$(count)" = $((before + 2)) ] || fail "after image insert, get --json read $(text get "$file" --json)"
rc=0; "$doc" image insert "$file" "$work/gone.svg" 2>/dev/null || rc=$?
[ "$rc" = 2 ] || fail "a missing picture exited $rc, not 2"
printf 'not a picture' > "$work/fake.png"
if "$doc" image insert "$file" "$work/fake.png" 2>"$work/err"; then fail "a file that is not a picture went in"; fi
grep -q 'cannot read as an image' "$work/err" || fail "the picture refusal says $(cat "$work/err")"
[ "$(count)" = $((before + 2)) ] || fail "a refused picture changed the document"
echo "ok: ElyDoc inserted pictures at the end and after a match; a missing file and a non-picture were refused"
