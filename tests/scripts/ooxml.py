#!/usr/bin/env python3
"""Checks a saved DOCX, XLSX or PPTX without LibreOffice."""

import datetime
import math
import posixpath
import os
import re
import shutil
import subprocess
import sys
import tempfile
import zipfile
import xml.etree.ElementTree as ET

W = "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}"
S = "{http://schemas.openxmlformats.org/spreadsheetml/2006/main}"
P = "{http://schemas.openxmlformats.org/presentationml/2006/main}"
A = "{http://schemas.openxmlformats.org/drawingml/2006/main}"
R = "{http://schemas.openxmlformats.org/officeDocument/2006/relationships}"
REL = "{http://schemas.openxmlformats.org/package/2006/relationships}"

failures = []


def check(ok, what):
    if not ok:
        failures.append(what)
    return ok


def xml(archive, name):
    return ET.fromstring(archive.read(name))


def texts(element, tag):
    return "".join(node.text or "" for node in element.iter(tag))


def rels(archive, part):
    """A part's relationships: id to (type's last word, target part)."""
    folder, name = posixpath.split(part)
    path = posixpath.join(folder, "_rels", name + ".rels")
    if path not in archive.namelist():
        return {}
    found = {}
    for rel in xml(archive, path).iter(REL + "Relationship"):
        target = rel.get("Target")
        if rel.get("TargetMode") != "External":
            target = posixpath.normpath(posixpath.join(folder, target)).lstrip("/")
        found[rel.get("Id")] = (rel.get("Type").rsplit("/", 1)[-1], target)
    return found


def related(archive, part, kind):
    """The parts `part` relates to as `kind`; each must exist."""
    targets = [target for got, target in rels(archive, part).values() if got == kind]
    for target in targets:
        check(target in archive.namelist(), f"{part} relates to a missing {target}")
    return [target for target in targets if target in archive.namelist()]


def main_part(archive):
    parts = related(archive, "", "officeDocument")
    check(len(parts) == 1, "one main part")
    return parts[0] if parts else None


def on(element):
    """An OOXML on/off element: absent value means on."""
    return element is not None and element.get(W + "val", "true") in ("true", "1", "on")


def before_references(body):
    """Each comment reference's id, with its paragraph's text before it."""
    placed = {}
    for paragraph in body.iter(W + "p"):
        seen = ""
        for node in paragraph.iter():
            if node.tag == W + "t":
                seen += node.text or ""
            elif node.tag == W + "commentReference":
                placed[node.get(W + "id")] = seen
    return placed


def contract(path):
    """Typed text, a comment, a tracked insertion, both sessions."""
    with zipfile.ZipFile(path) as archive:
        document = main_part(archive)
        if not document:
            return
        body = xml(archive, document)
        paragraphs = [texts(p, W + "t") for p in body.iter(W + "p")]
        whole = "\n".join(paragraphs)
        for typed in ["Alpha 甲", "Hello 世界", "Gamma"]:
            check(typed in whole, f"document text has {typed!r}: {paragraphs}")
        inserted = "".join(texts(ins, W + "t") for ins in body.iter(W + "ins"))
        for tracked in ["Beta", "Gamma"]:
            check(tracked in inserted, f"{tracked} is a tracked insertion, not {inserted!r}")
        placed = before_references(body)
        comments = related(archive, document, "comments")
        if check(len(comments) == 1, "the document relates to one comment part"):
            said = {c.get(W + "id"): texts(c, W + "t") for c in xml(archive, comments[0]).iter(W + "comment")}
            ours = [id_ for id_, text in said.items() if "Check alpha" in text]
            check(len(ours) == 1, f"one comment says Check alpha: {said}")
            for id_ in ours:
                at = placed.get(id_, "")
                check(at.rstrip().endswith("Alpha 甲"), f"comment {id_} follows Alpha 甲, not {at[-20:]!r}")
                # Made at a point: no range markers.
                ranged = [n for n in body.iter() if n.tag in (W + "commentRangeStart", W + "commentRangeEnd") and n.get(W + "id") == id_]
                check(not ranged, f"comment {id_} stays a point, not a range")
        settings = related(archive, document, "settings")
        if check(len(settings) == 1, "the document relates to its settings"):
            check(on(xml(archive, settings[0]).find(W + "trackRevisions")), "tracking stays on")
    # Apple's own DOCX reader, a second independent opinion.
    if shutil.which("textutil"):
        shown = subprocess.run(["textutil", "-convert", "txt", "-stdout", path],
                               capture_output=True, text=True, check=True).stdout
        check("Alpha 甲" in shown and "Hello 世界" in shown, f"textutil reads the text: {shown!r}")


def cells(archive):
    """The workbook's first declared sheet, through its relationship."""
    book = main_part(archive)
    if not book:
        return {}
    shared = []
    for part in related(archive, book, "sharedStrings"):
        shared = [texts(si, S + "t") for si in xml(archive, part).iter(S + "si")]
    sheets = list(xml(archive, book).iter(S + "sheet"))
    if not check(sheets, "the workbook lists a sheet"):
        return {}
    target = rels(archive, book).get(sheets[0].get(R + "id"))
    if not check(target and target[1] in archive.namelist(), f"the first sheet's part exists: {target}"):
        return {}
    found = {}
    for c in xml(archive, target[1]).iter(S + "c"):
        kind = c.get("t", "n")
        f, v = c.find(S + "f"), c.find(S + "v")
        value = v.text if v is not None else None
        if kind == "s" and value is not None:
            value = shared[int(value)]
        elif kind == "inlineStr":
            value = texts(c, S + "t")
        found[c.get("r")] = (kind, f.text if f is not None else None, value)
    return found


def sheet(path):
    """Inputs, formulas and cached results against Python's own sums."""
    h1, h2 = 7, 5
    serial = (datetime.date(2024, 2, 29) - datetime.date(1899, 12, 30)).days
    # Exact: integers, dates, errors, text. Floats: the case's tolerance.
    expected = {
        "H1": (None, "n", h1, 0),
        "H2": (None, "n", h2, 0),
        "H3": ("H1*H2", "n", h1 * h2, 0),
        "H4": ("H1/3", "n", h1 / 3, 1e-12),
        "H5": ("DATE(2024,2,29)", "n", serial, 0),
        "H6": ("1/0", "e", "#DIV/0!", None),
        "H7": ('"Ely"&"-"&TEXT(H1,"0")', "str", f"Ely-{h1}", None),
        "H8": ("SUM(H1:H2)", "n", h1 + h2, 0),
        "H9": ("H3+1", "n", h1 * h2 + 1, 0),
    }
    with zipfile.ZipFile(path) as archive:
        found = cells(archive)
    for ref, (formula, kind, value, tolerance) in expected.items():
        if not check(ref in found, f"{ref} is there"):
            continue
        got_kind, got_formula, got = found[ref]
        check(got_formula == formula, f"{ref} formula {got_formula!r}, not {formula!r}")
        check(got_kind == kind, f"{ref} type {got_kind!r}, not {kind!r}")
        if tolerance is None:
            check(got == value, f"{ref} is {got!r}, not {value!r}")
        elif tolerance == 0:
            check(got is not None and float(got) == value and float(got).is_integer(), f"{ref} is {got!r}, not {value}")
        else:
            check(got is not None and math.isclose(float(got), value, rel_tol=0, abs_tol=tolerance),
                  f"{ref} is {got!r}, not {value} within {tolerance}")


def notes_body(notes):
    """A notes page's body placeholder text: the editable notes."""
    found = []
    for shape in notes.iter(P + "sp"):
        holder = shape.find(f"{P}nvSpPr/{P}nvPr/{P}ph")
        if holder is not None and holder.get("type") == "body":
            found.append(texts(shape, A + "t"))
    return " ".join(found)


def deck(path):
    """Slides in order, a hidden slide, notes on their slide."""
    with zipfile.ZipFile(path) as archive:
        show = main_part(archive)
        if not show:
            return
        links = rels(archive, show)
        order = [sid.get(R + "id") for sid in xml(archive, show).iter(P + "sldId")]
        slides = []
        for rid in order:
            target = links.get(rid)
            if check(target and target[1] in archive.namelist(), f"slide {rid} has its part: {target}"):
                slides.append(target[1])
        check(len(order) == 3, f"three slides, not {len(order)}")
        if not slides:
            return
        check(len(set(slides)) == len(slides), f"each slide its own part: {slides}")
        # The fixture: First, the inserted blank, the hidden Second.
        said = [texts(xml(archive, part), A + "t").strip() for part in slides]
        check(said == ["First", "", "Second"], f"slides in order say {said}")
        hidden = [xml(archive, part).get("show") == "0" for part in slides]
        check(hidden == [False, False, True], f"only the last slide is hidden: {hidden}")
        spoken = [" ".join(notes_body(xml(archive, n)) for n in related(archive, part, "notesSlide")) for part in slides]
        check("Speaker 甲" in spoken[0], f"slide one's notes say Speaker 甲, not {spoken[0]!r}")
        check(not any("Speaker" in text for text in spoken[1:]), f"only slide one has the notes: {spoken}")


CUBE = 'CUBEVALUE("Sales","[Measures].[Total]")'
# tests/fixtures/table/kept.xlsx: what Calc cannot compute keeps the file's result.
KEPT = {
    "A1": (None, "n", "2"),
    "B1": (CUBE, "n", "41"),
    "C1": ("B1+1", "n", "42"),
    "D1": ("A1*" + CUBE, "n", "82"),
    "E1": ('_xlfn.TRANSLATE("hello","en","de")', "str", "hallo"),
    "F1": ("SUM(A1,1)", "n", "3"),
    "A2": (None, "n", "10"),
    "B2": ('CUBEVALUE("Sales",A2)', "n", "100"),
    "B3": ('CUBEVALUE("Sales",A3)', "n", "200"),
}
# After A1 and A2 change, their dependents are unknown.
EDITED = KEPT | {
    "A1": (None, "n", "3"),
    "D1": ("A1*" + CUBE, "e", "#NAME?"),
    "F1": ("SUM(A1,1)", "n", "4"),
    "A2": (None, "n", "11"),
    "B2": ('CUBEVALUE("Sales",A2)', "e", "#NAME?"),
}

# tests/fixtures/table/kept-more.xlsx: each kind of result the file holds.
MORE = {
    "A1": ("FUTUREFN(1)", "b", "1"),
    "B1": ("FUTUREFN(2)", "str", None),
    "C1": ("FUTUREFN(3)", "e", "#NAME?"),
    "D1": ("FUTUREFN(4)", "e", "#NAME?"),
    "E1": (CUBE, "n", "41"),
    "A2": ("FUTUREFN(I2)", "b", "1"),
    "A3": ("FUTUREFN(I3)", "b", "0"),
    "B2": ("FUTUREFN(I2)", "str", None),
    "B3": ("FUTUREFN(I3)", "str", "ok"),
    "G5": ("FUTUREFN(1)", "n", "1"),
    "H5": (None, "n", "2"),
    "G6": (None, "n", "3"),
    "H6": (None, "str", "four"),
    "K8": ('FUTUREFN(INDIRECT("J8"))', "n", "42"),
    "L8": ("FUTUREFN(OFFSET(J8,0,0))", "n", "43"),
    "J1": ("FUTUREFN(9)", "b", "1"),
    "G10": ("FUTUREFN(5)", "b", "1"),
    "H10": (None, "b", "0"),
    "G11": ("FUTUREFN(6)", "n", "41"),
    "H11": (None, "str", None),
    "G12": ("FUTUREFN(7)", "e", "#NAME?"),
    "H12": (None, "e", "#NAME?"),
    "G13": ("FUTUREFN(8)", "e", "#NAME?"),
    "H13": (None, "e", "#NAME?"),
    "G14": ("FUTUREFN(10)", "b", "1"),
    "H14": (None, "n", "41"),
    "G15": ("FUTUREFN(11)", "n", "41"),
    "H15": (None, "b", "1"),
}
# A sheet copy is a change: volatile cells recalculate.
COPIED = MORE | {
    "K8": ('FUTUREFN(INDIRECT("J8"))', "e", "#NAME?"),
    "L8": ("FUTUREFN(OFFSET(J8,0,0))", "e", "#NAME?"),
}


def cells_are(path, expected):
    """Formulas as written, with the file's type and value."""
    with zipfile.ZipFile(path) as archive:
        found = cells(archive)
    for ref, want in expected.items():
        if check(ref in found, f"{ref} is there"):
            got = (found[ref][1], found[ref][0], found[ref][2])
            check(got == want, f"{ref} is {got!r}, not {want!r}")


CASES = {
    "contract": contract,
    "sheet": sheet,
    "deck": deck,
    "kept": lambda path: cells_are(path, KEPT),
    "kept-edited": lambda path: cells_are(path, EDITED),
    "kept-more": lambda path: cells_are(path, MORE),
    "kept-copied": lambda path: cells_are(path, COPIED),
}

# Damage each check must catch: part, pattern, replacement.
MUTATIONS = {
    "contract": [
        ("word/document.xml", r"<w:commentReference [^>]*/>", ""),
        ("word/settings.xml", r"<w:trackRevisions/>", '<w:trackRevisions w:val="false"/>'),
        ("word/document.xml", r'(?s)<w:commentReference w:id="0"/>((?:(?!</w:p>).)*)</w:p>',
         r'\1<w:r><w:commentReference w:id="0"/></w:r></w:p>'),
        ("word/document.xml", r'(?s)^(.*?<w:p\b[^>]*>)', r'\1<w:commentRangeStart w:id="0"/>'),
        ("word/document.xml", r'(?s)<w:ins\b[^>]*>((?:(?!</w:ins>).)*Gamma(?:(?!</w:ins>).)*)</w:ins>', r"\1"),
    ],
    "sheet": [
        ("xl/worksheets/sheet1.xml", r'(<c r="H1"[^>]*><v>)7(</v>)', r"\g<1>70\g<2>"),
        ("xl/_rels/workbook.xml.rels", r"worksheets/sheet1\.xml", "worksheets/missing.xml"),
    ],
    "kept": [
        ("xl/worksheets/sheet1.xml", r'(<c r="B1"[^>]*>.*?<v>)41(</v>)', r"\g<1>0\g<2>"),
        ("xl/worksheets/sheet1.xml", r"CUBEVALUE\(&quot;Sales&quot;,&quot;\[", "cubevalue(&quot;Sales&quot;,&quot;["),
        ("xl/worksheets/sheet1.xml", r'<c r="E1"[^>]*t="str"', '<c r="E1" s="0" t="e"'),
    ],
    "kept-edited": [
        ("xl/worksheets/sheet1.xml", r'<c r="D1"[^>]*t="e"><f([^>]*)>(.*?)</f><v>#NAME\?</v>', r'<c r="D1" t="n"><f\1>\2</f><v>82</v>'),
        ("xl/worksheets/sheet1.xml", r'(<c r="B3"[^>]*>.*?<v>)200(</v>)', r"\g<1>0\g<2>"),
    ],
    "kept-more": [
        ("xl/worksheets/sheet1.xml", r'(<c r="A1"[^>]*)t="b"', r'\g<1>t="n"'),
        ("xl/worksheets/sheet1.xml", r'(<c r="H10"[^>]*)t="b"', r'\g<1>t="n"'),
        ("xl/worksheets/sheet1.xml", r'(<c r="H6"[^>]*><v>)four(</v>)', r"\g<1>five\g<2>"),
        ("xl/worksheets/sheet1.xml", r'(<c r="K8"[^>]*>.*?<v>)42(</v>)', r"\g<1>0\g<2>"),
    ],
    "kept-copied": [
        ("xl/worksheets/sheet1.xml", r'(<c r="H10"[^>]*)t="b"', r'\g<1>t="n"'),
        ("xl/worksheets/sheet1.xml", r'(<c r="K8"[^>]*)t="e"', r'\g<1>t="n"'),
    ],
    "deck": [
        ("ppt/_rels/presentation.xml.rels", r"slides/slide2\.xml", "slides/missing.xml"),
        ("ppt/_rels/presentation.xml.rels", r"slides/slide2\.xml", "slides/slide1.xml"),
        ("ppt/presentation.xml", r'(?s)r:id="rId5"(.*?)r:id="rId6"', r'r:id="rId6"\1r:id="rId5"'),
        ("ppt/notesSlides/notesSlide1.xml", r'<p:ph type="body"', '<p:ph type="ftr"'),
        ("ppt/slides/_rels/slide1.xml.rels", r"<Relationship [^>]*notesSlide\"[^>]*/>", ""),
    ],
}


def selftest(case, path):
    """Each mutation of a good file must fail the check."""
    global failures
    missed = []
    for part, pattern, replacement in MUTATIONS[case]:
        with zipfile.ZipFile(path) as archive:
            entries = {name: archive.read(name) for name in archive.namelist()}
        text = entries[part].decode()
        changed, count = re.subn(pattern, replacement, text)
        if count == 0:
            missed.append(f"{part}: {pattern} found nothing to change")
            continue
        entries[part] = changed.encode()
        # A folder, not an open file: Windows cannot reopen those.
        with tempfile.TemporaryDirectory() as folder:
            copy = os.path.join(folder, "mutated" + path[-5:])
            with zipfile.ZipFile(copy, "w") as archive:
                for name, body in entries.items():
                    archive.writestr(name, body)
            failures = []
            CASES[case](copy)
            if not failures:
                missed.append(f"{part}: {pattern} passed")
    failures = [f"a mutation went unnoticed: {what}" for what in missed]


if __name__ == "__main__":
    if sys.argv[1] == "selftest":
        selftest(sys.argv[2], sys.argv[3])
    else:
        CASES[sys.argv[1]](sys.argv[2])
    for failure in failures:
        print(f"FAIL: {failure}")
    sys.exit(1 if failures else 0)
