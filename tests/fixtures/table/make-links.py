"""Writes hyperlink fixtures; one location carries its own '#'."""
import os
import sys
import zipfile

NS = 'xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"'
RNS = 'xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"'
REL = "http://schemas.openxmlformats.org/officeDocument/2006/relationships/"
HEAD = '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>\n'
TARGET = "历史严重违法失信"


def rels(*links):
    body = "".join(
        f'<Relationship Id="rId{n}" Type="{REL}{kind}" Target="{target}"{mode}/>'
        for n, (kind, target, mode) in enumerate(links, 1)
    )
    return HEAD + f'<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">{body}</Relationships>'


def text(ref, value):
    return f'<c r="{ref}" t="inlineStr"><is><t>{value}</t></is></c>'


LIST = HEAD + (
    f"<worksheet {NS} {RNS}><sheetData>"
    f'<row r="1">{text("A1", "失信名单")}</row>'
    f'<row r="2">{text("A2", "目标格")}</row>'
    f'<row r="3">{text("A3", "网站")}</row>'
    "</sheetData><hyperlinks>"
    f'<hyperlink ref="A1" location="#&apos;{TARGET}&apos;!A1" display="失信名单"/>'
    f'<hyperlink ref="A2" location="&apos;{TARGET}&apos;!B2" display="目标格"/>'
    '<hyperlink ref="A3" r:id="rId1" display="网站"/>'
    "</hyperlinks></worksheet>"
)
GOAL = HEAD + f'<worksheet {NS}><sheetData><row r="2">{text("B2", "到了")}</row></sheetData></worksheet>'
OVERRIDE = "application/vnd.openxmlformats-officedocument.spreadsheetml."
TYPES = [("/xl/workbook.xml", "sheet.main+xml"), ("/xl/worksheets/sheet1.xml", "worksheet+xml"),
         ("/xl/worksheets/sheet2.xml", "worksheet+xml")]
PARTS = {
    "[Content_Types].xml": HEAD + '<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">'
    '<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>'
    '<Default Extension="xml" ContentType="application/xml"/>'
    + "".join(f'<Override PartName="{part}" ContentType="{OVERRIDE}{kind}"/>' for part, kind in TYPES) + "</Types>",
    "_rels/.rels": rels(("officeDocument", "xl/workbook.xml", "")),
    "xl/workbook.xml": HEAD + f'<workbook {NS} {RNS}><sheets><sheet name="清单" sheetId="1" r:id="rId1"/>'
    f'<sheet name="{TARGET}" sheetId="2" r:id="rId2"/></sheets></workbook>',
    "xl/_rels/workbook.xml.rels": rels(("worksheet", "worksheets/sheet1.xml", ""), ("worksheet", "worksheets/sheet2.xml", "")),
    "xl/worksheets/sheet1.xml": LIST,
    "xl/worksheets/_rels/sheet1.xml.rels": rels(("hyperlink", "https://example.com/", ' TargetMode="External"')),
    "xl/worksheets/sheet2.xml": GOAL,
}
with zipfile.ZipFile(os.path.join(sys.argv[1], "links.xlsx"), "w", zipfile.ZIP_DEFLATED) as out:
    for part, body in PARTS.items():
        info = zipfile.ZipInfo(part, (2026, 11, 2, 0, 0, 0))
        info.compress_type = zipfile.ZIP_DEFLATED
        out.writestr(info, body.encode("utf-8"))
