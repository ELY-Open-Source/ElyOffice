"""Writes sample.epub: two chapters, a table of contents, Chinese text."""
import os
import sys
import zipfile

CONTAINER = """<?xml version="1.0" encoding="UTF-8"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
<rootfiles><rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/></rootfiles>
</container>"""

OPF = """<?xml version="1.0" encoding="UTF-8"?>
<package xmlns="http://www.idpf.org/2007/opf" version="3.0" unique-identifier="id">
<metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
<dc:identifier id="id">ely-sample</dc:identifier><dc:title>Ely Sample</dc:title><dc:language>zh</dc:language>
</metadata>
<manifest>
<item id="nav" href="nav.xhtml" media-type="application/xhtml+xml" properties="nav"/>
<item id="style" href="style.css" media-type="text/css"/>
<item id="one" href="one.xhtml" media-type="application/xhtml+xml"/>
<item id="two" href="two.xhtml" media-type="application/xhtml+xml"/>
</manifest>
<spine><itemref idref="one"/><itemref idref="two"/></spine>
</package>"""

NAV = """<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops"><head><title>Contents</title></head>
<body><nav epub:type="toc"><ol>
<li><a href="one.xhtml">First Chapter</a></li>
<li><a href="two.xhtml">Second Chapter</a></li>
</ol></nav></body></html>"""

CSS = "p { margin: 0 0 1em 0; }\n"


def chapter(title, body):
    return ("""<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="http://www.w3.org/1999/xhtml"><head><title>""" + title
            + """</title><link rel="stylesheet" href="style.css"/></head><body><h1>""" + title + "</h1>" + body + "</body></html>")


# A picture the book lacks: MuPDF warns while laying out.
ONE = chapter("First Chapter", "<p>Ely reads books.</p><p>你好，世界。离线阅读。</p><img src=\"ely-private-picture.png\"/>")
TWO = chapter("Second Chapter", "".join("<p>Line %d of the second chapter, long enough to wrap the page.</p>" % n for n in range(60)))

with zipfile.ZipFile(os.path.join(sys.argv[1], "sample.epub"), "w") as out:
    for part, text, kind in [("mimetype", "application/epub+zip", zipfile.ZIP_STORED),
                             ("META-INF/container.xml", CONTAINER, zipfile.ZIP_DEFLATED),
                             ("OEBPS/content.opf", OPF, zipfile.ZIP_DEFLATED),
                             ("OEBPS/nav.xhtml", NAV, zipfile.ZIP_DEFLATED),
                             ("OEBPS/style.css", CSS, zipfile.ZIP_DEFLATED),
                             ("OEBPS/one.xhtml", ONE, zipfile.ZIP_DEFLATED),
                             ("OEBPS/two.xhtml", TWO, zipfile.ZIP_DEFLATED)]:
        info = zipfile.ZipInfo(part, (2026, 11, 1, 0, 0, 0))
        info.compress_type = kind
        out.writestr(info, text.encode("utf-8"))
