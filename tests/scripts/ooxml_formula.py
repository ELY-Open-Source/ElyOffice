"""Checks a saved XLSX's formula text without LibreOffice."""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import ooxml  # noqa: E402

# tests/fixtures/table/booleans.xlsx: Excel's words, as Excel wrote them.
BOOLEANS = {
    "B1": ("IF(A1>0,TRUE,FALSE)", "b", "1"),
    "C1": ("VLOOKUP(2,A1:B1,1,FALSE)", "n", "2"),
}
CASES = {"booleans": lambda path: ooxml.cells_are(path, BOOLEANS)}
SHEET = "xl/worksheets/sheet1.xml"
MUTATIONS = {
    "booleans": [
        (SHEET, r",TRUE,", ",TRUE(),"),
        (SHEET, r",FALSE\)</f><v>2", ",FALSE())</f><v>2"),
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
