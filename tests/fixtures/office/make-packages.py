"""Writes #49's packages, and an equation with minus signs."""
import os
import sys
import zipfile

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


HEAD = '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
M = 'xmlns:m="http://schemas.openxmlformats.org/officeDocument/2006/math"'
EQUATION = {
    "[Content_Types].xml": HEAD + '<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/></Types>',
    "_rels/.rels": HEAD + '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>',
    # Math, literal, double-struck and normal-text hyphens.
    "word/document.xml": HEAD + '<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" ' + M + '><w:body><w:p><m:oMathPara><m:oMath><m:r><m:t>x</m:t></m:r><m:r><m:t>-</m:t></m:r><m:r><m:t>b-c</m:t></m:r><m:r><m:rPr><m:nor/></m:rPr><m:t>a-b</m:t></m:r><m:r><m:rPr><m:lit/></m:rPr><m:t>1-2</m:t></m:r><m:r><m:rPr><m:scr m:val="double-struck"/></m:rPr><m:t>3-4</m:t></m:r></m:oMath></m:oMathPara></w:p><w:sectPr/></w:body></w:document>',
}
with zipfile.ZipFile(os.path.join(sys.argv[1], "equation.docx"), "w", zipfile.ZIP_DEFLATED) as out:
    for part, text in EQUATION.items():
        out.writestr(zipfile.ZipInfo(part, (2026, 11, 1, 0, 0, 0)), text)
