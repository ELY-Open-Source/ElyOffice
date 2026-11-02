"""Writes #49's packages and small Word structure samples."""
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
# Titled TOC in its content control; the field ends alone.
package("toc.docx", '<w:sdt><w:sdtPr><w:docPartObj><w:docPartGallery w:val="Table of Contents"/><w:docPartUnique/></w:docPartObj></w:sdtPr><w:sdtContent><w:p><w:pPr><w:pStyle w:val="TOCHeading"/></w:pPr><w:r><w:t>Contents</w:t></w:r></w:p><w:p><w:pPr><w:pStyle w:val="TOC1"/></w:pPr><w:r><w:fldChar w:fldCharType="begin"/></w:r><w:r><w:instrText xml:space="preserve"> TOC \\o "1-3" \\h \\z \\u </w:instrText></w:r><w:r><w:fldChar w:fldCharType="separate"/></w:r><w:r><w:t>Alpha</w:t></w:r><w:r><w:tab/><w:t>1</w:t></w:r></w:p><w:p><w:pPr><w:pStyle w:val="TOC1"/></w:pPr><w:r><w:t>Beta</w:t></w:r><w:r><w:tab/><w:t>1</w:t></w:r></w:p><w:p><w:r><w:fldChar w:fldCharType="end"/></w:r></w:p></w:sdtContent></w:sdt><w:p><w:pPr><w:pStyle w:val="Heading1"/></w:pPr><w:r><w:t>Alpha</w:t></w:r></w:p><w:p><w:pPr><w:pStyle w:val="Heading1"/></w:pPr><w:r><w:t>Beta</w:t></w:r></w:p>')

package("redline.docx", '<w:p><w:bookmarkStart w:id="0" w:name="Target"/><w:r><w:t>Target</w:t></w:r><w:bookmarkEnd w:id="0"/></w:p><w:p><w:r><w:t xml:space="preserve">See page </w:t></w:r><w:r><w:fldChar w:fldCharType="begin"/></w:r><w:r><w:instrText xml:space="preserve"> PAGEREF Target \\h </w:instrText></w:r><w:r><w:fldChar w:fldCharType="separate"/></w:r><w:r><w:t>1</w:t></w:r><w:r><w:fldChar w:fldCharType="end"/></w:r></w:p><w:p><w:r><w:t xml:space="preserve">Kept </w:t></w:r><w:ins w:id="1" w:author="Word" w:date="2026-01-01T00:00:00Z"><w:r><w:t>added</w:t></w:r></w:ins><w:del w:id="2" w:author="Word" w:date="2026-01-01T00:00:00Z"><w:r><w:delText>gone</w:delText></w:r></w:del></w:p>')
