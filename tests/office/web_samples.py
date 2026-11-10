#!/usr/bin/env python3
"""Thirty Ely-saved files for Office on the web."""

import os
import shutil
import subprocess
import sys

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "../.."))
sys.path.insert(0, os.path.join(ROOT, "tests/perf"))
import loadfiles  # noqa: E402
import richfiles  # noqa: E402

FIXTURES = os.path.join(ROOT, "tests/fixtures")
# Fixtures as they are; the made ones fill each ten.
COPIED = {
    "office": ["word.docx", "redline.docx", "equation.docx", "toc.docx", "toc-shape.docx", "t.docx", "backslash.docx", "t.xlsx", "t.pptx"],
    "table": ["kept.xlsx", "pivot.xlsx", "pivot-values.xlsx", "spill.xlsx", "links.xlsx", "booleans.xlsx", "date1904.xlsx"],
    "show": ["sections.pptx", "sections-utf16.pptx", "video.pptx", "trouble.fodp"],
}
PAIR = """<?xml version="1.0" encoding="UTF-8"?>
<office:document xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
 xmlns:draw="urn:oasis:names:tc:opendocument:xmlns:drawing:1.0"
 xmlns:svg="urn:oasis:names:tc:opendocument:xmlns:svg-compatible:1.0"
 xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
 office:version="1.3" office:mimetype="application/vnd.oasis.opendocument.presentation">
 <office:body><office:presentation>
 <draw:page draw:name="One"><draw:rect svg:x="2cm" svg:y="5cm" svg:width="4cm" svg:height="3cm"><text:p>左 Left</text:p></draw:rect><draw:ellipse svg:x="16cm" svg:y="5cm" svg:width="4cm" svg:height="3cm"><text:p>右 Right</text:p></draw:ellipse><draw:line svg:x1="6cm" svg:y1="6.5cm" svg:x2="16cm" svg:y2="6.5cm"/></draw:page>
 <draw:page draw:name="Two"><draw:rect svg:x="3cm" svg:y="3cm" svg:width="10cm" svg:height="2cm"><text:p>第二页 Second</text:p></draw:rect></draw:page>
 </office:presentation></office:body>
</office:document>"""


def sources(folder):
    """The files Ely opens, written into `folder`."""
    at = lambda name: os.path.join(folder, name)
    for kind, names in COPIED.items():
        for name in names:
            shutil.copy(os.path.join(FIXTURES, kind, name), at(name))
    loadfiles.small_docx(at("rich.docx"), 1)
    richfiles.docx(at("rich.docx"))
    richfiles.pages_docx(at("pages.docx"), 30)
    loadfiles.big_docx(at("big.docx"))
    loadfiles.workbook(at("rich.xlsx"), 2000)
    richfiles.xlsx(at("rich.xlsx"), 2000)
    loadfiles.workbook(at("workbook.xlsx"), 500, typing=True)
    loadfiles.presentation(at("rich.pptx"), 3)
    richfiles.pptx(at("rich.pptx"))
    loadfiles.presentation(at("deck1.pptx"), 1)
    loadfiles.presentation(at("deck3.pptx"), 3)
    loadfiles.big_pptx(at("big.pptx"))
    with open(at("pair.fodp"), "w", encoding="utf-8") as deck:
        deck.write(PAIR)


def main():
    if len(sys.argv) != 3:
        sys.exit("usage: web_samples.py <resave program> <output folder>")
    resave, out = sys.argv[1], os.path.abspath(sys.argv[2])
    made = os.path.join(out, "sources")
    os.makedirs(made, exist_ok=True)
    sources(made)
    for name in sorted(os.listdir(made)):
        # Flat ODF goes out as the Office type.
        saved = os.path.join(out, name.replace(".fodp", ".pptx"))
        subprocess.run([resave, os.path.join(made, name), saved], check=True, timeout=600)
    shutil.rmtree(made)
    kinds = [name.rsplit(".", 1)[1] for name in os.listdir(out)]
    print("web_samples:", ", ".join(f"{kinds.count(kind)} {kind}" for kind in sorted(set(kinds))), "in", out)


if __name__ == "__main__":
    main()
