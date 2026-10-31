# Sourced by Linux window tests: inputs, cases and the judge.

# The words an app shows, then its input.
shows() { # slug
  case $1 in
    ely-doc) echo "Kernel prose|prose.docx" ;;
    ely-table) echo "Kernel cell|cells.xlsx" ;;
    ely-show) echo "Click to add Title|" ;;
    ely-pdf) echo "Packaged PDF|page.pdf" ;;
    ely-mail) echo "No mail accounts yet|" ;;
    ely-tree) echo "hello.txt|repo" ;;
    ely-mark) echo "Packaged|note.md" ;;
    *) return 1 ;;
  esac
}
# Case: slug|words|input, for each first-batch app.
cases=()
for slug in $(awk -F'\t' '$4 == 1 { print $1 }' "$root/packaging/apps.tsv"); do
  said=$(shows "$slug") || fail "apps.tsv's $slug has no window case"
  cases+=("$slug|$said")
done
[ ${#cases[@]} -gt 0 ] || fail "apps.tsv lists no first-batch app"

# Omarchy on Catppuccin; a repo, a note, a PDF.
linux_inputs() { # root, work
  local fixtures=$1/crates/ely-ui/tests/fixtures/omarchy current=$2/omarchy/current
  mkdir -p "$current/theme"
  cp "$fixtures/catppuccin.toml" "$current/theme/colors.toml"
  echo catppuccin >"$current/theme.name"
  git init -q "$2/repo"
  echo hi >"$2/repo/hello.txt"
  printf '# Packaged on Linux\n\nEly reads this.\n' >"$2/note.md"
  python3 - "$2/page.pdf" <<'PY'
import sys
text = b"BT /F1 36 Tf 72 700 Td (Packaged PDF) Tj ET"
objects = [
    b"<< /Type /Catalog /Pages 2 0 R >>",
    b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
    b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R"
    b" /Resources << /Font << /F1 5 0 R >> >> >>",
    b"<< /Length %d >>\nstream\n%s\nendstream" % (len(text), text),
    b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
]
out = bytearray(b"%PDF-1.4\n")
offsets = []
for number, body in enumerate(objects, 1):
    offsets.append(len(out))
    out += b"%d 0 obj\n%s\nendobj\n" % (number, body)
xref = len(out)
out += b"xref\n0 %d\n0000000000 65535 f \n" % (len(objects) + 1)
out += b"".join(b"%010d 00000 n \n" % offset for offset in offsets)
out += b"trailer\n<< /Size %d /Root 1 0 R >>\nstartxref\n%d\n%%%%EOF\n" % (len(objects) + 1, xref)
open(sys.argv[1], "wb").write(out)
PY
  python3 - "$2" <<'PY'
import sys, zipfile
work = sys.argv[1]
types = '<?xml version="1.0" encoding="UTF-8"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/>%s</Types>'
rels = '<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">%s</Relationships>'
rel = '<Relationship Id="%s" Type="http://schemas.openxmlformats.org/%s" Target="%s"/>'
main = "http://schemas.openxmlformats.org/%s"
with zipfile.ZipFile(work + "/prose.docx", "w") as doc:
    doc.writestr("[Content_Types].xml", types % '<Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>')
    doc.writestr("_rels/.rels", rels % rel % ("r1", "officeDocument/2006/relationships/officeDocument", "word/document.xml"))
    doc.writestr("word/document.xml", '<?xml version="1.0" encoding="UTF-8"?><w:document xmlns:w="%s"><w:body><w:p><w:r><w:rPr><w:sz w:val="96"/></w:rPr><w:t>Kernel prose</w:t></w:r></w:p></w:body></w:document>' % (main % "wordprocessingml/2006/main"))
sheet = main % "spreadsheetml/2006/main"
with zipfile.ZipFile(work + "/cells.xlsx", "w") as book:
    book.writestr("[Content_Types].xml", types % ('<Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>'
        '<Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>'
        '<Override PartName="/xl/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml"/>'))
    book.writestr("_rels/.rels", rels % rel % ("r1", "officeDocument/2006/relationships/officeDocument", "xl/workbook.xml"))
    book.writestr("xl/_rels/workbook.xml.rels", rels % (rel % ("r1", "officeDocument/2006/relationships/worksheet", "worksheets/sheet1.xml") + rel % ("r2", "officeDocument/2006/relationships/styles", "styles.xml")))
    book.writestr("xl/workbook.xml", '<?xml version="1.0" encoding="UTF-8"?><workbook xmlns="%s" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets><sheet name="Sheet1" sheetId="1" r:id="r1"/></sheets></workbook>' % sheet)
    book.writestr("xl/styles.xml", '<?xml version="1.0" encoding="UTF-8"?><styleSheet xmlns="%s"><fonts count="2"><font><sz val="11"/><name val="Liberation Sans"/></font><font><sz val="36"/><name val="Liberation Sans"/></font></fonts><fills count="1"><fill><patternFill patternType="none"/></fill></fills><borders count="1"><border/></borders><cellStyleXfs count="1"><xf/></cellStyleXfs><cellXfs count="2"><xf fontId="0"/><xf fontId="1" applyFont="1"/></cellXfs></styleSheet>' % sheet)
    book.writestr("xl/worksheets/sheet1.xml", '<?xml version="1.0" encoding="UTF-8"?><worksheet xmlns="%s"><cols><col min="2" max="2" width="40" customWidth="1"/></cols><sheetData><row r="3" ht="48" customHeight="1"><c r="B3" s="1" t="inlineStr"><is><t>Kernel cell</t></is></c></row></sheetData></worksheet>' % sheet)
PY
}

# Stops an app started with setsid, and all it started.
stop_app() { # pid
  kill -- -"$1" 2>/dev/null || true
  local found
  for _ in $(seq 60); do
    found=0
    pgrep -s "$1" >/dev/null || found=$?
    [ "$found" -eq 1 ] && return 0
    [ "$found" -eq 0 ] || fail "pgrep failed with $found"
    sleep 0.25
  done
  fail "processes of session $1 did not exit"
}

# Title bar: Catppuccin is blue-grey, Latte is light.
band() { magick "$1" -crop 100%x3%+0+0 +repage -format "%[fx:int(mean.$2*255)]" info:; }

judge() { # slug, screenshot, words, app log
  local text
  grep -q 'omarchy catppuccin applied' "$4" || fail "$1 did not apply the Omarchy theme"
  case $1 in
    ely-doc | ely-table | ely-show)
      grep -q 'lok-worker: 1 documents open' "$4" || fail "$1's kernel opened no document"
      ! grep -q -E 'lok: tile.*failed' "$4" || fail "$1's kernel failed a tile"
      ;;
  esac
  local blue red
  blue=$(band "$2" b) || fail "magick cannot read $2"
  red=$(band "$2" r) || fail "magick cannot read $2"
  [ $((blue - red)) -gt 10 ] || fail "$1's title bar is not Catppuccin"
  # Dark text, twice the size: Xvfb draws at scale 1.
  magick "$2" -colorspace Gray -negate -resize 200% "$2.ocr.png"
  # Sparse text: a page beside panels hides from layout analysis.
  text=$(tesseract "$2.ocr.png" - --psm 11 2>"$2.ocr") || fail "tesseract failed: $(cat "$2.ocr")"
  grep -qi "$3" <<<"$text" || fail "$1 shows no '$3': $text"
}
