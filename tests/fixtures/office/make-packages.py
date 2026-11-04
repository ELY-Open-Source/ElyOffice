"""Writes #49's packages and small Word structure samples."""
import os
import struct
import sys
import zipfile
import zlib

HERE = os.path.dirname(os.path.abspath(__file__))
BACKSLASH = chr(92)


def copy(source, target, names):
    with zipfile.ZipFile(os.path.join(HERE, source)) as old, zipfile.ZipFile(os.path.join(sys.argv[1], target), "w") as out:
        for item in old.infolist():
            out.writestr(item, old.read(item.filename))
        for name in names:
            info = zipfile.ZipInfo("x", (2026, 11, 1, 0, 0, 0))
            # Set after: ZipInfo turns the backslash on Windows.
            info.filename = name
            out.writestr(info, b"" if name.endswith("/") else b"<x/>")


# Tencent Docs' folder entries.
copy("t.docx", "backslash.docx", ["word" + BACKSLASH + "theme/", "word" + BACKSLASH + "_rels/"])
copy("t.xlsx", "backslash.xlsx", ["xl" + BACKSLASH + "worksheets/", "xl" + BACKSLASH + "theme/", "xl" + BACKSLASH + "_rels/"])
# No zip name may hold a question mark.
copy("t.docx", "broken.docx", ["word/a?b.xml"])


def cut(source, target, part):
    with zipfile.ZipFile(os.path.join(HERE, source)) as old, zipfile.ZipFile(os.path.join(sys.argv[1], target), "w") as out:
        for item in old.infolist():
            data = old.read(item.filename)
            out.writestr(item, data[: len(data) // 2] if item.filename == part else data)


# The body's XML ends halfway.
cut("t.docx", "malformed.docx", "word/document.xml")


HEAD = '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
M = 'xmlns:m="http://schemas.openxmlformats.org/officeDocument/2006/math"'


def package(name, body):
    parts = {
        "[Content_Types].xml": HEAD + '<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/></Types>',
        "_rels/.rels": HEAD + '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>',
        "word/document.xml": HEAD + '<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" ' + M + '><w:body>' + body + '<w:sectPr/></w:body></w:document>',
    }
    with zipfile.ZipFile(os.path.join(sys.argv[1], name), "w", zipfile.ZIP_DEFLATED) as out:
        for part, text in parts.items():
            out.writestr(zipfile.ZipInfo(part, (2026, 11, 1, 0, 0, 0)), text)


# Math, literal, double-struck and normal-text hyphens.
package("equation.docx", '<w:p><m:oMathPara><m:oMath><m:r><m:t>x</m:t></m:r><m:r><m:t>-</m:t></m:r><m:r><m:t>b-c</m:t></m:r><m:r><m:rPr><m:nor/></m:rPr><m:t>a-b</m:t></m:r><m:r><m:rPr><m:lit/></m:rPr><m:t>1-2</m:t></m:r><m:r><m:rPr><m:scr m:val="double-struck"/></m:rPr><m:t>3-4</m:t></m:r><m:r><m:rPr><m:nor/><m:scr m:val="double-struck"/></m:rPr><m:t>5-6</m:t></m:r><m:r><m:rPr><m:nor/><m:scr m:val="double-struck"/></m:rPr><m:t>"alpha"</m:t></m:r><m:r><m:rPr><m:lit/></m:rPr><m:t>"alpha"</m:t></m:r></m:oMath></m:oMathPara></w:p>')
def toc(name, title, beta, tail, alone=True, control=True):
    """A TOC, in its content control or bare, then two headings."""
    head = '<w:p><w:pPr><w:pStyle w:val="TOCHeading"/></w:pPr><w:r><w:t>Contents</w:t></w:r></w:p>' if title else ""
    entry = '<w:p><w:pPr><w:pStyle w:val="TOC1"/></w:pPr>'
    field = '<w:r><w:fldChar w:fldCharType="begin"/></w:r><w:r><w:instrText xml:space="preserve"> TOC \\o "1-3" \\h \\z \\u </w:instrText></w:r><w:r><w:fldChar w:fldCharType="separate"/></w:r>'
    page = '<w:r><w:tab/><w:t>1</w:t></w:r></w:p>'
    end = ('<w:p>' if alone else '') + '<w:r><w:fldChar w:fldCharType="end"/></w:r>' + tail + '</w:p>'
    gallery = '<w:sdtPr><w:docPartObj><w:docPartGallery w:val="Table of Contents"/><w:docPartUnique/></w:docPartObj></w:sdtPr>'
    headings = "".join(f'<w:p><w:pPr><w:pStyle w:val="Heading1"/></w:pPr><w:r><w:t>{text}</w:t></w:r></w:p>' for text in ("Alpha", "Beta"))
    last = page if alone else page[:-len('</w:p>')]
    body = head + entry + field + '<w:r><w:t>Alpha</w:t></w:r>' + page + entry + beta + last + end
    wrapped = '<w:sdt>' + gallery + '<w:sdtContent>' + body + '</w:sdtContent></w:sdt>' if control else body
    package(name, wrapped + headings)


BETA = '<w:r><w:t>Beta</w:t></w:r>'
# Titled TOC in its content control; the field ends alone.
toc("toc.docx", True, BETA, "")
# Untitled, and a bare field with no content control.
toc("toc-untitled.docx", False, BETA, "")
toc("toc-bare.docx", False, BETA, "", control=False)
# Body text after the field end, titled or not.
toc("toc-tail.docx", True, BETA, '<w:r><w:t>After</w:t></w:r>')
toc("toc-tail-untitled.docx", False, BETA, '<w:r><w:t>After</w:t></w:r>')
# The field ends inside the last entry, before the text.
toc("toc-tail-entry.docx", True, BETA, '<w:r><w:t>After</w:t></w:r>', alone=False)
toc("toc-tail-entry-untitled.docx", False, BETA, '<w:r><w:t>After</w:t></w:r>', alone=False)
# Only a floating text box follows the field end.
SHAPE = '<w:r><w:pict><v:shape xmlns:v="urn:schemas-microsoft-com:vml" id="TailShape" style="position:absolute;margin-left:0;margin-top:0;width:100pt;height:30pt"><v:textbox><w:txbxContent><w:p><w:r><w:t>ObjectTail</w:t></w:r></w:p></w:txbxContent></v:textbox></v:shape></w:pict></w:r>'
toc("toc-shape.docx", True, BETA, SHAPE, alone=False)
toc("toc-shape-untitled.docx", False, BETA, SHAPE, alone=False)
# An entry in its own inline content control.
toc("toc-inline.docx", True, '<w:sdt><w:sdtPr><w:alias w:val="Entry"/></w:sdtPr><w:sdtContent>' + BETA + '</w:sdtContent></w:sdt>', "")

# A Writer TOC; the heading after it is body text.
ODF = 'xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0"'
ENTRIES = "".join(f'<text:p>{text}<text:tab/>1</text:p>' for text in ("Alpha", "Beta"))
SOURCE = '<text:table-of-content-source text:outline-level="3"><text:table-of-content-entry-template text:outline-level="1"><text:index-entry-text/><text:index-entry-tab-stop style:type="right"/><text:index-entry-page-number/></text:table-of-content-entry-template></text:table-of-content-source>'
AFTER = "".join(f'<text:h text:outline-level="1">{h}</text:h><text:p>{p}</text:p>' for h, p in (("Alpha", "Body one."), ("Beta", "Body two.")))
with open(os.path.join(sys.argv[1], "toc-native.fodt"), "w", encoding="utf-8", newline="\n") as native:
    native.write(HEAD + f'<office:document {ODF} office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.text"><office:body><office:text>'
                 f'<text:table-of-content text:name="Table of Contents1">{SOURCE}<text:index-body>{ENTRIES}</text:index-body></text:table-of-content>'
                 f'{AFTER}</office:text></office:body></office:document>\n')

package("redline.docx", '<w:p><w:bookmarkStart w:id="0" w:name="Target"/><w:r><w:t>Target</w:t></w:r><w:bookmarkEnd w:id="0"/></w:p><w:p><w:r><w:t xml:space="preserve">See page </w:t></w:r><w:r><w:fldChar w:fldCharType="begin"/></w:r><w:r><w:instrText xml:space="preserve"> PAGEREF Target \\h </w:instrText></w:r><w:r><w:fldChar w:fldCharType="separate"/></w:r><w:r><w:t>1</w:t></w:r><w:r><w:fldChar w:fldCharType="end"/></w:r></w:p><w:p><w:r><w:t xml:space="preserve">Kept </w:t></w:r><w:ins w:id="1" w:author="Word" w:date="2026-01-01T00:00:00Z"><w:r><w:t>added</w:t></w:r></w:ins><w:del w:id="2" w:author="Word" w:date="2026-01-01T00:00:00Z"><w:r><w:delText>gone</w:delText></w:r></w:del></w:p>')


def field(code, cached):
    return (f'<w:r><w:fldChar w:fldCharType="begin"/></w:r><w:r><w:instrText xml:space="preserve"> {code} </w:instrText></w:r>'
            f'<w:r><w:fldChar w:fldCharType="separate"/></w:r><w:r><w:t>{cached}</w:t></w:r><w:r><w:fldChar w:fldCharType="end"/></w:r>')


# INCLUDETEXT alone, beside other text, and over two paragraphs.
ALONE = field(f'INCLUDETEXT "include.docx" {BACKSLASH}* MERGEFORMAT', "Included before")
BESIDE = field('INCLUDETEXT "include.docx"', "Included before")
TWO = field(f'INCLUDETEXT "include.docx" {BACKSLASH}!', "Line one").replace("</w:t></w:r>", "</w:t></w:r></w:p><w:p><w:r><w:t>Line two</w:t></w:r>", 1)
package("includetext.docx", f'<w:p><w:r><w:t>Before</w:t></w:r></w:p><w:p>{ALONE}</w:p><w:p><w:r><w:t xml:space="preserve">Beside </w:t></w:r>{BESIDE}</w:p><w:p>{TWO}</w:p><w:p><w:r><w:t>After</w:t></w:r></w:p>')


def png(width, height):
    """A grey PNG of `width` by `height`."""
    def chunk(kind, data):
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data))
    rows = b"".join(bytes([0]) + bytes([128]) * width for _ in range(height))
    header = struct.pack(">IIBBBBB", width, height, 8, 0, 0, 0, 0)
    return bytes([137]) + b"PNG\r\n\x1a\n" + chunk(b"IHDR", header) + chunk(b"IDAT", zlib.compress(rows)) + chunk(b"IEND", b"")


# Word's linked objects: a LINK field around Type="Link".
REL = "http://schemas.openxmlformats.org/officeDocument/2006/relationships"
TYPES = '<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Default Extension="png" ContentType="image/png"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/></Types>'

def ole_link(name, target, prog):
    """Word's LINK field around a linked object of kind `prog`."""
    obj = ('<w:r><w:object w:dxaOrig="2000" w:dyaOrig="1000"><v:shape id="_x0000_i1025" type="#_x0000_t75" style="width:100pt;height:50pt" o:ole="">'
           f'<v:imagedata r:id="rPreview" o:title=""/></v:shape><o:OLEObject Type="Link" ProgID="{prog}" ShapeID="_x0000_i1025" '
           'DrawAspect="Content" r:id="rOle" UpdateMode="Always"><o:LinkType>EnhancedMetaFile</o:LinkType><o:LockedField>false</o:LockedField>'
           '</o:OLEObject></w:object></w:r>')
    code = target.replace(BACKSLASH, BACKSLASH * 2)
    link = field(f'LINK {prog} "{code}" "Rates!R1C1:R12C2" {BACKSLASH}a {BACKSLASH}p', "").replace("<w:r><w:t></w:t></w:r>", obj)
    with zipfile.ZipFile(os.path.join(sys.argv[1], name), "w", zipfile.ZIP_DEFLATED) as out:
        for part, data in {
            "[Content_Types].xml": HEAD + TYPES,
            "_rels/.rels": HEAD + f'<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="{REL}/officeDocument" Target="word/document.xml"/></Relationships>',
            "word/_rels/document.xml.rels": HEAD + f'<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rOle" Type="{REL}/oleObject" Target="{target}" TargetMode="External"/><Relationship Id="rPreview" Type="{REL}/image" Target="media/preview.png"/></Relationships>',
            "word/document.xml": HEAD + f'<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:r="{REL}" xmlns:v="urn:schemas-microsoft-com:vml" xmlns:o="urn:schemas-microsoft-com:office:office"><w:body><w:p><w:r><w:t>Before</w:t></w:r></w:p><w:p>{link}</w:p><w:sectPr/></w:body></w:document>',
            "word/media/preview.png": png(40, 20),
        }.items():
            out.writestr(zipfile.ZipInfo(part, (2026, 11, 1, 0, 0, 0)), data)


ole_link("ole-link.docx", "source.xlsx", "Excel.Sheet.12")
ole_link("ole-link-other.docx", "source.pdf", "AcroExch.Document.DC")
# A test's own target, as Word writes it: file:///C:\...
if len(sys.argv) > 2:
    ole_link("ole-link-word.docx", sys.argv[2], "Excel.Sheet.12")
