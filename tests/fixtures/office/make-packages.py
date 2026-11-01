"""Writes #49's packages: backslash folder entries, and one truly broken."""
import os
import sys
import zipfile

HERE = os.path.dirname(os.path.abspath(__file__))
BACKSLASH = chr(92)


def copy(source, target, names):
    with zipfile.ZipFile(os.path.join(HERE, source)) as old, zipfile.ZipFile(os.path.join(sys.argv[1], target), "w") as out:
        for item in old.infolist():
            out.writestr(item, old.read(item.filename))
        for name in names:
            info = zipfile.ZipInfo("x", (2026, 11, 1, 0, 0, 0))
            # Set after: ZipInfo turns the backslash on Windows.
            info.filename = name
            out.writestr(info, b"" if name.endswith("/") else b"<x/>")


# Tencent Docs' folder entries.
copy("t.docx", "backslash.docx", ["word" + BACKSLASH + "theme/", "word" + BACKSLASH + "_rels/"])
copy("t.xlsx", "backslash.xlsx", ["xl" + BACKSLASH + "worksheets/", "xl" + BACKSLASH + "theme/", "xl" + BACKSLASH + "_rels/"])
# No zip name may hold a question mark.
copy("t.docx", "broken.docx", ["word/a?b.xml"])
