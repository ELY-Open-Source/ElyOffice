#!/usr/bin/env python3
"""A DOCX table: pictures in one column, text beside."""

# bigtable.py out rows width height jpeg|png inline|anchor|tight [inches [cantsplit]]

import io
import random
import re
import sys
import zipfile

from PIL import Image

OUT, ROWS, WIDTH, HEIGHT, FORMAT, PLACE = sys.argv[1], int(sys.argv[2]), int(sys.argv[3]), int(sys.argv[4]), sys.argv[5], sys.argv[6]
EMU = 914400  # per inch

TYPES = """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
<Default Extension="xml" ContentType="application/xml"/>
<Default Extension="jpeg" ContentType="image/jpeg"/>
<Default Extension="png" ContentType="image/png"/>
<Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
</Types>"""
RELS = """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>
</Relationships>"""
NS = ('xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" '
      'xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" '
      'xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing" '
      'xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" '
      'xmlns:pic="http://schemas.openxmlformats.org/drawingml/2006/picture"')
WORDS = "产品检查记录现场照片说明设备编号位置状态正常需要复查已处理负责人日期备注"


def image(seed):
    """Smooth noise at WIDTH x HEIGHT, like a photo."""
    rng = random.Random(seed)
    small = Image.new("RGB", (WIDTH // 8, HEIGHT // 8))
    small.putdata([(rng.randrange(256), rng.randrange(256), rng.randrange(256)) for _ in range(small.width * small.height)])
    big = small.resize((WIDTH, HEIGHT), Image.BICUBIC)
    out = io.BytesIO()
    big.save(out, "JPEG", quality=70) if FORMAT == "jpeg" else big.save(out, "PNG")
    return out.getvalue()


def picture(n):
    inches = float(sys.argv[7]) if len(sys.argv) > 7 else 3.0
    cx, cy = int(inches * EMU), int(inches * EMU * HEIGHT / WIDTH)
    graphic = (f'<wp:extent cx="{cx}" cy="{cy}"/>' + ({'anchor': '<wp:wrapSquare wrapText="bothSides"/>', 'tight': '<wp:wrapTight wrapText="bothSides"><wp:wrapPolygon edited="0"><wp:start x="0" y="0"/><wp:lineTo x="0" y="21600"/><wp:lineTo x="21600" y="21600"/><wp:lineTo x="21600" y="0"/><wp:lineTo x="0" y="0"/></wp:wrapPolygon></wp:wrapTight>'}.get(PLACE, ''))
               + f'<wp:docPr id="{n}" name="Picture {n}"/>'
               f'<a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/picture"><pic:pic>'
               f'<pic:nvPicPr><pic:cNvPr id="{n}" name="p{n}.{FORMAT}"/><pic:cNvPicPr/></pic:nvPicPr>'
               f'<pic:blipFill><a:blip r:embed="rIdImg{n}"/><a:stretch><a:fillRect/></a:stretch></pic:blipFill>'
               f'<pic:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="{cx}" cy="{cy}"/></a:xfrm>'
               f'<a:prstGeom prst="rect"><a:avLst/></a:prstGeom></pic:spPr></pic:pic></a:graphicData></a:graphic>')
    if PLACE == "inline":
        return f'<w:r><w:drawing><wp:inline>{graphic}</wp:inline></w:drawing></w:r>'
    return ('<w:r><w:drawing><wp:anchor distT="0" distB="0" distL="114300" distR="114300" simplePos="0" relativeHeight="1" '
            'behindDoc="0" locked="0" layoutInCell="1" allowOverlap="1"><wp:simplePos x="0" y="0"/>'
            '<wp:positionH relativeFrom="column"><wp:posOffset>0</wp:posOffset></wp:positionH>'
            '<wp:positionV relativeFrom="paragraph"><wp:posOffset>0</wp:posOffset></wp:positionV>'
            f'{re.sub("<wp:wrap", "<wp:effectExtent l=\"0\" t=\"0\" r=\"0\" b=\"0\"/><wp:wrap", graphic, count=1)}'
            '</wp:anchor></w:drawing></w:r>')


def text(n):
    rng = random.Random(n)
    paras = []
    for _ in range(4):
        line = "".join(rng.choice(WORDS) for _ in range(rng.randrange(20, 60)))
        paras.append(f"<w:p><w:r><w:t>{n}. {line}</w:t></w:r></w:p>")
    return "".join(paras)


def main():
    rows = []
    for n in range(1, ROWS + 1):
        rows.append(f'<w:tr>{"<w:trPr><w:cantSplit/></w:trPr>" if len(sys.argv) > 8 else ""}<w:tc><w:tcPr><w:tcW w:w="4500" w:type="dxa"/></w:tcPr><w:p>{picture(n)}</w:p></w:tc>'
                    f'<w:tc><w:tcPr><w:tcW w:w="4500" w:type="dxa"/></w:tcPr>{text(n)}</w:tc></w:tr>')
    body = (f'<w:tbl><w:tblPr><w:tblW w:w="9000" w:type="dxa"/><w:tblBorders>'
            + "".join(f'<w:{s} w:val="single" w:sz="4"/>' for s in ["top", "left", "bottom", "right", "insideH", "insideV"])
            + f'</w:tblBorders></w:tblPr><w:tblGrid><w:gridCol w:w="4500"/><w:gridCol w:w="4500"/></w:tblGrid>{"".join(rows)}</w:tbl>')
    document = f'<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:document {NS}><w:body>{body}<w:p/></w:body></w:document>'
    links = "".join(f'<Relationship Id="rIdImg{n}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" '
                    f'Target="media/p{n}.{FORMAT}"/>' for n in range(1, ROWS + 1))
    doc_rels = ('<?xml version="1.0" encoding="UTF-8" standalone="yes"?>'
                f'<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">{links}</Relationships>')
    with zipfile.ZipFile(OUT, "w", zipfile.ZIP_DEFLATED) as z:
        z.writestr("[Content_Types].xml", TYPES)
        z.writestr("_rels/.rels", RELS)
        z.writestr("word/document.xml", document)
        z.writestr("word/_rels/document.xml.rels", doc_rels)
        for n in range(1, ROWS + 1):
            z.writestr(f"word/media/p{n}.{FORMAT}", image(n), zipfile.ZIP_STORED)


main()
