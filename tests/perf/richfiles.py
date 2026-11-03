"""PERF-10's complex content: what Office files carry beyond text."""

import json
import os
import re
import shutil
import zipfile

import loadfiles
import samples

COMMENTS = "application/vnd.openxmlformats-officedocument.wordprocessingml.comments+xml"
DRAWING = "application/vnd.openxmlformats-officedocument.drawing+xml"
CHART = "application/vnd.openxmlformats-officedocument.drawingml.chart+xml"
C = 'xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart"'
XDR = 'xmlns:xdr="http://schemas.openxmlformats.org/drawingml/2006/spreadsheetDrawing"'
# Word's own namespace attribute; samples.W declares the rest.
WNS = 'xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"'


def override(part, kind):
    return lambda xml: loadfiles.before_last(xml, "</Types>", f'<Override PartName="/{part}" ContentType="{kind}"/>')


def related(rid, kind, target):
    return lambda xml: loadfiles.before_last(xml, "</Relationships>", f'<Relationship Id="{rid}" Type="{loadfiles.REL}/{kind}" Target="{target}"/>')


def docx(path):
    """A TOC, fields, a comment, an insertion, a deletion."""
    toc = loadfiles.field(' TOC \\o "1-3" \\h ', "<w:r><w:t>Page 1</w:t></w:r>")
    fields = loadfiles.field(" PAGE ", "<w:r><w:t>1</w:t></w:r>") + loadfiles.field(" DATE ", "<w:r><w:t>2026-01-01</w:t></w:r>")
    review = ('<w:p><w:commentRangeStart w:id="1"/><w:r><w:t>Reviewed text</w:t></w:r><w:commentRangeEnd w:id="1"/>'
              '<w:r><w:commentReference w:id="1"/></w:r>'
              '<w:ins w:id="2" w:author="Ely" w:date="2026-01-01T00:00:00Z"><w:r><w:t xml:space="preserve"> inserted</w:t></w:r></w:ins>'
              '<w:del w:id="3" w:author="Ely" w:date="2026-01-01T00:00:00Z"><w:r><w:delText xml:space="preserve"> deleted</w:delText></w:r></w:del></w:p>')
    head = f"<w:p>{toc}</w:p><w:p>{fields}</w:p>{review}"
    comments = (f'<w:comments {WNS}><w:comment w:id="1" w:author="Ely" w:date="2026-01-01T00:00:00Z">'
                "<w:p><w:r><w:t>A comment</w:t></w:r></w:p></w:comment></w:comments>")
    loadfiles.rewrite(path, {"word/document.xml": lambda xml: xml.replace("<w:body>", "<w:body>" + head, 1),
                   "word/_rels/document.xml.rels": related("rCom", "comments", "comments.xml"),
                   "[Content_Types].xml": override("word/comments.xml", COMMENTS)},
            {"word/comments.xml": comments})


def docx_kept(path):
    """What a saved rich DOCX still holds."""
    with zipfile.ZipFile(path) as archive:
        xml = archive.read("word/document.xml").decode("utf-8")
        names = archive.namelist()
    return {"comment kept": "word/comments.xml" in names and "commentReference" in xml,
            "insertion kept": "<w:ins " in xml, "deletion kept": "<w:del " in xml,
            "TOC kept": "TOC \\o" in xml, "PAGE field kept": re.search(r"instrText[^>]*>\s*PAGE", xml) is not None,
            "20 pictures kept": sum(name.startswith("word/media/") for name in names) >= 20}


def xlsx(path, rows):
    """A line chart over Data!F on the Calc sheet."""
    span = f"Data!$F$1:$F${min(rows, 200)}"
    chart = (f'<c:chartSpace {C} xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><c:chart><c:plotArea>'
             f'<c:lineChart><c:grouping val="standard"/><c:ser><c:idx val="0"/><c:order val="0"/><c:val><c:numRef>'
             f'<c:f>{span}</c:f></c:numRef></c:val></c:ser><c:axId val="1"/><c:axId val="2"/></c:lineChart>'
             '<c:catAx><c:axId val="1"/><c:scaling/><c:axPos val="b"/><c:crossAx val="2"/></c:catAx>'
             '<c:valAx><c:axId val="2"/><c:scaling/><c:axPos val="l"/><c:crossAx val="1"/></c:valAx></c:plotArea></c:chart></c:chartSpace>')
    anchor = (f'<xdr:wsDr {XDR} xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="{loadfiles.REL}" {C}>'
              '<xdr:twoCellAnchor><xdr:from><xdr:col>5</xdr:col><xdr:colOff>0</xdr:colOff><xdr:row>1</xdr:row><xdr:rowOff>0</xdr:rowOff></xdr:from>'
              '<xdr:to><xdr:col>12</xdr:col><xdr:colOff>0</xdr:colOff><xdr:row>18</xdr:row><xdr:rowOff>0</xdr:rowOff></xdr:to>'
              '<xdr:graphicFrame macro=""><xdr:nvGraphicFramePr><xdr:cNvPr id="2" name="Chart 1"/><xdr:cNvGraphicFramePr/></xdr:nvGraphicFramePr>'
              '<xdr:xfrm><a:off x="0" y="0"/><a:ext cx="0" cy="0"/></xdr:xfrm><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/chart">'
              '<c:chart r:id="rChart"/></a:graphicData></a:graphic></xdr:graphicFrame><xdr:clientData/></xdr:twoCellAnchor></xdr:wsDr>')
    with zipfile.ZipFile(path) as archive:
        sheet = loadfiles.sheet_part(archive, "Calc")
    folder, name = sheet.rsplit("/", 1)
    loadfiles.rewrite(path, {sheet: lambda xml: xml.replace("</sheetData>", '</sheetData><drawing r:id="rDraw"/>', 1),
                   "[Content_Types].xml": lambda xml: override("xl/charts/chart1.xml", CHART)(override("xl/drawings/drawing1.xml", DRAWING)(xml))},
            {f"{folder}/_rels/{name}.rels": f'<Relationships xmlns="{loadfiles.PKG}"><Relationship Id="rDraw" Type="{loadfiles.REL}/drawing" '
                                            'Target="../drawings/drawing1.xml"/></Relationships>',
             "xl/drawings/drawing1.xml": anchor, "xl/charts/chart1.xml": chart,
             "xl/drawings/_rels/drawing1.xml.rels": f'<Relationships xmlns="{loadfiles.PKG}"><Relationship Id="rChart" Type="{loadfiles.REL}/chart" '
                                                   'Target="../charts/chart1.xml"/></Relationships>'})


def xlsx_kept(path):
    with zipfile.ZipFile(path) as archive:
        names = archive.namelist()
    sort = loadfiles.formula(path, "Calc", "D1") or ""
    volatile = loadfiles.formula(path, "Calc", "A14") or ""
    return {"chart kept": any(re.fullmatch(r"xl/charts/chart\d+\.xml", name) for name in names),
            "dynamic array SORT kept": "SORT" in sort.upper(), "volatile NOW kept": "NOW" in volatile.upper()}


def timing(spid):
    """PowerPoint's click-to-fade entrance on shape `spid`."""
    target = f'<p:tgtEl><p:spTgt spid="{spid}"/></p:tgtEl>'
    return ('<p:timing><p:tnLst><p:par><p:cTn id="1" dur="indefinite" restart="never" nodeType="tmRoot"><p:childTnLst>'
            '<p:seq concurrent="1" nextAc="seek"><p:cTn id="2" dur="indefinite" nodeType="mainSeq"><p:childTnLst><p:par>'
            '<p:cTn id="3" fill="hold"><p:stCondLst><p:cond delay="indefinite"/></p:stCondLst><p:childTnLst><p:par>'
            '<p:cTn id="4" fill="hold"><p:stCondLst><p:cond delay="0"/></p:stCondLst><p:childTnLst><p:par>'
            '<p:cTn id="5" presetID="10" presetClass="entr" presetSubtype="0" fill="hold" nodeType="clickEffect">'
            '<p:stCondLst><p:cond delay="0"/></p:stCondLst><p:childTnLst><p:set><p:cBhvr><p:cTn id="6" dur="1" fill="hold">'
            f'<p:stCondLst><p:cond delay="0"/></p:stCondLst></p:cTn>{target}<p:attrNameLst><p:attrName>style.visibility'
            '</p:attrName></p:attrNameLst></p:cBhvr><p:to><p:strVal val="visible"/></p:to></p:set>'
            f'<p:animEffect transition="in" filter="fade"><p:cBhvr><p:cTn id="7" dur="500"/>{target}</p:cBhvr></p:animEffect>'
            '</p:childTnLst></p:cTn></p:par></p:childTnLst></p:cTn></p:par></p:childTnLst></p:cTn></p:par></p:childTnLst>'
            '</p:cTn><p:prevCondLst><p:cond evt="onPrev" delay="0"><p:tgtEl><p:sldTgt/></p:tgtEl></p:cond></p:prevCondLst>'
            '<p:nextCondLst><p:cond evt="onNext" delay="0"><p:tgtEl><p:sldTgt/></p:tgtEl></p:cond></p:nextCondLst></p:seq>'
            '</p:childTnLst></p:cTn></p:par></p:tnLst></p:timing>')


def pptx(path):
    """Every added slide's picture fades in on click."""
    with zipfile.ZipFile(path) as archive:
        slides = [name for name in archive.namelist() if re.fullmatch(r"ppt/slides/slide\d+\.xml", name)]
    changes = {}
    for name in slides:
        n = int(re.search(r"(\d+)\.xml$", name).group(1))
        if n > 9:
            changes[name] = lambda xml, n=n: loadfiles.before_last(xml, "</p:sld>", timing(9000 + n))
    loadfiles.rewrite(path, changes)


def pptx_kept(path):
    with zipfile.ZipFile(path) as archive:
        slides = [archive.read(name).decode("utf-8") for name in archive.namelist() if re.fullmatch(r"ppt/slides/slide\d+\.xml", name)]
    pictures = sum(xml.count("<p:pic>") for xml in slides)
    return {f"{len(slides)} slides kept": len(slides) >= 45, f"{pictures} pictures kept": pictures >= 36,
            "36 animations kept": sum("animEffect" in xml for xml in slides) >= 36}


def pages_docx(path, pages):
    """`pages` pages; tables every 5, pictures every 10."""
    words = "office local file engine page table slide note draft review".split()
    body, rels, media = [], "", {}
    for page in range(pages):
        breaks = '<w:r><w:br w:type="page"/></w:r>' if page else ""
        body.append(f'<w:p><w:pPr><w:pStyle w:val="Heading1"/></w:pPr>{breaks}<w:r><w:t>Page {page + 1}</w:t></w:r></w:p>')
        body += [loadfiles.para(" ".join(words[(page + k + i) % 10] for i in range(60))) for k in range(5)]
        if page % 5 == 0:
            cells = "".join("<w:tr>" + f"<w:tc><w:p><w:r><w:t>{page}.{r}</w:t></w:r></w:p></w:tc>" * 4 + "</w:tr>" for r in range(6))
            body.append(f'<w:tbl><w:tblPr><w:tblW w:w="0" w:type="auto"/></w:tblPr>{cells}</w:tbl>')
        if page % 10 == 0:
            n = page // 10 + 1
            body.append(samples.picture(n))
            rels += f'<Relationship Id="img{n}" Type="{loadfiles.REL}/image" Target="media/p{n}.png"/>'
            media[f"word/media/p{n}.png"] = samples.noise_png(n, 200, 150)
    body.append(loadfiles.para(f"{loadfiles.TOKEN}0"))
    loadfiles.word(path, body, rels, media)


def newest_points(app, work):
    """Each edited document's newest recovery point, copied out."""
    folder, found = os.path.join(app.home, app.slug, "recovery"), {}
    with open(os.path.join(folder, "index.json"), encoding="utf-8") as index:
        for entry in json.load(index):
            points = sorted(os.listdir(os.path.join(folder, entry["slot"], "points")))
            if points:
                copy = os.path.join(work, f"point-{app.slug[4:]}-{entry['name']}")
                found[entry["name"]] = shutil.copy(os.path.join(folder, entry["slot"], "points", points[-1]), copy)
    return found


def held(slug, point):
    """Keys and the round-1 edit a recovery point holds."""
    if slug == "ely-table":
        typed = loadfiles.cell(point, "Typing", "A1") or ""
        return typed.count("1"), loadfiles.cell(point, "Data", "H1")
    return loadfiles.text_of(point).count("x"), loadfiles.tokens(point)


# Each huge sample's size: pages, rows, slide copies, notes.
HUGE = {"ely-doc": 500, "ely-table": 60000, "ely-show": 20, "ely-mark": 6000}


def huge_sample(slug, path):
    n = HUGE[slug]
    if slug == "ely-doc":
        pages_docx(path, n)
    elif slug == "ely-table":
        loadfiles.workbook(path, n, typing=True)
        xlsx(path, n)
    elif slug == "ely-show":
        loadfiles.presentation(path, n)
    else:
        with open(path, "w", encoding="utf-8", newline="\n") as note:
            body = loadfiles.small_md(0).replace(f"{loadfiles.TOKEN}0", "") * n
            note.write(f"# Typing\n\nA paragraph.\n\n{loadfiles.TOKEN}0\n\n{body}")


def huge_kept(slug, path):
    """What a saved huge sample must still hold."""
    n = HUGE[slug]
    if slug == "ely-table":
        return {**xlsx_kept(path), f"Data!A{n} kept": loadfiles.cell(path, "Data", f"A{n}") is not None,
                f"Calc!B{n} chain kept": loadfiles.formula(path, "Calc", f"B{n}") is not None}
    if slug == "ely-mark":
        return {f"{n} notes kept": loadfiles.text_of(path).count("| 19 | row 19 | 361 |") == n}
    with zipfile.ZipFile(path) as archive:
        names = archive.namelist()
        if slug == "ely-show":
            slides = [archive.read(name).decode("utf-8") for name in names if re.fullmatch(r"ppt/slides/slide\d+\.xml", name)]
            return {f"{9 * (n + 1)} slides kept": len(slides) == 9 * (n + 1),
                    f"{9 * n} pictures kept": sum(xml.count("<p:pic>") for xml in slides) >= 9 * n}
        xml = archive.read("word/document.xml").decode("utf-8")
    pages = len(re.findall(r"Page \d+", loadfiles.text_of(path)))
    return {f"{n} pages kept": pages == n, f"{n // 5} tables kept": xml.count("<w:tbl>") >= n // 5,
            f"{n // 10} pictures kept": sum(name.startswith("word/media/") for name in names) >= n // 10}
