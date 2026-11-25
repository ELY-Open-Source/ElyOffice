#!/usr/bin/env bash
# Another program edits and saves a workbook open in ElyTable.
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
cargo build -q -p ely-table --manifest-path "$root/Cargo.toml"
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
  lok_workers_gone "$work/out" || fail "the worker outlived ElyTable"
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
# A block through standard input, read back as JSON.
python - "$(native "$table")" "$file" <<'PY' || fail "range set and get did not round-trip"
import json, subprocess, sys
table, file = sys.argv[1], sys.argv[2]
def run(*words, data=None):
    return subprocess.run([table, 'range', *words], input=data, capture_output=True)
assert run('set', file, 't!D5', '-', data='1\t=D5*2\n世界\t\n'.encode()).returncode == 0
got = json.loads(run('get', file, 't!D5:E6').stdout)
assert [[c['input'] for c in row] for row in got['rows']] == [['1', '=D5*2'], ['世界', '']], got
assert got['rows'][0][1]['text'] == '2', got
shape = run('set', file, 't!D5:E6', '[["x"]]')
assert shape.returncode == 1 and b'does not match' in shape.stderr, shape
ragged = run('set', file, 't!D5', 'a\tb\nc')
assert ragged.returncode == 2 and b'row 2 has 1 cells' in ragged.stderr, ragged
PY
# Sheets and rows change from outside; the data moves along.
python - "$(native "$table")" "$file" <<'PY' || fail "sheet and rows did not change from outside"
import json, subprocess, sys
table, file = sys.argv[1], sys.argv[2]
def run(*words):
    return subprocess.run([table, *words], capture_output=True)
def names():
    return [sheet['name'] for sheet in json.loads(run('sheet', 'list', file).stdout)]
first = names()
assert run('sheet', 'add', file, 'Added').returncode == 0
assert names() == first + ['Added'], names()
assert run('rows', 'insert', file, 't!1').returncode == 0
got = json.loads(run('range', 'get', file, 't!D6').stdout)
assert got['rows'][0][0]['input'] == '1', got
assert run('rows', 'delete', file, 't!1').returncode == 0
assert run('sheet', 'remove', file, 'Added').returncode == 0
assert names() == first, names()
size = run('columns', 'width', file, 't!C', 'wide')
assert size.returncode == 2 and b'invalid size' in size.stderr, size
missing = run('sheet', 'remove', file, 'Nope')
assert missing.returncode == 1 and b'Nope: no such sheet' in missing.stderr, missing
PY
# Formats from outside: the shown text follows the number format.
python - "$(native "$table")" "$file" <<'PY' || fail "format did not apply from outside"
import json, subprocess, sys
table, file = sys.argv[1], sys.argv[2]
def run(*words):
    return subprocess.run([table, *words], capture_output=True)
done = run('format', file, 't!D5:E5', 'number=0.00', 'bold=on', 'border=outer', 'border-bottom=double:1.5pt:#000000')
assert done.returncode == 0, done
got = json.loads(run('range', 'get', file, 't!D5:E5').stdout)
assert [c['text'] for c in got['rows'][0]] == ['1.00', '2.00'], got
unknown = run('format', file, 't!D5', 'shade=on')
assert unknown.returncode == 2 and b'shade: unknown format property' in unknown.stderr, unknown
code = run('format', file, 't!D5', 'number=0E')
assert code.returncode == 1 and b'invalid number format code "0E"' in code.stderr, code
PY
saved=$(cell save "$file" | python -c 'import json, sys; sys.stdout.write(json.load(sys.stdin))') || fail "saving"
[ "$(native "$saved")" = "$file" ] || fail "save answered '$saved', not $file"
# On disk at the answer: crash recovery cannot stand in.
python -c "import sys,zipfile;z=zipfile.ZipFile(sys.argv[1]);x=z.read('xl/worksheets/sheet1.xml').decode();sys.exit(0 if '>6*7</f>' in x else 1)" "$file" || fail "the saved book.xlsx lacks B2"
"$table" "$big" || fail "handing the big workbook over"
shows big.xlsx
# The write and the second save arrive during the first.
mark=$(wc -l < "$work/out")
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
# Both saves were asked before the first one landed.
tail -n +"$((mark + 1))" "$work/out" | awk '/another program asks save/ { asks++ }
  /files: saved revision .*big\.xlsx/ { landed = 1; exit }
  END { exit !(landed && asks >= 2) }' || fail "the saves did not overlap"
echo "ok: ElyTable answered cell docs, get, set, range set and get, sheet and rows changes, formats, and save from another process; each save answered once its own write was on disk"
