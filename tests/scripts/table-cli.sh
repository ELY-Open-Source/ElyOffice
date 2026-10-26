#!/usr/bin/env bash
# Another program edits and saves a workbook open in ElyTable.
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

cargo build -q -p ely-lok --bins -p ely-table --manifest-path "$root/Cargo.toml"
table=$root/target/debug/ely-table
export ELY_DATA_ROOT ELY_LOCALE=en ELY_NO_DIALOG=1 ELY_LOG=info
ELY_DATA_ROOT=$(native "$work/data")
file=$(native "$work/book.xlsx")
cp "$root/tests/fixtures/office/t.xlsx" "$work/book.xlsx"
big=$(native "$work/big.xlsx")
# Sixty thousand rows: a save takes long enough to overlap.
python - "$big" <<'PY'
import sys, zipfile
rows = ''.join(f'<row r="{n}">' + ''.join(f'<c r="{c}{n}"><v>7</v></c>' for c in 'ABCD') + '</row>' for n in range(1, 60001))
ns = 'xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"'
rel = 'http://schemas.openxmlformats.org/officeDocument/2006/relationships'
with zipfile.ZipFile(sys.argv[1], 'w', zipfile.ZIP_DEFLATED) as z:
    z.writestr('[Content_Types].xml', '<?xml version="1.0"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">'
               '<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>'
               '<Default Extension="xml" ContentType="application/xml"/>'
               '<Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>'
               '<Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/></Types>')
    z.writestr('_rels/.rels', f'<?xml version="1.0"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">'
               f'<Relationship Id="r1" Type="{rel}/officeDocument" Target="xl/workbook.xml"/></Relationships>')
    z.writestr('xl/workbook.xml', f'<?xml version="1.0"?><workbook {ns} xmlns:r="{rel}"><sheets><sheet name="t" sheetId="1" r:id="r1"/></sheets></workbook>')
    z.writestr('xl/_rels/workbook.xml.rels', f'<?xml version="1.0"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">'
               f'<Relationship Id="r1" Type="{rel}/worksheet" Target="worksheets/sheet1.xml"/></Relationships>')
    z.writestr('xl/worksheets/sheet1.xml', f'<?xml version="1.0"?><worksheet {ns}><sheetData>{rows}</sheetData></worksheet>')
PY
cell() { "$table" cell "$@"; }

shows() {
  for _ in $(seq 120); do
    cell docs 2>/dev/null | grep -qF "$1" && return
    kill -0 "$pid" 2>/dev/null || fail "ElyTable exited"
    sleep 1
  done
  fail "$1 never showed in cell docs"
}
start() {
  ELY_SMOKE=hold "$table" "$file" >"$work/out" 2>&1 &
  pid=$!
  shows book.xlsx
}
stop() {
  kill "$pid"
  wait "$pid" 2>/dev/null || true
  pid=
}

start
first=$(cell get "$file" 't!A1') || fail "reading A1"
case $first in *'"address"'*) ;; *) fail "A1 read as $first" ;; esac
cell set "$file" 't!B2' '=6*7' || fail "writing B2"
[ "$(cell get "$file" 't!B2')" = '{"address":"t!$B$2","text":"42","input":"=6*7"}' ] || fail "B2 read as $(cell get "$file" 't!B2')"
if cell get "$file" 'B2' 2>"$work/err"; then fail "an address without its sheet was read"; fi
grep -q "names no cell" "$work/err" || fail "the refusal says $(cat "$work/err")"
if cell get "$work/other.xlsx" 't!A1' 2>/dev/null; then fail "a file not open was read"; fi
if cell set "$file" 2>/dev/null; then fail "a short command ran"; fi
# Past the old frame cap; MSYS limits arguments in bytes.
python - "$(native "$table")" "$file" <<'PY' || fail "24,000 characters did not go in and read back whole"
import json, subprocess, sys
table, file = sys.argv[1], sys.argv[2]
text = '汉' * 24000
subprocess.run([table, 'cell', 'set', file, 't!C3', text], check=True)
got = json.loads(subprocess.run([table, 'cell', 'get', file, 't!C3'], check=True, capture_output=True).stdout)
sys.exit(0 if got['text'] == got['input'] == text else 1)
PY
saved=$(cell save "$file") || fail "saving"
[ "$(native "$saved")" = "$file" ] || fail "save answered '$saved', not $file"
# On disk at the answer: crash recovery cannot stand in.
python -c "import sys,zipfile;z=zipfile.ZipFile(sys.argv[1]);x=z.read('xl/worksheets/sheet1.xml').decode();sys.exit(0 if '>6*7</f>' in x else 1)" "$file" || fail "the saved book.xlsx lacks B2"
"$table" "$big" || fail "handing the big workbook over"
shows big.xlsx
# The write and the second save arrive during the first.
cell save "$big" >/dev/null &
first_save=$!
sleep 0.05
cell set "$big" 't!B2' 'later' &
setting=$!
sleep 0.05
cell save "$big" >/dev/null || fail "the second save"
python -c "import sys,zipfile;z=zipfile.ZipFile(sys.argv[1]);sys.exit(0 if any(b'>later<' in z.read(n) for n in z.namelist()) else 1)" "$big" || fail "the second save answered before B2 was on disk"
wait "$first_save" || fail "the first save"
wait "$setting" || fail "writing B2 during a save"
grep -q "waits for the running one" "$work/out" || fail "the saves did not overlap"
echo "ok: ElyTable answered cell docs, get, set and save from another process; each save answered once its own write was on disk"
