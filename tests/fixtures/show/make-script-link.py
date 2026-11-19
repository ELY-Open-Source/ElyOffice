"""Writes script-link.pptx: t.pptx with a click on a script URL."""
import os
import sys
import zipfile

here = os.path.dirname(os.path.abspath(__file__))
source = os.path.join(here, "..", "office", "t.pptx")
out = os.path.join(here, "script-link.pptx")
SHAPE = '<p:cNvPr id="463" name="">'
REL = ('<Relationship Id="rIdS" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink" '
       'Target="vnd.sun.star.script:Standard.Module1.Touch?language=Basic&amp;location=document" TargetMode="External"/>')

with zipfile.ZipFile(source) as old, zipfile.ZipFile(out, "w", zipfile.ZIP_DEFLATED) as new:
    for item in old.infolist():
        data = old.read(item)
        if item.filename == "ppt/slides/slide1.xml":
            text = data.decode("utf-8")
            if text.count(SHAPE) != 1:
                sys.exit("make-script-link.py: slide 1's first shape moved")
            data = text.replace(SHAPE, SHAPE[:-1] + '><a:hlinkClick r:id="rIdS"/>').encode("utf-8")
        elif item.filename == "ppt/slides/_rels/slide1.xml.rels":
            data = data.replace(b"</Relationships>", REL.encode("utf-8") + b"</Relationships>")
        new.writestr(item, data)
