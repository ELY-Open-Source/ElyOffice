"""Benchmark samples, made the same way every time (§20.1)."""

import hashlib
import os
import random
import struct
import subprocess
import zipfile
import zlib

import child
import sheets


def noise_png(seed, width, height):
    """An RGB PNG of seeded noise; it does not compress."""
    rng = random.Random(seed)
    rows = b"".join(b"\0" + rng.randbytes(width * 3) for _ in range(height))

    def chunk(kind, body):
        return struct.pack(">I", len(body)) + kind + body + struct.pack(">I", zlib.crc32(kind + body))

    head = struct.pack(">IIBBBBB", width, height, 8, 2, 0, 0, 0)
    return b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", head) + chunk(b"IDAT", zlib.compress(rows, 1)) + chunk(b"IEND", b"")


W = 'xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" ' \
    'xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" ' \
    'xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing" ' \
    'xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" ' \
    'xmlns:pic="http://schemas.openxmlformats.org/drawingml/2006/picture"'


def picture(n):
    cx, cy = 3600000, 2700000
    return (f'<w:p><w:r><w:drawing><wp:inline><wp:extent cx="{cx}" cy="{cy}"/><wp:docPr id="{n}" name="p{n}"/>'
            '<a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/picture">'
            f'<pic:pic><pic:nvPicPr><pic:cNvPr id="{n}" name="p{n}.png"/><pic:cNvPicPr/></pic:nvPicPr>'
            f'<pic:blipFill><a:blip r:embed="img{n}"/><a:stretch><a:fillRect/></a:stretch></pic:blipFill>'
            f'<pic:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="{cx}" cy="{cy}"/></a:xfrm>'
            '<a:prstGeom prst="rect"><a:avLst/></a:prstGeom></pic:spPr></pic:pic></a:graphicData></a:graphic>'
            '</wp:inline></w:drawing></w:r></w:p>')


def docx(path):
    """PERF-03: 100 pages, 20 images, under 10 MB."""
    rng = random.Random(3)
    words = "office local file engine page table slide note draft review".split()
    body = []
    for page in range(100):
        # The break runs in the heading; Heading 1 drops pageBreakBefore.
        breaks = '<w:r><w:br w:type="page"/></w:r>' if page else ""
        body.append(f'<w:p><w:pPr><w:pStyle w:val="Heading1"/></w:pPr>{breaks}<w:r><w:t>Page {page + 1}</w:t></w:r></w:p>')
        for _ in range(6):
            text = " ".join(rng.choice(words) for _ in range(60))
            body.append(f"<w:p><w:r><w:t>{text}</w:t></w:r></w:p>")
        if page % 5 == 0:
            body.append(picture(page // 5 + 1))
    rels = "".join(f'<Relationship Id="img{n}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" '
                   f'Target="media/p{n}.png"/>' for n in range(1, 21))
    with zipfile.ZipFile(path, "w", zipfile.ZIP_DEFLATED) as archive:
        archive.writestr("[Content_Types].xml", '<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">'
                         '<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>'
                         '<Default Extension="xml" ContentType="application/xml"/><Default Extension="png" ContentType="image/png"/>'
                         '<Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/></Types>')
        archive.writestr("_rels/.rels", '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">'
                         '<Relationship Id="r1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>')
        archive.writestr("word/_rels/document.xml.rels", f'<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">{rels}</Relationships>')
        archive.writestr("word/document.xml", f'<w:document {W}><w:body>{"".join(body)}</w:body></w:document>')
        for n in range(1, 21):
            archive.writestr(f"word/media/p{n}.png", noise_png(n, 400, 300), zipfile.ZIP_STORED)


def xlsx(path):
    """PERF-04: 100,000 values and 10,000 simple formulas."""
    rows = []
    for r in range(1, 10001):
        cells = "".join(f'<c r="{chr(65 + c)}{r}"><v>{(r * 7 + c * 13) % 1000}</v></c>' for c in range(10))
        rows.append(f'<row r="{r}">{cells}<c r="K{r}"><f>SUM(A{r}:J{r})</f></c></row>')
    ns = 'xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" ' \
         'xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"'
    with zipfile.ZipFile(path, "w", zipfile.ZIP_DEFLATED) as archive:
        archive.writestr("[Content_Types].xml", '<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">'
                         '<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>'
                         '<Default Extension="xml" ContentType="application/xml"/>'
                         '<Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>'
                         '<Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/></Types>')
        archive.writestr("_rels/.rels", '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">'
                         '<Relationship Id="r1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>')
        archive.writestr("xl/_rels/workbook.xml.rels", '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">'
                         '<Relationship Id="s1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>')
        archive.writestr("xl/workbook.xml", f'<workbook {ns}><sheets><sheet name="Data" sheetId="1" r:id="s1"/></sheets></workbook>')
        archive.writestr("xl/worksheets/sheet1.xml", f'<worksheet {ns}><sheetData>{"".join(rows)}</sheetData></worksheet>')


def pdf(path):
    """PERF-05: 1000 pages, about 200 MB, one image a page."""
    rng = random.Random(5)
    offsets, out = [], open(path, "wb")
    out.write(b"%PDF-1.7\n")

    def obj(n, body):
        offsets.append((n, out.tell()))
        out.write(f"{n} 0 obj\n".encode() + body + b"\nendobj\n")

    pages = 1000
    kids = " ".join(f"{3 + i * 3} 0 R" for i in range(pages))
    obj(1, b"<< /Type /Catalog /Pages 2 0 R >>")
    obj(2, f"<< /Type /Pages /Kids [{kids}] /Count {pages} >>".encode())
    for i in range(pages):
        page, content, image = 3 + i * 3, 4 + i * 3, 5 + i * 3
        pixels = rng.randbytes(240 * 280 * 3)
        obj(page, f"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents {content} 0 R "
                  f"/Resources << /XObject << /I {image} 0 R >> /Font << /F <</Type /Font /Subtype /Type1 /BaseFont /Helvetica>> >> >> >>".encode())
        draw = f"q 480 0 0 560 66 160 cm /I Do Q BT /F 24 Tf 72 100 Td (Page {i + 1}) Tj ET".encode()
        obj(content, f"<< /Length {len(draw)} >>\nstream\n".encode() + draw + b"\nendstream")
        obj(image, f"<< /Type /XObject /Subtype /Image /Width 240 /Height 280 /ColorSpace /DeviceRGB "
                   f"/BitsPerComponent 8 /Length {len(pixels)} >>\nstream\n".encode() + pixels + b"\nendstream")
    xref = out.tell()
    count = 3 + pages * 3
    places = dict(offsets)
    out.write(f"xref\n0 {count}\n0000000000 65535 f \n".encode())
    for n in range(1, count):
        out.write(f"{places[n]:010d} 00000 n \n".encode())
    out.write(f"trailer\n<< /Size {count} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n".encode())
    out.close()


def repo(path, git):
    """PERF-06: a repository of 100,000 commits."""
    subprocess.run([git, "init", "-q", "-b", "main", path], check=True)
    stream = []
    for n in range(1, 100001):
        message = f"Change {n}: {'fix' if n % 3 else 'add'} part {n % 97}\n".encode()
        body = f"line {n}\n".encode()
        stream.append(b"commit refs/heads/main\n")
        stream.append(f"committer Bench <bench@ely.local> {1700000000 + n} +0000\n".encode())
        stream.append(f"data {len(message)}\n".encode() + message)
        stream.append(f"M 644 inline part{n % 97}.txt\ndata {len(body)}\n".encode() + body)
    subprocess.run([git, "-C", path, "fast-import", "--quiet"], input=b"".join(stream), check=True)
    subprocess.run([git, "-C", path, "checkout", "-q", "main"], check=True)


def markdown(path):
    """PERF-07: a 1 MB Markdown document."""
    rng = random.Random(7)
    words = "local file engine page table slide note draft review commit".split()
    parts, size, n = [], 0, 0
    while size < 1024 * 1024:
        n += 1
        block = f"## Section {n}\n\n" + " ".join(rng.choice(words) for _ in range(120)) + \
                f"\n\n- item {n}\n- **bold** and `code`\n\n```rust\nfn f{n}() -> u32 {{ {n} }}\n```\n\n"
        parts.append(block)
        size += len(block)
    with open(path, "w") as out:
        out.write("".join(parts))


def made(folder, git):
    """Each sample, made once, with its SHA-256."""
    os.makedirs(folder, exist_ok=True)
    makers = {"doc.docx": docx, "sheet.xlsx": xlsx, "pages.pdf": pdf, "notes.md": markdown, **sheets.MAKERS}
    found = {}
    for name, make in makers.items():
        path = os.path.join(folder, name)
        if not os.path.exists(path):
            make(path)
        found[name] = (path, hashlib.sha256(open(path, "rb").read()).hexdigest())
    path = os.path.join(folder, "repo")
    if not os.path.exists(path):
        repo(path, git)
    head = child.run([git, "-C", path, "rev-parse", "HEAD"], capture_output=True, check=True).stdout.strip()
    found["repo"] = (path, head)
    return found
