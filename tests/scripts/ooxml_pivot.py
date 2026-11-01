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


CASES = {
    "pivot": lambda path: captions(path, DEFAULTS, (None, None, None)),
    "pivot-named": lambda path: captions(path, NAMED, NAMED),
    "pivot-blank": lambda path: captions(path, ("", "", "Grand Total"), ("", "", None)),
    "pivot-columns": columns,
    "pivot-values": lambda path: columns(path, 6),
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
    ],
    "pivot-values": [
        ("xl/sharedStrings.xml", r">列标签<", ">Product<"),
        ("xl/sharedStrings.xml", r">总计<", ">Total Result<"),
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
