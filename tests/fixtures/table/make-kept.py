"""Writes tests/fixtures/table/kept.xlsx as Excel would save it."""
import sys
import zipfile

FILES = {
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
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData><row r="1">"""
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
    """</row></sheetData></worksheet>""",
}

with zipfile.ZipFile(sys.argv[1], "w", zipfile.ZIP_DEFLATED) as out:
    for name, text in FILES.items():
        info = zipfile.ZipInfo(name, (2026, 10, 28, 0, 0, 0))
        info.compress_type = zipfile.ZIP_DEFLATED
        out.writestr(info, text)
