"""Checks a saved DOCX's math and contents without LibreOffice."""
import os
import sys
import zipfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import ooxml  # noqa: E402

# Word's double-struck letters and digits.
HOLES = {"C": 0x2102, "H": 0x210D, "N": 0x2115, "P": 0x2119, "Q": 0x211A, "R": 0x211D, "Z": 0x2124}


def italic(text):
    """Word's math italic letters."""
    shown = ""
    for c in text:
        if c == "h":
            shown += chr(0x210E)
        elif "a" <= c <= "z":
            shown += chr(0x1D44E + ord(c) - ord("a"))
        elif "A" <= c <= "Z":
            shown += chr(0x1D434 + ord(c) - ord("A"))
        else:
            shown += c
    return shown


def struck(run, text):
    """What Word shows for a math run."""
    scr = run.find(f"{ooxml.MATH}rPr/{ooxml.MATH}scr")
    if scr is None or scr.get(ooxml.MATH + "val") != "double-struck":
        return italic(text)
    shown = ""
    for c in text:
        if c in HOLES:
            shown += chr(HOLES[c])
        elif "A" <= c <= "Z":
            shown += chr(0x1D538 + ord(c) - ord("A"))
        elif "a" <= c <= "z":
            shown += chr(0x1D552 + ord(c) - ord("a"))
        elif "0" <= c <= "9":
            shown += chr(0x1D7D8 + ord(c) - ord("0"))
        else:
            shown += c
    return shown


def equation(path):
    """Math hyphens are minus signs; normal text keeps its own."""
    with zipfile.ZipFile(path) as archive:
        body = ooxml.xml(archive, ooxml.main_part(archive))
    said = ""
    for run in body.iter(ooxml.MATH + "r"):
        text = ooxml.texts(run, ooxml.MATH + "t")
        nor = run.find(f"{ooxml.MATH}rPr/{ooxml.MATH}nor")
        if nor is not None and nor.get(ooxml.MATH + "val", "on") in ("on", "1", "true"):
            said += text
        else:
            # Word shows this hyphen as U+2212.
            said += struck(run, text.replace("-", "−"))
    ooxml.check(said == "𝑥−𝑏−𝑐a-b1−2𝟛−𝟜5-6\"alpha\"\"𝑎𝑙𝑝ℎ𝑎\"", f"Word reads {said!r}")


def toc_end(path):
    """The TOC field ends alone, last in its content control."""
    with zipfile.ZipFile(path) as archive:
        body = ooxml.xml(archive, ooxml.main_part(archive)).find(ooxml.W + "body")
    kids = list(body)
    sdts = [k for k in kids if k.tag == ooxml.W + "sdt" and "Table of Contents" in ooxml.ET.tostring(k, encoding="unicode")]
    if not ooxml.check(len(sdts) == 1, "one TOC content control"):
        return
    last = sdts[0].find(ooxml.W + "sdtContent").findall(ooxml.W + "p")[-1]
    kinds = [f.get(ooxml.W + "fldCharType") for f in last.iter(ooxml.W + "fldChar")]
    ooxml.check(kinds == ["end"] and ooxml.texts(last, ooxml.W + "t") == "", f"last paragraph holds {kinds} {ooxml.texts(last, ooxml.W + 't')!r}")
    after = ooxml.texts(kids[kids.index(sdts[0]) + 1], ooxml.W + "t")
    ooxml.check(after == "Alpha", f"the TOC is followed by {after!r}")


CASES = {"equation": equation, "toc-end": toc_end}
MUTATIONS = {
    "equation": [
        ("word/document.xml", r"<m:r>(<m:t[^>]*>)[-−](</m:t>)", r"<m:r><m:rPr><m:nor/></m:rPr>\1-\2"),
        ("word/document.xml", r"<m:nor/>", ""),
        ("word/document.xml", r"a-b", "a−b"),
        ("word/document.xml", r"<m:r>(<m:t[^>]*>)−(</m:t></m:r><m:r><m:t[^>]*>2<)", r"<m:r><m:rPr><m:nor/></m:rPr>\1-\2"),
        ("word/document.xml", r"𝟛", "3"),
        ("word/document.xml", r"5-6", "𝟝-𝟞"),
        ("word/document.xml", r'"alpha"|&quot;alpha&quot;', "alpha"),
        ("word/document.xml", r"(?s)<m:nor/>((?:(?!</m:r>).)*?alpha)", r"\1"),
    ],
    "toc-end": [
        ("word/document.xml", r'(?s)(</w:r>)(</w:p>)(<w:p[ >](?:(?!<w:p[ >]).)*?)<w:r>(?:<w:rPr></w:rPr>)?<w:fldChar w:fldCharType="end"/></w:r>(</w:p>)</w:sdtContent></w:sdt>', r'\1<w:r><w:fldChar w:fldCharType="end"/></w:r>\2</w:sdtContent></w:sdt>\3\4'),
        ("word/document.xml", r"</w:sdt>", "</w:sdt><w:p/>"),
        ("word/document.xml", r'(<w:r>(?:<w:rPr></w:rPr>)?<w:fldChar w:fldCharType="end"/></w:r></w:p></w:sdtContent>)', r'<w:r><w:t>x</w:t></w:r>\1'),
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
