#!/usr/bin/env bash
# ElyDoc print: the document's paper and scale, cancel changes nothing.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
[ "$(uname)" = Darwin ] || { echo "SKIP: macOS only"; exit 0; }
command -v pdftotext >/dev/null && command -v pdfinfo >/dev/null || { echo "SKIP: needs poppler"; exit 0; }
work=$(mktemp -d)
cleanup() {
  local status=$?
  if [ -n "${pid:-}" ]; then
    pkill -9 -P "$pid" 2>/dev/null || true
    kill "$pid" 2>/dev/null || true
    wait "$pid" 2>/dev/null || true
  fi
  [ "$status" = 0 ] && rm -rf "$work" || echo "kept $work" >&2
  exit "$status"
}
trap cleanup EXIT
fail() {
  echo "FAIL: $*" >&2
  exit 1
}

cargo build -q -p ely-lok --bin ely-lok-worker --manifest-path "$root/Cargo.toml"
cargo build -q -p ely-doc --manifest-path "$root/Cargo.toml"
for tool in ax-values ax-press; do swiftc -O -o "$work/$tool" "$root/tests/scripts/$tool.swift"; done
[ "$("$work/ax-values" $$ | head -n 1)" != untrusted ] || { echo "SKIP: needs Accessibility permission"; exit 0; }
home="$work/home"
data="$home/Library/Application Support/ElyOffice/ely-doc"
mkdir -p "$home"
# A5 landscape, and A5 then A3; written without LibreOffice.
python3 - "$work" <<'PY'
import sys, zipfile
w = 'xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"'
margins = '<w:pgMar w:top="1440" w:right="1440" w:bottom="1440" w:left="1440"/>'
a5 = f'<w:pgSz w:w="11906" w:h="8391" w:orient="landscape"/>{margins}'
a3 = f'<w:pgSz w:w="23811" w:h="16838" w:orient="landscape"/>{margins}'
thin = '<w:pgMar w:top="360" w:right="720" w:bottom="360" w:left="720"/>'
wide = f'<w:pgSz w:w="20000" w:h="4000" w:orient="landscape"/>{thin}'
square = f'<w:pgSz w:w="12000" w:h="12000"/>{thin}'
bodies = {
    "a5.docx": f'<w:p><w:r><w:t>Print me</w:t></w:r></w:p><w:sectPr>{a5}</w:sectPr>',
    "mixed.docx": f'<w:p><w:pPr><w:sectPr>{a5}</w:sectPr></w:pPr><w:r><w:t>Small</w:t></w:r></w:p>'
                  f'<w:p><w:pPr><w:jc w:val="right"/></w:pPr><w:r><w:t>Edge</w:t></w:r></w:p><w:sectPr>{a3}</w:sectPr>',
    "crossed.docx": f'<w:p><w:r><w:t>Begin</w:t></w:r></w:p>'
                    f'<w:p><w:pPr><w:jc w:val="right"/><w:sectPr>{wide}</w:sectPr></w:pPr><w:r><w:t>Right</w:t></w:r></w:p>'
                    f'<w:p><w:r><w:t>Square</w:t></w:r></w:p><w:sectPr>{square}</w:sectPr>',
}
for name, body in bodies.items():
    parts = {
        "[Content_Types].xml": '<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">'
        '<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>'
        '<Default Extension="xml" ContentType="application/xml"/>'
        '<Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/></Types>',
        "_rels/.rels": '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">'
        '<Relationship Id="r1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>',
        "word/document.xml": f'<w:document {w}><w:body>{body}</w:body></w:document>',
    }
    with zipfile.ZipFile(f"{sys.argv[1]}/{name}", "w") as archive:
        for part, text in parts.items():
            archive.writestr(part, text)
PY
cp "$work/a5.docx" "$work/a5.orig"
keys() { osascript -e 'tell application "System Events"' -e "$1" -e 'end tell' >/dev/null; }
seen() {
  for _ in $(seq 80); do "$work/ax-values" "$pid" | grep -q -- "$1" && return 0; sleep 0.25; done
  return 1
}
# How often $1 has opened so far.
opened() { local n; n=$(grep -c "opened $1" "$data/ely.log" 2>/dev/null || true); echo "${n:-0}"; }
# Opens $1; extra environment in the rest.
start() {
  local file=$1 before
  shift
  before=$(opened "$file")
  env "$@" ELY_SMOKE=hold ELY_LOCALE=en ELY_NO_DIALOG=1 HOME="$home" perl -e 'alarm 200; exec @ARGV' \
    "$root/target/debug/ely-doc" "$work/$file" >"$work/out" 2>&1 &
  pid=$!
  for _ in $(seq 150); do [ "$(opened "$file")" -gt "$before" ] && break; sleep 0.2; done
  [ "$(opened "$file")" -gt "$before" ] || fail "$file did not open"
  osascript -e "tell application \"System Events\" to set frontmost of (first process whose unix id is $pid) to true" >/dev/null
  sleep 1
}
stop() {
  pkill -9 -P "$pid" 2>/dev/null || true
  kill "$pid" 2>/dev/null || true
  wait "$pid" 2>/dev/null || true
  pid=
}
print() {
  keys 'keystroke "p" using command down'
  seen "title=Print…" || fail "no print dialog"
  "$work/ax-press" "$pid" "Print…" AXButton >/dev/null
}

# The panel, cancelled: nothing changes.
start a5.docx
print
for _ in $(seq 80); do grep -q "print: paper" "$data/ely.log" && break; sleep 0.25; done
grep -q "print: paper" "$data/ely.log" || fail "the print panel did not open"
sleep 1
"$work/ax-press" "$pid" Cancel AXButton >/dev/null
for _ in $(seq 40); do grep -q "print done, printed false" "$data/ely.log" && break; sleep 0.25; done
grep -q "print done, printed false" "$data/ely.log" || fail "the cancel was not seen"
[ -z "$(find "$data/work" -name print.pdf)" ] || fail "the print copy stays"
cmp -s "$work/a5.docx" "$work/a5.orig" || fail "printing changed the file"
keys 'keystroke "w" using command down'
for _ in $(seq 40); do grep -q "office: closed a5.docx" "$data/ely.log" && break; sleep 0.25; done
grep -q "office: closed a5.docx" "$data/ely.log" || fail "the printed document asks or stays"
stop

# The real job, saved: its pages and text positions.
printed() {
  rm -f "$work/printed.pdf"
  # A shared half scale the print must undo.
  start "$1" ELY_PRINT_TO="$work/printed.pdf" ELY_PRINT_SHARED_SCALE=0.5
  print
  for _ in $(seq 80); do [ -s "$work/printed.pdf" ] && break; sleep 0.25; done
  [ -s "$work/printed.pdf" ] || fail "$1: no printed output"
  stop
}
sizes() { pdfinfo -f 1 -l 9 "$work/printed.pdf" | awk '/^Page +[0-9]+ size:/ { printf "%.0fx%.0f ", $4, $6 }'; }
# The printed pages are $1 (sizes), each within a point.
pages_are() {
  python3 -c 'import sys; got, want = sys.argv[1].split(), sys.argv[2].split()
ok = len(got) == len(want) and all(abs(float(a) - float(b)) <= 1 for g, w in zip(got, want) for a, b in zip(g.split("x"), w.split("x")))
sys.exit(0 if ok else 1)' "$(sizes)" "$1"
}
word() { pdftotext -bbox "$work/printed.pdf" - | grep -m 1 ">$1<"; }
printed a5.docx
pages_are "595x420" || fail "a5: printed on $(sizes), not 595x420"
# One inch in: no scale and no shift.
x=$(word Print | sed -E 's/.*xMin="([0-9.]+)".*/\1/')
python3 -c 'import sys; sys.exit(0 if abs(float(sys.argv[1]) - 72) < 2 else 1)' "$x" || fail "a5: the text starts at $x, not 72"
printed mixed.docx
pages_are "1191x842 1191x842" || fail "mixed: printed on $(sizes), not the A3 paper twice"
word Small >/dev/null || fail "mixed: the A5 page lost its text"
edge=$(word Edge | sed -E 's/.*xMax="([0-9.]+)".*/\1/')
python3 -c 'import sys; sys.exit(0 if 1000 < float(sys.argv[1]) <= 1191 else 1)' "$edge" || fail "mixed: the A3 edge text is at $edge"
printed crossed.docx
pages_are "1000x600 1000x600" || fail "crossed: printed on $(sizes), not 1000x600 twice"
word Begin >/dev/null && word Square >/dev/null || fail "crossed: a page lost its text"
right=$(word Right | sed -E 's/.*xMax="([0-9.]+)".*/\1/')
python3 -c 'import sys; sys.exit(0 if 900 < float(sys.argv[1]) <= 1000 else 1)' "$right" || fail "crossed: the right end is at $right"
echo "ok: print"
