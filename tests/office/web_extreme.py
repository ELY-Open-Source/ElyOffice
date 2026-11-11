#!/usr/bin/env python3
"""Large and strange files Ely saves, for Office online."""

import os
import shutil
import subprocess
import sys

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "../.."))
sys.path.insert(0, os.path.join(ROOT, "tests/perf"))
import loadfiles  # noqa: E402
import richfiles  # noqa: E402

FIXTURES = os.path.join(ROOT, "tests/fixtures")
BIGTABLE = os.path.join(ROOT, "tests/perf/bigtable.py")
P = 'xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"'
A = 'xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"'
# The Word-encrypted fixtures open with this.
PASSWORD = "ely"
BREAK = ["page.break"]
SLIDE = ["slide.insert"]


def cell(sheet):
    return ["cell.set", f"{sheet}!Z1", "Ely 编辑"]


def nested_docx(path, depth=12):
    """Tables inside tables, and a list nine levels deep."""
    table = loadfiles.para("最里层 innermost")
    for level in range(depth, 0, -1):
        cell = f"<w:tc>{loadfiles.para(f'第 {level} 层 level {level}')}{table}<w:p/></w:tc>"
        table = f'<w:tbl><w:tblPr><w:tblW w:w="0" w:type="auto"/></w:tblPr><w:tr>{cell}<w:tc><w:p/></w:tc></w:tr></w:tbl>'
    items = [f'<w:p><w:pPr><w:pStyle w:val="ListParagraph"/><w:ind w:left="{360 * (k + 1)}"/></w:pPr>'
             f'<w:r><w:t>项 {k} item {k}</w:t></w:r></w:p>' for k in range(9)]
    loadfiles.word(path, [loadfiles.para("嵌套 Nested")] + items + [table, loadfiles.para("末尾 End")])


def column(n):
    name = ""
    while n:
        n, rest = divmod(n - 1, 26)
        name = chr(65 + rest) + name
    return name


def wide_xlsx(path, rows=1000, columns=1000):
    """A million cells; each row ends in a sum."""
    ns = f'xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="{loadfiles.REL}"'
    last = column(columns)
    body = []
    for r in range(1, rows + 1):
        values = "".join(f'<c r="{column(c)}{r}"><v>{(r * c) % 997}</v></c>' for c in range(1, columns))
        body.append(f'<row r="{r}">{values}<c r="{last}{r}"><f>SUM(A{r}:{column(columns - 1)}{r})</f></c></row>')
    parts = {"xl/worksheets/sheet1.xml": f"<worksheet {ns}><sheetData>{''.join(body)}</sheetData></worksheet>",
             "xl/workbook.xml": f'<workbook {ns}><sheets><sheet name="宽表 Wide" sheetId="1" r:id="s1"/></sheets></workbook>'}
    rels = f'<Relationship Id="s1" Type="{loadfiles.REL}/worksheet" Target="worksheets/sheet1.xml"/>'
    loadfiles.package(path, "xl/workbook.xml", loadfiles.BOOK_MAIN, parts, rels, {"xl/worksheets/sheet1.xml": loadfiles.SHEET})


def nested_pptx(path, depth=10):
    """t.pptx with groups ten deep on its first slide."""
    loadfiles.presentation(path, 0)
    box = ('<p:sp><p:nvSpPr><p:cNvPr id="8000" name="innermost"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>'
           '<p:spPr><a:xfrm><a:off x="1000000" y="1000000"/><a:ext cx="3000000" cy="800000"/></a:xfrm>'
           '<a:prstGeom prst="rect"><a:avLst/></a:prstGeom></p:spPr><p:txBody><a:bodyPr/><a:lstStyle/>'
           '<a:p><a:r><a:rPr lang="zh-CN"/><a:t>最里层 innermost</a:t></a:r></a:p></p:txBody></p:sp>')
    for level in range(depth, 0, -1):
        frame = '<a:off x="1000000" y="1000000"/><a:ext cx="3000000" cy="800000"/>'
        box = (f'<p:grpSp><p:nvGrpSpPr><p:cNvPr id="{8000 + level}" name="group {level}"/><p:cNvGrpSpPr/><p:nvPr/>'
               f'</p:nvGrpSpPr><p:grpSpPr><a:xfrm>{frame}<a:chOff x="1000000" y="1000000"/>'
               f'<a:chExt cx="3000000" cy="800000"/></a:xfrm></p:grpSpPr>{box}</p:grpSp>')
    loadfiles.rewrite(path, {"ppt/slides/slide1.xml": lambda xml: loadfiles.before_last(xml, "</p:spTree>", box)})


def bigtable(path):
    """Four hundred rows, a picture in each."""
    subprocess.run([sys.executable, "-I", BIGTABLE, path, "400", "640", "480", "jpeg", "inline"], check=True)


def copy(kind, name):
    return lambda path: shutil.copy(os.path.join(FIXTURES, kind, name), path)


def huge_xlsx(path):
    loadfiles.workbook(path, 100000)
    richfiles.xlsx(path, 100000)


def rich_docx(path):
    loadfiles.small_docx(path, 1)
    richfiles.docx(path)


def rich_pptx(path):
    loadfiles.presentation(path, 3)
    richfiles.pptx(path)


# Name, maker, password, the one edit.
SAMPLES = [
    ("bigtable.docx", bigtable, None, BREAK),
    ("nested.docx", nested_docx, None, BREAK),
    ("pages300.docx", lambda path: richfiles.pages_docx(path, 300), None, BREAK),
    ("rich.docx", rich_docx, None, BREAK),
    ("equation.docx", copy("office", "equation.docx"), None, BREAK),
    ("locked.docx", copy("office", "ole-link-other-locked.docx"), PASSWORD, BREAK),
    ("huge.xlsx", huge_xlsx, None, cell("Data")),
    ("wide.xlsx", wide_xlsx, None, cell("'宽表 Wide'")),
    ("pivot.xlsx", copy("table", "pivot.xlsx"), None, cell("Sheet1")),
    ("spill.xlsx", copy("table", "spill.xlsx"), None, cell("Sheet1")),
    ("kept-more.xlsx", copy("table", "kept-more.xlsx"), None, cell("Sheet1")),
    ("slides207.pptx", lambda path: loadfiles.presentation(path, 22), None, SLIDE),
    ("nested.pptx", nested_pptx, None, SLIDE),
    ("rich.pptx", rich_pptx, None, SLIDE),
    ("video.pptx", copy("show", "video.pptx"), None, SLIDE),
    ("locked.pptx", copy("show", "sections-locked.pptx"), PASSWORD, SLIDE),
]


def main():
    if len(sys.argv) != 3:
        sys.exit("usage: web_extreme.py <resave program> <output folder>")
    resave, out = sys.argv[1], os.path.abspath(sys.argv[2])
    made = os.path.join(out, "sources")
    os.makedirs(made, exist_ok=True)
    for name, make, password, edit in SAMPLES:
        source = os.path.join(made, name)
        make(source)
        locked = ["-p", password] if password else []
        subprocess.run([resave, source, os.path.join(out, name), *locked, *edit], check=True, timeout=1800)
        print("web_extreme:", name, os.path.getsize(source), "->", os.path.getsize(os.path.join(out, name)), "bytes")
    # Word's own encrypted files go up too.
    for name, _, password, _ in SAMPLES:
        if password:
            shutil.copy(os.path.join(made, name), os.path.join(out, "original-" + name))
    shutil.rmtree(made)


if __name__ == "__main__":
    main()
