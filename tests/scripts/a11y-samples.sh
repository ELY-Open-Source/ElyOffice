# Sourced by the name tests.

# Each app, its sample and its page's name.
a11y_apps() {
  cat <<'EOF'
ely-doc|doc.fodt|Document
ely-table|table.fods|Spreadsheet
ely-show|show.fodp|Presentation
ely-pdf|pdf.pdf|PDF document
ely-mail|-|
ely-tree|repo|
ely-mark|mark.md|Document
EOF
}

# Writes the files the apps open into $1.
a11y_samples() {
  local dir=$1
  local ns='xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"'
  cat >"$dir/doc.fodt" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<office:document $ns office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.text">
 <office:body><office:text><text:p>Hello world</text:p></office:text></office:body>
</office:document>
EOF
  cat >"$dir/table.fods" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<office:document $ns xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0" office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.spreadsheet">
 <office:body><office:spreadsheet><table:table table:name="S"><table:table-row><table:table-cell><text:p>Name</text:p></table:table-cell></table:table-row></table:table></office:spreadsheet></office:body>
</office:document>
EOF
  cat >"$dir/show.fodp" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<office:document $ns xmlns:draw="urn:oasis:names:tc:opendocument:xmlns:drawing:1.0" xmlns:svg="urn:oasis:names:tc:opendocument:xmlns:svg-compatible:1.0" office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.presentation">
 <office:body><office:presentation><draw:page draw:name="One"><draw:frame svg:x="2cm" svg:y="2cm" svg:width="10cm" svg:height="3cm"><draw:text-box><text:p>First</text:p></draw:text-box></draw:frame></draw:page></office:presentation></office:body>
</office:document>
EOF
  local native=$dir
  ! command -v cygpath >/dev/null || native=$(cygpath -m "$dir")
  python3 -I - "$native/pdf.pdf" <<'EOF'
import sys
s = b"BT /F1 24 Tf 36 300 Td (Hello PDF) Tj ET"
objects = [b"<< /Type /Catalog /Pages 2 0 R >>", b"<< /Type /Pages /Kids [4 0 R] /Count 1 >>",
           b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
           b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 400 400] /Contents 5 0 R /Resources << /Font << /F1 3 0 R >> >> >>",
           b"<< /Length %d >>\nstream\n" % len(s) + s + b"\nendstream"]
out, offsets = bytearray(b"%PDF-1.4\n"), []
for n, body in enumerate(objects, 1):
    offsets.append(len(out))
    out += b"%d 0 obj\n" % n + body + b"\nendobj\n"
xref = len(out)
out += b"xref\n0 6\n0000000000 65535 f \n" + b"".join(b"%010d 00000 n \n" % at for at in offsets)
out += b"trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n%d\n%%EOF\n" % xref
open(sys.argv[1], "wb").write(out)
EOF
  printf '# Title\n\nA paragraph.\n\n- item\n' >"$dir/mark.md"
  git init -q "$dir/repo"
  printf 'a\n' >"$dir/repo/a.txt"
  git -C "$dir/repo" add .
  git -C "$dir/repo" -c user.name=t -c user.email=t@t commit -qm one
  printf 'b\n' >>"$dir/repo/a.txt"
}

# Text reader samples: ElyDoc's paragraph, ElyPDF's pages, into $1.
a11y_text_samples() {
  local dir=$1
  cat >"$dir/probe.fodt" <<'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
 xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.text">
 <office:body><office:text><text:p>Hello 世界 😀 end</text:p></office:text></office:body>
</office:document>
EOF

  # A tall page, then three short ones.
  "$(command -v python3 || command -v python)" -I - "$(cygpath -w "$dir" 2>/dev/null || echo "$dir")" <<'EOF'
import os, sys
def line(y, text):
    return b"BT /F1 24 Tf 36 %d Td (%s) Tj ET " % (y, text)
def write(name, size, streams):
    kids = b" ".join(b"%d 0 R" % (4 + 2 * n) for n in range(len(streams)))
    objects = [
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [%s] /Count %d >>" % (kids, len(streams)),
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
    ]
    for n, stream in enumerate(streams):
        objects.append(b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 %d %d] /Contents %d 0 R /Resources << /Font << /F1 3 0 R >> >> >>" % (size[0], size[1], 5 + 2 * n))
        objects.append(b"<< /Length %d >>\nstream\n" % len(stream) + stream + b"\nendstream")
    out, offsets = bytearray(b"%PDF-1.4\n"), []
    for n, body in enumerate(objects, 1):
        offsets.append(len(out))
        out += b"%d 0 obj\n" % n + body + b"\nendobj\n"
    xref = len(out)
    out += b"xref\n0 %d\n0000000000 65535 f \n" % (len(objects) + 1)
    out += b"".join(b"%010d 00000 n \n" % at for at in offsets)
    out += b"trailer\n<< /Size %d /Root 1 0 R >>\nstartxref\n%d\n%%EOF\n" % (len(objects) + 1, xref)
    open(os.path.join(sys.argv[1], name), "wb").write(out)
write("probe.pdf", (300, 1000), [line(940, b"Hello PDF reader") + line(100, b"second line")])
write("pages.pdf", (300, 120), [line(50, b"page " + word) for word in (b"one", b"two", b"three")])
EOF
}
