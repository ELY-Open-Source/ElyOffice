"""Writes kept.xlsx and kept-more.xlsx as Excel would save them."""
import os
import sys
import zipfile

KEPT = (
    """<row r="1">"""
    """<c r="A1"><v>2</v></c>"""
    """<c r="B1"><f>CUBEVALUE("Sales","[Measures].[Total]")</f><v>41</v></c>"""
    """<c r="C1"><f>B1+1</f><v>42</v></c>"""
    """<c r="D1"><f>A1*CUBEVALUE("Sales","[Measures].[Total]")</f><v>82</v></c>"""
    """<c r="E1" t="str"><f>_xlfn.TRANSLATE("hello","en","de")</f><v>hallo</v></c>"""
    """<c r="F1"><f>SUM(A1,1)</f><v>999</v></c>"""
    """</row><row r="2">"""
    """<c r="A2"><v>10</v></c>"""
    """<c r="B2"><f t="shared" ref="B2:B3" si="0">CUBEVALUE("Sales",A2)</f><v>100</v></c>"""
    """</row><row r="3">"""
    """<c r="A3"><v>20</v></c>"""
    """<c r="B3"><f t="shared" si="0"/><v>200</v></c>"""
    """</row>"""
)

# Booleans, empty and missing results, a bad number, arrays, volatile inputs.
MORE = (
    """<row r="1">"""
    """<c r="A1" t="b"><f>FUTUREFN(1)</f><v>1</v></c>"""
    """<c r="B1" t="str"><f>FUTUREFN(2)</f><v/></c>"""
    """<c r="C1" t="str"><f>FUTUREFN(3)</f></c>"""
    """<c r="D1"><f>FUTUREFN(4)</f><v>not-a-number</v></c>"""
    """<c r="E1"><f t="array" ref="E1">CUBEVALUE("Sales","[Measures].[Total]")</f><v>41</v></c>"""
    """<c r="J1" t="b"><f t="array" ref="J1">FUTUREFN(9)</f><v>1</v></c>"""
    """</row><row r="2">"""
    """<c r="A2" t="b"><f t="shared" ref="A2:A3" si="0">FUTUREFN(I2)</f><v>1</v></c>"""
    """<c r="B2" t="str"><f t="shared" ref="B2:B3" si="1">FUTUREFN(I2)</f><v/></c>"""
    """</row><row r="3">"""
    """<c r="A3" t="b"><f t="shared" si="0"/><v>0</v></c>"""
    """<c r="B3" t="str"><f t="shared" si="1"/><v>ok</v></c>"""
    """</row><row r="5">"""
    """<c r="G5"><f t="array" ref="G5:H6">FUTUREFN(1)</f><v>1</v></c>"""
    """<c r="H5"><v>2</v></c>"""
    """</row><row r="6">"""
    """<c r="G6"><v>3</v></c>"""
    """<c r="H6" t="str"><v>four</v></c>"""
    """</row><row r="8">"""
    """<c r="J8"><v>2</v></c>"""
    """<c r="K8"><f>FUTUREFN(INDIRECT("J8"))</f><v>42</v></c>"""
    """<c r="L8"><f>FUTUREFN(OFFSET(J8,0,0))</f><v>43</v></c>"""
    # Array members: booleans, an empty string, bad numbers, a mix.
    """</row><row r="10">"""
    """<c r="G10" t="b"><f t="array" ref="G10:H10">FUTUREFN(5)</f><v>1</v></c>"""
    """<c r="H10" t="b"><v>0</v></c>"""
    """</row><row r="11">"""
    """<c r="G11"><f t="array" ref="G11:H11">FUTUREFN(6)</f><v>41</v></c>"""
    """<c r="H11" t="str"><v/></c>"""
    """</row><row r="12">"""
    """<c r="G12"><f t="array" ref="G12:H12">FUTUREFN(7)</f><v>41</v></c>"""
    """<c r="H12"><v>not-a-number</v></c>"""
    """</row><row r="13">"""
    """<c r="G13"><f t="array" ref="G13:H13">FUTUREFN(8)</f><v>41</v></c>"""
    """<c r="H13"><v>42garbage</v></c>"""
    """</row><row r="14">"""
    """<c r="G14" t="b"><f t="array" ref="G14:H14">FUTUREFN(10)</f><v>1</v></c>"""
    """<c r="H14"><v>41</v></c>"""
    """</row><row r="15">"""
    """<c r="G15"><f t="array" ref="G15:H15">FUTUREFN(11)</f><v>41</v></c>"""
    """<c r="H15" t="b"><v>1</v></c>"""
    """</row>"""
)

# ODS as Ely writes it, the string value moved before the formula.
ODS = (
    """<?xml version="1.0" encoding="UTF-8"?>\n"""
    """<office:document-content xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" """
    """xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0" """
    """xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" """
    """xmlns:of="urn:oasis:names:tc:opendocument:xmlns:of:1.2" """
    """xmlns:calcext="urn:org:documentfoundation:names:experimental:calc:xmlns:calcext:1.0" office:version="1.3">"""
    """<office:body><office:spreadsheet><table:table table:name="Sheet1">"""
    """<table:table-row>"""
    """<table:table-cell office:string-value="" office:value-type="string" table:formula="of:=FUTUREFN(2)" calcext:value-type="string"><text:p/></table:table-cell>"""
    """<table:table-cell table:number-matrix-columns-spanned="2" table:number-matrix-rows-spanned="1" """
    """table:formula="of:=FUTUREFN(1)" office:value-type="float" office:value="41"><text:p>41</text:p></table:table-cell>"""
    """<table:table-cell office:value-type="string" office:string-value="" calcext:value-type="string"><text:p/></table:table-cell>"""
    """</table:table-row><table:table-row>"""
    """<table:table-cell/>"""
    """<table:table-cell table:number-matrix-columns-spanned="2" table:number-matrix-rows-spanned="1" """
    """table:formula="of:=FUTUREFN(3)" office:value-type="float" office:value="41"><text:p>41</text:p></table:table-cell>"""
    """<table:table-cell office:value-type="float" office:value="not-a-number"><text:p>0</text:p></table:table-cell>"""
    """</table:table-row><table:table-row>"""
    """<table:table-cell/>"""
    """<table:table-cell table:formula="of:=FUTUREFN(5)" office:value-type="float" office:value="42garbage"><text:p>42</text:p></table:table-cell>"""
    """</table:table-row><table:table-row>"""
    """<table:table-cell/>"""
    """<table:table-cell table:number-matrix-columns-spanned="1" table:number-matrix-rows-spanned="1" """
    """table:formula="of:=FUTUREFN(6)"><text:p/></table:table-cell>"""
    """</table:table-row><table:table-row>"""
    """<table:table-cell/>"""
    """<table:table-cell table:number-matrix-columns-spanned="2" table:number-matrix-rows-spanned="1" """
    """table:formula="of:=FUTUREFN(7)" office:value-type="float" office:value="41"><text:p>41</text:p></table:table-cell>"""
    """<table:table-cell office:value-type="string" office:string-value="" calcext:value-type="string"/>"""
    """</table:table-row>"""
    """</table:table></office:spreadsheet></office:body></office:document-content>"""
)
MANIFEST = (
    """<?xml version="1.0" encoding="UTF-8"?>\n"""
    """<manifest:manifest xmlns:manifest="urn:oasis:names:tc:opendocument:xmlns:manifest:1.0" manifest:version="1.3">"""
    """<manifest:file-entry manifest:full-path="/" manifest:media-type="application/vnd.oasis.opendocument.spreadsheet"/>"""
    """<manifest:file-entry manifest:full-path="content.xml" manifest:media-type="text/xml"/>"""
    """</manifest:manifest>"""
)


def files(rows):
    return {
        "[Content_Types].xml": """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/><Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/><Override PartName="/docProps/app.xml" ContentType="application/vnd.openxmlformats-officedocument.extended-properties+xml"/></Types>""",
        "_rels/.rels": """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/extended-properties" Target="docProps/app.xml"/></Relationships>""",
        "docProps/app.xml": """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Properties xmlns="http://schemas.openxmlformats.org/officeDocument/2006/extended-properties"><Application>Microsoft Excel</Application><AppVersion>16.0300</AppVersion></Properties>""",
        "xl/workbook.xml": """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets><sheet name="Sheet1" sheetId="1" r:id="rId1"/></sheets><calcPr calcId="191029"/></workbook>""",
        "xl/_rels/workbook.xml.rels": """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>""",
        "xl/worksheets/sheet1.xml": """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData>""" + rows + """</sheetData></worksheet>""",
    }


with zipfile.ZipFile(os.path.join(sys.argv[1], "kept-more.ods"), "w") as out:
    for part, text, kind in [("mimetype", "application/vnd.oasis.opendocument.spreadsheet", zipfile.ZIP_STORED),
                             ("content.xml", ODS, zipfile.ZIP_DEFLATED),
                             ("META-INF/manifest.xml", MANIFEST, zipfile.ZIP_DEFLATED)]:
        info = zipfile.ZipInfo(part, (2026, 10, 28, 0, 0, 0))
        info.compress_type = kind
        out.writestr(info, text)

for name, rows in [("kept.xlsx", KEPT), ("kept-more.xlsx", MORE)]:
    with zipfile.ZipFile(os.path.join(sys.argv[1], name), "w", zipfile.ZIP_DEFLATED) as out:
        for part, text in files(rows).items():
            info = zipfile.ZipInfo(part, (2026, 10, 28, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            out.writestr(info, text)
