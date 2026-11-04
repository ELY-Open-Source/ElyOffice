"""Writes compact pivot fixtures as Excel saves them."""
import os
import sys
import zipfile

NS = 'xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"'
RNS = 'xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"'
REL = "http://schemas.openxmlformats.org/officeDocument/2006/relationships/"
HEAD = '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>\n'
DATA = [("East", "A", 1), ("East", "B", 2), ("West", "A", 3), ("West", "B", 4)]


def rels(*links):
    body = "".join(f'<Relationship Id="rId{n}" Type="{REL}{kind}" Target="{target}"/>' for n, (kind, target) in enumerate(links, 1))
    return HEAD + f'<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">{body}</Relationships>'


def text(ref, value):
    return f'<c r="{ref}" t="inlineStr"><is><t>{value}</t></is></c>'


def number(ref, value):
    return f'<c r="{ref}"><v>{value}</v></c>'


def cross(row_label, col_label, total):
    """Excel's cached table in E3:H7; blank captions have no cell."""
    return [("E3", "Sum of Revenue"), ("F3", col_label), ("E4", row_label), ("F4", "A"), ("G4", "B"),
            ("H4", total), ("E5", "East"), ("F5", 1), ("G5", 2), ("H5", 3), ("E6", "West"), ("F6", 3),
            ("G6", 4), ("H6", 7), ("E7", total), ("F7", 4), ("G7", 6), ("H7", 10)]


def columns(col_label, total, name="Sum of Revenue"):
    """Excel's cached table in E3:H5, with no row field."""
    return [("F3", col_label), ("F4", "A"), ("G4", "B"), ("H4", total), ("E5", name),
            ("F5", 4), ("G5", 6), ("H5", 10)]


def empty(col_label, total):
    """Excel's E3:G4 with no value field; H4 is the user's."""
    return [("E3", col_label), ("E4", "A"), ("F4", "B"), ("G4", total), ("H4", "NEIGHBOR")]


def values(col_label, total, caption="Values"):
    """Excel's cached table in E3:H6, two values on rows."""
    return [("F3", col_label), ("E4", caption), ("F4", "A"), ("G4", "B"), ("H4", total),
            ("E5", "Sum of Revenue"), ("F5", 4), ("G5", 6), ("H5", 10),
            ("E6", "Count revenue"), ("F6", 2), ("G6", 2), ("H6", 4)]


def sheet(cached):
    """The source in A1:C5 and the cached table."""
    rows = {1: [text("A1", "Region"), text("B1", "Product"), text("C1", "Revenue")]}
    for n, (region, product, revenue) in enumerate(DATA, 2):
        rows[n] = [text(f"A{n}", region), text(f"B{n}", product), number(f"C{n}", revenue)]
    for ref, value in cached:
        if value != "":
            cell = number(ref, value) if isinstance(value, int) else text(ref, value)
            rows.setdefault(int(ref[1:]), []).append(cell)
    body = "".join(f'<row r="{n}">{"".join(cells)}</row>' for n, cells in sorted(rows.items()))
    return HEAD + f"<worksheet {NS}><sheetData>{body}</sheetData></worksheet>"


CACHE = HEAD + (
    f'<pivotCacheDefinition {NS} {RNS} r:id="rId1" refreshedBy="Ely" refreshedDate="46000" createdVersion="8" '
    'refreshedVersion="8" minRefreshableVersion="3" recordCount="4">'
    '<cacheSource type="worksheet"><worksheetSource ref="A1:C5" sheet="Sheet1"/></cacheSource>'
    '<cacheFields count="3">'
    '<cacheField name="Region" numFmtId="0"><sharedItems count="2"><s v="East"/><s v="West"/></sharedItems></cacheField>'
    '<cacheField name="Product" numFmtId="0"><sharedItems count="2"><s v="A"/><s v="B"/></sharedItems></cacheField>'
    '<cacheField name="Revenue" numFmtId="0"><sharedItems containsSemiMixedTypes="0" containsString="0" '
    'containsNumber="1" containsInteger="1" minValue="1" maxValue="4"/></cacheField>'
    "</cacheFields></pivotCacheDefinition>"
)
RECORDS = HEAD + f'<pivotCacheRecords {NS} count="4">' + "".join(
    f'<r><x v="{0 if region == "East" else 1}"/><x v="{0 if product == "A" else 1}"/><n v="{revenue}"/></r>'
    for region, product, revenue in DATA) + "</pivotCacheRecords>"


SUM = '<dataField name="Sum of Revenue" fld="2" baseField="0" baseItem="0"/>'
COUNT = '<dataField name="Count revenue" fld="2" subtotal="count" baseField="0" baseItem="0"/>'


def table(captions, ref="E3:H7", name="Sum of Revenue", caption="Values"):
    two = '<items count="3"><item x="0"/><item x="1"/><item t="default"/></items>'
    lines = '<i><x/></i><i><x v="1"/></i><i t="grand"><x/></i>'
    region = '<pivotField axis="axisRow" showAll="0">' if ref == "E3:H7" else '<pivotField showAll="0">'
    rows = {"E3:H7": f'<rowFields count="1"><field x="0"/></rowFields><rowItems count="3">{lines}</rowItems>',
            "E3:H5": '<rowItems count="1"><i/></rowItems>',
            "E3:G4": '<rowItems count="1"><i/></rowItems>',
            "E3:H6": '<rowFields count="1"><field x="-2"/></rowFields>'
                     '<rowItems count="2"><i><x/></i><i i="1"><x v="1"/></i></rowItems>'}[ref]
    one = f'<dataField name="{name}" fld="2" baseField="0" baseItem="0"/>'
    data = {"E3:H6": f'<dataFields count="2">{SUM}{COUNT}</dataFields>', "E3:G4": ""}.get(ref, f'<dataFields count="1">{one}</dataFields>')
    revenue = '<pivotField showAll="0"/>' if ref == "E3:G4" else '<pivotField dataField="1" showAll="0"/>'
    first = "0" if ref == "E3:G4" else "1"
    on_rows = ' dataOnRows="1"' if ref == "E3:H6" else ""
    return HEAD + (
        f'<pivotTableDefinition {NS} name="PivotTable1" cacheId="1"{on_rows} applyNumberFormats="0" applyBorderFormats="0" '
        'applyFontFormats="0" applyPatternFormats="0" applyAlignmentFormats="0" applyWidthHeightFormats="1" '
        f'dataCaption="{caption}"{captions} updatedVersion="8" minRefreshableVersion="3" useAutoFormatting="1" '
        'itemPrintTitles="1" createdVersion="8" indent="0" outline="1" outlineData="1" multipleFieldFilters="0">'
        f'<location ref="{ref}" firstHeaderRow="1" firstDataRow="2" firstDataCol="{first}"/>'
        f'<pivotFields count="3">{region}{two}</pivotField>'
        f'<pivotField axis="axisCol" showAll="0">{two}</pivotField>{revenue}</pivotFields>'
        f'{rows}<colFields count="1"><field x="1"/></colFields><colItems count="3">{lines}</colItems>'
        f'{data}'
        '<pivotTableStyleInfo name="PivotStyleLight16" showRowHeaders="1" showColHeaders="1" showRowStripes="0" '
        'showColStripes="0" showLastColumn="1"/></pivotTableDefinition>'
    )


def files(captions, cached, ref="E3:H7", name="Sum of Revenue", caption="Values"):
    override = "application/vnd.openxmlformats-officedocument.spreadsheetml."
    types = [("/xl/workbook.xml", "sheet.main+xml"), ("/xl/worksheets/sheet1.xml", "worksheet+xml"),
             ("/xl/pivotCache/pivotCacheDefinition1.xml", "pivotCacheDefinition+xml"),
             ("/xl/pivotCache/pivotCacheRecords1.xml", "pivotCacheRecords+xml"),
             ("/xl/pivotTables/pivotTable1.xml", "pivotTable+xml")]
    return {
        "[Content_Types].xml": HEAD + '<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">'
        '<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>'
        '<Default Extension="xml" ContentType="application/xml"/>'
        + "".join(f'<Override PartName="{part}" ContentType="{override}{kind}"/>' for part, kind in types) + "</Types>",
        "_rels/.rels": rels(("officeDocument", "xl/workbook.xml")),
        "xl/workbook.xml": HEAD + f'<workbook {NS} {RNS}><sheets><sheet name="Sheet1" sheetId="1" r:id="rId1"/></sheets>'
        '<pivotCaches><pivotCache cacheId="1" r:id="rId2"/></pivotCaches></workbook>',
        "xl/_rels/workbook.xml.rels": rels(("worksheet", "worksheets/sheet1.xml"), ("pivotCacheDefinition", "pivotCache/pivotCacheDefinition1.xml")),
        "xl/worksheets/sheet1.xml": sheet(cached),
        "xl/worksheets/_rels/sheet1.xml.rels": rels(("pivotTable", "../pivotTables/pivotTable1.xml")),
        "xl/pivotCache/pivotCacheDefinition1.xml": CACHE,
        "xl/pivotCache/_rels/pivotCacheDefinition1.xml.rels": rels(("pivotCacheRecords", "pivotCacheRecords1.xml")),
        "xl/pivotCache/pivotCacheRecords1.xml": RECORDS,
        "xl/pivotTables/pivotTable1.xml": table(captions, ref, name, caption),
        "xl/pivotTables/_rels/pivotTable1.xml.rels": rels(("pivotCacheDefinition", "../pivotCache/pivotCacheDefinition1.xml")),
    }


NAMED = ' grandTotalCaption="总计" rowHeaderCaption="行标签" colHeaderCaption="列标签"'
BLANK = ' rowHeaderCaption="" colHeaderCaption=""'
for name, parts in [("pivot.xlsx", files("", cross("Row Labels", "Column Labels", "Grand Total"))),
                    ("pivot-named.xlsx", files(NAMED, cross("行标签", "列标签", "总计"))),
                    ("pivot-blank.xlsx", files(BLANK, cross("", "", "Grand Total"))),
                    ("pivot-columns.xlsx", files(NAMED, columns("列标签", "总计"), "E3:H5")),
                    ("pivot-values.xlsx", files(NAMED, values("列标签", "总计"), "E3:H6")),
                    ("pivot-values-named.xlsx", files(NAMED, values("列标签", "总计", "数值"), "E3:H6", caption="数值")),
                    ("pivot-empty.xlsx", files(NAMED, empty("列标签", "总计"), "E3:G4"))] + [
                    # Value names that look like input stay text.
                    (f"pivot-name-{n}.xlsx", files(NAMED, columns("列标签", "总计", name), "E3:H5", name))
                    for n, name in [("formula", "=1+1"), ("quote", "'Revenue"), ("number", "123")]]:
    with zipfile.ZipFile(os.path.join(sys.argv[1], name), "w", zipfile.ZIP_DEFLATED) as out:
        for part, body in parts.items():
            info = zipfile.ZipInfo(part, (2026, 11, 1, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            out.writestr(info, body.encode("utf-8"))
