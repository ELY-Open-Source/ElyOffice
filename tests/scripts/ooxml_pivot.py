"""Checks a saved XLSX pivot's words without LibreOffice."""
import os
import sys
import zipfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import ooxml  # noqa: E402

DEFAULTS = ("Row Labels", "Column Labels", "Grand Total")
NAMED = ("行标签", "列标签", "总计")
ATTRIBUTES = ("rowHeaderCaption", "colHeaderCaption", "grandTotalCaption")


def captions(path, words, written):
    """E3:H7 shows `words`; the definition writes `written` captions."""
    row, col, total = words
    with zipfile.ZipFile(path) as archive:
        found = {ref: value for ref, (_, _, value) in ooxml.cells(archive).items()}
        book = ooxml.main_part(archive)
        part = book and ooxml.first_sheet(archive, book)
        tables = ooxml.related(archive, part, "pivotTable") if part else []
        if not ooxml.check(len(tables) == 1, f"one pivot table on the sheet: {tables}"):
            return
        table = ooxml.xml(archive, tables[0])
    shown = {ref: found.get(ref) or "" for ref in ("E4", "F3", "H4", "E7")}
    ooxml.check(shown == {"E4": row, "F3": col, "H4": total, "E7": total}, f"the table's words: {shown}")
    ooxml.check(found.get("H7") == "10", f"the grand total sums: {found.get('H7')}")
    inside = {found.get(f"{c}{r}") for c in "EFGH" for r in range(3, 8)}
    ooxml.check(not inside & {"Region", "Product", "Total Result"}, f"no Calc words in the table: {inside}")
    got = {name: table.get(name) for name in ATTRIBUTES}
    ooxml.check(got == dict(zip(ATTRIBUTES, written)), f"the captions written: {got}")


def columns(path, last=5):
    """No row field: the caption heads the columns."""
    with zipfile.ZipFile(path) as archive:
        found = {ref: value for ref, (_, _, value) in ooxml.cells(archive).items()}
        book = ooxml.main_part(archive)
        tables = ooxml.related(archive, ooxml.first_sheet(archive, book), "pivotTable")
        table = ooxml.xml(archive, tables[0])
    head = [found.get(f"{c}3") for c in "EFGH" if found.get(f"{c}3")]
    ooxml.check(head == ["列标签"], f"the column header: {head}")
    inside = {found.get(f"{c}{r}") for c in "EFGH" for r in range(3, last + 1)}
    ooxml.check("总计" in inside and "10" in inside, f"the grand total: {inside}")
    ooxml.check(not inside & {"Region", "Product", "Total Result"}, f"no Calc words in the table: {inside}")
    got = {name: table.get(name) for name in ATTRIBUTES}
    ooxml.check(got == dict(zip(ATTRIBUTES, NAMED)), f"the captions written: {got}")


COLUMNS = {"E3": None, "F3": "列标签", "G3": None, "H3": None, "E4": None, "F4": "A", "G4": "B",
           "H4": "总计", "E5": "Sum of Revenue", "F5": "4", "G5": "6", "H5": "10"}
PLACE = {"ref": "E3:H5", "firstHeaderRow": "1", "firstDataRow": "2", "firstDataCol": "1"}


def only_columns(path):
    """Excel's layout: the caption in E, the columns from F."""
    columns(path)
    with zipfile.ZipFile(path) as archive:
        found = {ref: value for ref, (_, _, value) in ooxml.cells(archive).items()}
        tables = ooxml.related(archive, ooxml.first_sheet(archive, ooxml.main_part(archive)), "pivotTable")
        place = ooxml.xml(archive, tables[0]).find(ooxml.S + "location")
    shown = {ref: found.get(ref) or None for ref in COLUMNS}
    ooxml.check(shown == COLUMNS, f"the table's cells: {shown}")
    ooxml.check(not any(found.get(f"I{r}") for r in range(3, 6)), "nothing right of the table")
    got = {name: place.get(name) for name in PLACE}
    ooxml.check(got == PLACE, f"the location: {got}")


def read(path):
    """The first sheet's cells and its pivot definition."""
    with zipfile.ZipFile(path) as archive:
        found = ooxml.cells(archive)
        tables = ooxml.related(archive, ooxml.first_sheet(archive, ooxml.main_part(archive)), "pivotTable")
        return found, ooxml.xml(archive, tables[0])


def no_values(path):
    """No value field: Excel's columns, H4 untouched."""
    found, table = read(path)
    want = {"E3": "列标签", "F3": None, "G3": None, "E4": "A", "F4": "B", "G4": "总计", "H4": "NEIGHBOR", "E5": None}
    shown = {ref: (found.get(ref) or (None, None, None))[2] or None for ref in want}
    ooxml.check(shown == want, f"the cells: {shown}")
    place = table.find(ooxml.S + "location")
    # Calc's empty data row is R15.11l
    got = {"ref": place.get("ref")[:4], "firstDataCol": place.get("firstDataCol")}
    ooxml.check(got == {"ref": "E3:G", "firstDataCol": "0"}, f"the location: {got}")
    ooxml.check(table.find(ooxml.S + "dataFields") is None, "no value field written")


def values(path, caption):
    """Values on rows: E4 and dataCaption are the file's."""
    columns(path, 6)
    found, table = read(path)
    shown = (found.get("E4") or (None, None, None))[2]
    ooxml.check(shown == caption, f"the values header: {shown}")
    ooxml.check(table.get("dataCaption") == caption, f"dataCaption: {table.get('dataCaption')}")


def value_name(path):
    """The value field's name shows as plain text."""
    found, table = read(path)
    name = table.find(f"{ooxml.S}dataFields/{ooxml.S}dataField").get("name")
    kind, formula, value = found.get("E5") or (None, None, None)
    ooxml.check(kind in ("s", "inlineStr") and formula is None and value == name, f"E5 for {name!r}: {kind} {formula} {value}")


CASES = {
    "pivot": lambda path: captions(path, DEFAULTS, (None, None, None)),
    "pivot-named": lambda path: captions(path, NAMED, NAMED),
    "pivot-blank": lambda path: captions(path, ("", "", "Grand Total"), ("", "", None)),
    "pivot-columns": only_columns,
    "pivot-values": lambda path: values(path, "Values"),
    "pivot-values-named": lambda path: values(path, "数值"),
    "pivot-empty": no_values,
    "pivot-name": value_name,
}
DEFINITION = "xl/pivotTables/pivotTable1.xml"
MUTATIONS = {
    "pivot": [
        ("xl/sharedStrings.xml", r">Row Labels<", ">Region<"),
        ("xl/sharedStrings.xml", r">Grand Total<", ">Total Result<"),
        (DEFINITION, r'dataCaption="Values"', 'dataCaption="Values" grandTotalCaption="Grand Total"'),
    ],
    "pivot-named": [
        ("xl/sharedStrings.xml", r">列标签<", ">Product<"),
        (DEFINITION, r' rowHeaderCaption="[^"]*"', ""),
    ],
    "pivot-blank": [
        ("xl/worksheets/sheet1.xml", r'<c r="F4"', '<c r="E4" t="inlineStr"><is><t>Row Labels</t></is></c><c r="F4"'),
        (DEFINITION, r' colHeaderCaption=""', ""),
    ],
    "pivot-columns": [
        (DEFINITION, r' colHeaderCaption="[^"]*"', ""),
        ("xl/sharedStrings.xml", r">列标签<", ">Product<"),
        (DEFINITION, r'firstDataCol="1"', 'firstDataCol="0"'),
        (DEFINITION, r'ref="E3:H5"', 'ref="E3:G5"'),
        ("xl/worksheets/sheet1.xml", r'<c r="E5"', '<c r="E3"'),
    ],
    "pivot-empty": [
        (DEFINITION, r'firstDataCol="0"', 'firstDataCol="1"'),
        (DEFINITION, r'ref="E3:G\d"', 'ref="E3:H5"'),
        ("xl/worksheets/sheet1.xml", r'<c r="H4"[^>]*>.*?</c>', '<c r="H4" t="inlineStr"><is><t>总计</t></is></c>'),
    ],
    "pivot-name": [
        ("xl/worksheets/sheet1.xml", r'<c r="E5"[^>]*>.*?</c>', '<c r="E5"><f>1+1</f><v>2</v></c>'),
        (DEFINITION, r'(<dataField [^>]*name=")[^"]*"', r'\1other"'),
    ],
    "pivot-values": [
        ("xl/sharedStrings.xml", r">列标签<", ">Product<"),
        ("xl/sharedStrings.xml", r">总计<", ">Total Result<"),
        ("xl/sharedStrings.xml", r">Values<", ">Data<"),
    ],
    "pivot-values-named": [
        ("xl/sharedStrings.xml", r">数值<", ">Data<"),
        (DEFINITION, r'dataCaption="[^"]*"', 'dataCaption="Values"'),
    ],
}

if __name__ == "__main__":
    ooxml.CASES.update(CASES)
    ooxml.MUTATIONS.update(MUTATIONS)
    if sys.argv[1] == "selftest":
        ooxml.selftest(sys.argv[2], sys.argv[3])
    else:
        ooxml.CASES[sys.argv[1]](sys.argv[2])
    for failure in ooxml.failures:
        print(f"FAIL: {failure}")
    sys.exit(1 if ooxml.failures else 0)
