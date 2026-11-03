"""PERF-10 samples, written by Python, and their read-back."""

import os
import re
import shutil
import zipfile
import xml.etree.ElementTree as ET
from xml.sax.saxutils import escape

import drive
import richfiles
import samples

TOKEN = "ELYEDIT"
EXT = {"ely-doc": "docx", "ely-table": "xlsx", "ely-show": "pptx", "ely-mark": "md"}
W = "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}"
A = "{http://schemas.openxmlformats.org/drawingml/2006/main}"
S = "{http://schemas.openxmlformats.org/spreadsheetml/2006/main}"
R = "{http://schemas.openxmlformats.org/officeDocument/2006/relationships}"
P = "{http://schemas.openxmlformats.org/package/2006/relationships}"
REL = R[1:-1]
PKG = P[1:-1]
DOC_MAIN = "application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"
BOOK_MAIN = "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"
SHEET = "application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"
SLIDE = "application/vnd.openxmlformats-officedocument.presentationml.slide+xml"
VML = 'xmlns:v="urn:schemas-microsoft-com:vml" xmlns:o="urn:schemas-microsoft-com:office:office"'
# Linked sources hold newer values than the links cached.
RATES_NOW = [2 + m / 100 for m in range(1, 13)]
RATES_CACHED = [1 + m / 100 for m in range(1, 13)]
INCLUDED_NOW, INCLUDED_CACHED = "Included after", "Included before"


def package(path, main, kind, parts, rels="", overrides=None):
    """An OOXML package: `main`, its rels, and other parts."""
    overrides = {main: kind, **(overrides or {})}
    types = "".join(f'<Override PartName="/{part}" ContentType="{t}"/>' for part, t in overrides.items())
    folder, name = main.rsplit("/", 1)
    with zipfile.ZipFile(path, "w", zipfile.ZIP_DEFLATED) as archive:
        archive.writestr("[Content_Types].xml", f'<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">'
                         f'<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>'
                         '<Default Extension="xml" ContentType="application/xml"/>'
                         f'<Default Extension="png" ContentType="image/png"/>{types}</Types>')
        archive.writestr("_rels/.rels", f'<Relationships xmlns="{PKG}"><Relationship Id="r1" '
                         f'Type="{REL}/officeDocument" Target="{main}"/></Relationships>')
        archive.writestr(f"{folder}/_rels/{name}.rels", f'<Relationships xmlns="{PKG}">{rels}</Relationships>')
        for part, data in parts.items():
            archive.writestr(part, data)


def rewrite(path, changes, added=None):
    """`path` with parts changed by name and `added` written."""
    with zipfile.ZipFile(path) as old:
        parts = [(info, old.read(info)) for info in old.infolist()]
    missing = set(changes) - {info.filename for info, _ in parts}
    if missing:
        raise RuntimeError(f"{path} has no {sorted(missing)}")
    with zipfile.ZipFile(path, "w", zipfile.ZIP_DEFLATED) as new:
        for info, data in parts:
            change = changes.get(info.filename)
            new.writestr(info, change(data.decode("utf-8")).encode("utf-8") if change else data)
        for name, data in (added or {}).items():
            new.writestr(name, data)


def before_last(text, mark, insert):
    at = text.rfind(mark)
    if at < 0:
        raise RuntimeError(f"no {mark!r} to insert before")
    return text[:at] + insert + text[at:]


def para(text):
    return f'<w:p><w:r><w:t xml:space="preserve">{escape(text)}</w:t></w:r></w:p>'


def word(path, body, rels="", parts=None):
    document = f'<w:document {samples.W} {VML}><w:body>{"".join(body)}<w:sectPr/></w:body></w:document>'
    package(path, "word/document.xml", DOC_MAIN, {"word/document.xml": document, **(parts or {})}, rels)


def small_docx(path, n):
    """Three pages, a table and the token."""
    text = " ".join(["Ely Office opens many documents at once."] * 12)
    body = [para(f"Note {n}")] + [para(f"{p} {text}") for p in range(36)]
    rows = "".join("<w:tr>" + "".join(f"<w:tc><w:p><w:r><w:t>R{r}C{c}</w:t></w:r></w:p></w:tc>" for c in range(4)) + "</w:tr>"
                   for r in range(10))
    body += [f'<w:tbl><w:tblPr><w:tblW w:w="0" w:type="auto"/></w:tblPr>{rows}</w:tbl>', para(f"{TOKEN}0")]
    word(path, body)


def field(instr, result):
    """A complex field and its cached result runs."""
    return ('<w:r><w:fldChar w:fldCharType="begin"/></w:r><w:r><w:instrText xml:space="preserve">'
            f'{escape(instr)}</w:instrText></w:r><w:r><w:fldChar w:fldCharType="separate"/></w:r>'
            f'{result}<w:r><w:fldChar w:fldCharType="end"/></w:r>')


def linked_docx(path, folder):
    """INCLUDETEXT, a linked picture and an OLE link."""
    def at(name):
        return os.path.abspath(os.path.join(folder, name))

    include = at("include.docx").replace("\\", "\\\\")
    cx, cy = 1800000, 1800000
    picture = (f'<w:p><w:r><w:drawing><wp:inline><wp:extent cx="{cx}" cy="{cy}"/><wp:docPr id="1" name="linked"/>'
               '<a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/picture">'
               '<pic:pic><pic:nvPicPr><pic:cNvPr id="1" name="picture.png"/><pic:cNvPicPr/></pic:nvPicPr>'
               '<pic:blipFill><a:blip r:link="rPic"/><a:stretch><a:fillRect/></a:stretch></pic:blipFill>'
               f'<pic:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="{cx}" cy="{cy}"/></a:xfrm>'
               '<a:prstGeom prst="rect"><a:avLst/></a:prstGeom></pic:spPr></pic:pic></a:graphicData></a:graphic>'
               '</wp:inline></w:drawing></w:r></w:p>')
    ole = ('<w:r><w:object w:dxaOrig="2000" w:dyaOrig="1000"><v:shape id="_x0000_i1025" type="#_x0000_t75" '
           'style="width:100pt;height:50pt" o:ole=""><v:imagedata r:id="rPreview" o:title=""/></v:shape>'
           '<o:OLEObject Type="Link" ProgID="Excel.Sheet.12" ShapeID="_x0000_i1025" DrawAspect="Content" r:id="rOle" '
           'UpdateMode="Always"><o:LinkType>EnhancedMetaFile</o:LinkType><o:LockedField>false</o:LockedField>'
           '</o:OLEObject></w:object></w:r>')
    source = at("source.xlsx").replace("\\", "\\\\")
    body = [para(f"{TOKEN}0"), para("Linked sources follow."),
            f'<w:p>{field(f" INCLUDETEXT \"{include}\" ", f"<w:r><w:t>{INCLUDED_CACHED}</w:t></w:r>")}</w:p>',
            picture,
            f'<w:p>{field(f" LINK Excel.Sheet.12 \"{source}\" \"Rates!R1C1:R12C2\" \\a \\p ", ole)}</w:p>']
    uri = lambda name: "file:///" + at(name).replace("\\", "/")
    rels = (f'<Relationship Id="rPic" Type="{REL}/image" Target="{uri("picture.png")}" TargetMode="External"/>'
            f'<Relationship Id="rOle" Type="{REL}/oleObject" Target="{uri("source.xlsx")}" TargetMode="External"/>'
            f'<Relationship Id="rPreview" Type="{REL}/image" Target="media/preview.png"/>')
    word(path, body, rels, {"word/media/preview.png": samples.noise_png(9, 120, 60)})


def cells(r, values):
    out = []
    for c, value in enumerate(values):
        ref = f"{chr(65 + c)}{r}"
        if value is None:
            continue
        if isinstance(value, tuple):
            out.append(f'<c r="{ref}"><f t="array" ref="{value[1]}">{escape(value[0])}</f></c>')
            continue
        if isinstance(value, str) and value.startswith("="):
            out.append(f'<c r="{ref}"><f>{escape(value[1:])}</f></c>')
        elif isinstance(value, str):
            out.append(f'<c r="{ref}" t="inlineStr"><is><t>{escape(value)}</t></is></c>')
        else:
            out.append(f'<c r="{ref}"><v>{value}</v></c>')
    return f'<row r="{r}">{"".join(out)}</row>'


def workbook(path, rows, linked=False, typing=False):
    """Data, formulas over it, a chain; Links reads source.xlsx."""
    data = [cells(r, [(r * 7 + c * 13) % 100 for c in range(5)] +
                  [f"=SUM(A{r}:E{r})*IF(MOD(A{r},2)=0,1,-1)", f"cat{r % 7}"]) for r in range(1, rows + 1)]
    span = lambda col: f"Data!${col}$1:${col}${rows}"
    calc = ["=Data!A1*2", f'=SUMIFS({span("F")},{span("G")},"cat3",{span("A")},">50")',
            f'=COUNTIFS({span("G")},"cat1",{span("B")},"<40")', f"=VLOOKUP(42,Data!$A$1:$F${rows},6,FALSE)",
            f"=INDEX({span('F')},MATCH(MAX({span('B')}),{span('B')},0))", f'=AVERAGEIFS({span("C")},{span("D")},"<30")',
            f"=SUMPRODUCT(({span('A')}>10)*({span('B')}<90)*{span('F')})", f'=TEXT(SUM(Amounts),"#,##0")',
            f"=IFERROR(1/(Data!A1-Data!A1),-1)", f"=ROUND(STDEV({span('C')}),3)", f"=MEDIAN({span('D')})",
            f"=LARGE({span('E')},5)", f'=IF(SUM(Amounts)>0,"up",IF(SUM(Amounts)<0,"down","flat"))',
            "=IF(NOW()>0,RAND()*0+1,0)", "=SUM(OFFSET(Data!$A$1,0,0,10,1))", '=INDIRECT("Data!B2")']
    dynamic = {1: [None, ("_xlfn._xlws.SORT(Data!$A$1:$A$10)", "D1:D10"), ("_xlfn.UNIQUE(Data!$G$1:$G$100)", "E1:E7")]}
    sheet_rows = [cells(r, [f, "=Data!B1" if r == 1 else f"=B{r - 1}+Data!B{r}*0.5", *dynamic.get(r, [])])
                  for r, f in enumerate(calc, 1)]
    sheet_rows += [cells(r, [None, f"=B{r - 1}+Data!B{r}*0.5"]) for r in range(len(calc) + 1, rows + 1)]
    array = f'<row r="{rows + 1}"><c r="C{rows + 1}"><f t="array" ref="C{rows + 1}">SUM({span("A")}*{span("B")})</f></c></row>'
    ns = f'xmlns="{S[1:-1]}" xmlns:r="{REL}"'
    sheets = ([("Typing", "")] if typing else []) + [("Data", "".join(data)), ("Calc", "".join(sheet_rows) + array)]
    parts, overrides = {}, {}
    if linked:
        links = (f'<row r="1"><c r="A1"><f>[1]Rates!$B$1</f><v>{RATES_CACHED[0]}</v></c></row>'
                 f'<row r="2"><c r="A2"><f>SUM([1]Rates!$B$1:$B$12)</f><v>{sum(RATES_CACHED)}</v></c></row>')
        sheets.append(("Links", links))
        cached = "".join(f'<row r="{m}"><cell r="B{m}"><v>{v}</v></cell></row>' for m, v in enumerate(RATES_CACHED, 1))
        parts["xl/externalLinks/externalLink1.xml"] = (f'<externalLink {ns}><externalBook r:id="rId1"><sheetNames>'
                                                       f'<sheetName val="Rates"/></sheetNames><sheetDataSet><sheetData sheetId="0">'
                                                       f'{cached}</sheetData></sheetDataSet></externalBook></externalLink>')
        parts["xl/externalLinks/_rels/externalLink1.xml.rels"] = (
            f'<Relationships xmlns="{PKG}"><Relationship Id="rId1" Type="{REL}/externalLinkPath" '
            'Target="source.xlsx" TargetMode="External"/></Relationships>')
        overrides["xl/externalLinks/externalLink1.xml"] = "application/vnd.openxmlformats-officedocument.spreadsheetml.externalLink+xml"
    listed = "".join(f'<sheet name="{name}" sheetId="{i}" r:id="s{i}"/>' for i, (name, _) in enumerate(sheets, 1))
    rels = "".join(f'<Relationship Id="s{i}" Type="{REL}/worksheet" Target="worksheets/sheet{i}.xml"/>'
                   for i in range(1, len(sheets) + 1))
    external = ""
    if linked:
        rels += f'<Relationship Id="x1" Type="{REL}/externalLink" Target="externalLinks/externalLink1.xml"/>'
        external = '<externalReferences><externalReference r:id="x1"/></externalReferences>'
    for i, (_, body) in enumerate(sheets, 1):
        parts[f"xl/worksheets/sheet{i}.xml"] = f"<worksheet {ns}><sheetData>{body}</sheetData></worksheet>"
        overrides[f"xl/worksheets/sheet{i}.xml"] = SHEET
    parts["xl/workbook.xml"] = (f"<workbook {ns}><sheets>{listed}</sheets>{external}<definedNames>"
                                f'<definedName name="Amounts">{span("F")}</definedName></definedNames></workbook>')
    package(path, "xl/workbook.xml", BOOK_MAIN, parts, rels, overrides)


def source_xlsx(path, rates=RATES_NOW):
    rows = "".join(cells(m, [m, v]) for m, v in enumerate(rates, 1))
    ns = f'xmlns="{S[1:-1]}" xmlns:r="{REL}"'
    package(path, "xl/workbook.xml", BOOK_MAIN,
            {"xl/workbook.xml": f'<workbook {ns}><sheets><sheet name="Rates" sheetId="1" r:id="s1"/></sheets></workbook>',
             "xl/worksheets/sheet1.xml": f"<worksheet {ns}><sheetData>{rows}</sheetData></worksheet>"},
            f'<Relationship Id="s1" Type="{REL}/worksheet" Target="worksheets/sheet1.xml"/>',
            {"xl/worksheets/sheet1.xml": SHEET})


def token_shape(xml):
    shape = ('<p:sp><p:nvSpPr><p:cNvPr id="9999" name="Ely edits"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>'
             '<p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="2400000" cy="400000"/></a:xfrm>'
             '<a:prstGeom prst="rect"><a:avLst/></a:prstGeom></p:spPr><p:txBody><a:bodyPr/><a:lstStyle/>'
             f'<a:p><a:r><a:rPr lang="en-US"/><a:t>{TOKEN}0</a:t></a:r></a:p></p:txBody></p:sp>')
    return before_last(xml, "</p:spTree>", shape)


def picture_shape(n):
    return (f'<p:pic><p:nvPicPr><p:cNvPr id="{9000 + n}" name="noise {n}"/><p:cNvPicPr/><p:nvPr/></p:nvPicPr>'
            '<p:blipFill><a:blip r:embed="rNoise"/><a:stretch><a:fillRect/></a:stretch></p:blipFill>'
            '<p:spPr><a:xfrm><a:off x="6000000" y="3200000"/><a:ext cx="3600000" cy="2400000"/></a:xfrm>'
            '<a:prstGeom prst="rect"><a:avLst/></a:prstGeom></p:spPr></p:pic>')


def presentation(path, copies):
    """t.pptx with the token, plus `copies` of its slides."""
    shutil.copy(os.path.join(drive.ROOT, "tests", "fixtures", "office", "t.pptx"), path)
    with zipfile.ZipFile(path) as old:
        slides = {n: (old.read(f"ppt/slides/slide{n}.xml").decode(), old.read(f"ppt/slides/_rels/slide{n}.xml.rels").decode())
                  for n in range(1, 10)}
    added, ids, rels, types = {}, "", "", ""
    for k in range(copies):
        for n, (xml, rel) in slides.items():
            new = 9 * (k + 1) + n
            added[f"ppt/slides/slide{new}.xml"] = before_last(xml, "</p:spTree>", picture_shape(new))
            added[f"ppt/slides/_rels/slide{new}.xml.rels"] = before_last(
                rel, "</Relationships>", f'<Relationship Id="rNoise" Type="{REL}/image" Target="../media/noise{new}.png"/>')
            added[f"ppt/media/noise{new}.png"] = samples.noise_png(new, 480, 360)
            ids += f'<p:sldId id="{256 + new}" r:id="rS{new}"/>'
            rels += f'<Relationship Id="rS{new}" Type="{REL}/slide" Target="slides/slide{new}.xml"/>'
            types += f'<Override PartName="/ppt/slides/slide{new}.xml" ContentType="{SLIDE}"/>'
    rewrite(path, {"ppt/slides/slide1.xml": token_shape,
                   "ppt/presentation.xml": lambda xml: before_last(xml, "</p:sldIdLst>", ids),
                   "ppt/_rels/presentation.xml.rels": lambda xml: before_last(xml, "</Relationships>", rels),
                   "[Content_Types].xml": lambda xml: before_last(xml, "</Types>", types)}, added)


def small_md(n):
    rows = "\n".join(f"| {r} | row {r} | {r * r} |" for r in range(20))
    return (f"# Note {n}\n\n{TOKEN}0\n\n" + "Ely Office opens many notes at once. " * 20 +
            f"\n\n- one\n- **two**\n- `three`\n\n| n | name | square |\n| --- | --- | --- |\n{rows}\n\n"
            "```python\nprint('hello')\n```\n\n$$e^{i\\pi} + 1 = 0$$\n")


def made(folder):
    """The big samples, written once into `folder`."""
    os.makedirs(folder, exist_ok=True)
    makers = {"big.docx": big_docx, "big.xlsx": big_xlsx, "big.pptx": big_pptx, "big.md": big_md}
    for name, make in makers.items():
        path = os.path.join(folder, name)
        if not os.path.exists(path):
            make(path + ".part")
            os.replace(path + ".part", path)
    return folder


def big_docx(path):
    samples.docx(path)
    rewrite(path, {"word/document.xml": lambda xml: before_last(xml, "</w:body>", para(f"{TOKEN}0"))})
    richfiles.docx(path)


def big_xlsx(path):
    workbook(path, 20000, linked=True)
    richfiles.xlsx(path, 20000)


def big_pptx(path):
    presentation(path, 4)
    richfiles.pptx(path)


def big_md(path):
    samples.markdown(path)
    with open(path, "r+", encoding="utf-8", newline="\n") as note:
        text = note.read()
        note.seek(0)
        note.write(f"{TOKEN}0\n\n{text}")


def role(slug, i):
    """Every tenth big; Doc's next is linked; the rest small."""
    if i % 10 == 0:
        return "big"
    return "linked" if slug == "ely-doc" and i % 10 == 1 else "small"


def sources(work, rates=RATES_NOW, seed=4):
    """What the linked documents point at; later, newer ones."""
    source_xlsx(os.path.join(work, "source.xlsx"), rates)
    if seed == 4:
        word(os.path.join(work, "include.docx"), [para(INCLUDED_NOW)])
    with open(os.path.join(work, "picture.png"), "wb") as png:
        png.write(samples.noise_png(seed, 64, 64))


def documents(folder, work, slug, n):
    """Documents opened at launch, and the front one."""
    sources(work)
    files = []
    for i in range(n - 1):
        kind = role(slug, i)
        path = os.path.join(work, f"{slug[4:]}-{i:03d}-{kind}.{EXT[slug]}")
        if kind == "big":
            shutil.copy(os.path.join(folder, f"big.{EXT[slug]}"), path)
        elif kind == "linked":
            linked_docx(path, work)
        elif slug == "ely-doc":
            small_docx(path, i)
        elif slug == "ely-table":
            workbook(path, 500)
        elif slug == "ely-show":
            presentation(path, 0)
        else:
            with open(path, "w", encoding="utf-8", newline="\n") as note:
                note.write(small_md(i))
        files.append(path)
    return files, front_file(work, slug)


def front_file(folder, slug):
    """A small document the keys go into."""
    front = os.path.join(folder, f"front.{EXT[slug]}")
    if slug == "ely-mark":
        with open(front, "w", encoding="utf-8", newline="\n") as note:
            note.write("# Typing\n\nA paragraph.\n")
    elif slug == "ely-doc":
        small_docx(front, 0)
    elif slug == "ely-table":
        workbook(front, 500, typing=True)
    else:
        presentation(front, 0)
    return front


def text_of(path):
    """The words a document shows, runs joined; Markdown as is."""
    if path.endswith(".md"):
        with open(path, encoding="utf-8") as note:
            return note.read()
    with zipfile.ZipFile(path) as archive:
        if path.endswith(".docx"):
            parts, tag = ["word/document.xml"], W + "t"
        else:
            parts, tag = sorted(name for name in archive.namelist() if re.fullmatch(r"ppt/slides/slide\d+\.xml", name)), A + "t"
        return "\n".join("".join(node.text or "" for node in ET.fromstring(archive.read(part)).iter(tag)) for part in parts)


def tokens(path):
    return [int(found) for found in re.findall(TOKEN + r"(\d+)", text_of(path))]


def sheet_part(archive, sheet):
    """The worksheet part named `sheet` in the workbook."""
    book = ET.fromstring(archive.read("xl/workbook.xml"))
    rid = next((s.get(R + "id") for s in book.iter(S + "sheet") if s.get("name") == sheet), None)
    if rid is None:
        raise RuntimeError(f"no sheet {sheet}")
    rels = ET.fromstring(archive.read("xl/_rels/workbook.xml.rels"))
    target = next(rel.get("Target") for rel in rels.iter(P + "Relationship") if rel.get("Id") == rid)
    return target.lstrip("/") if target.startswith("/") else "xl/" + target


def found_cell(archive, sheet, ref):
    return next((c for c in ET.fromstring(archive.read(sheet_part(archive, sheet))).iter(S + "c") if c.get("r") == ref), None)


def cell(path, sheet, ref):
    """A cell's saved value as text, or None."""
    with zipfile.ZipFile(path) as archive:
        found = found_cell(archive, sheet, ref)
        if found is None or found.find(S + "v") is None:
            inline = None if found is None else found.find(f"{S}is/{S}t")
            return None if inline is None else inline.text
        value = found.find(S + "v").text
        if found.get("t") == "s":
            shared = ET.fromstring(archive.read("xl/sharedStrings.xml"))
            return "".join(t.text or "" for t in list(shared.iter(S + "si"))[int(value)].iter(S + "t"))
        return value


def formula(path, sheet, ref):
    with zipfile.ZipFile(path) as archive:
        found = found_cell(archive, sheet, ref)
        kept = None if found is None else found.find(S + "f")
        return None if kept is None else kept.text


def external(archive, name):
    """Targets of `name`'s relationships that leave the package."""
    rels = os.path.dirname(name) + "/_rels/" + os.path.basename(name) + ".rels"
    if rels not in archive.namelist():
        return []
    return [rel.get("Target") for rel in ET.fromstring(archive.read(rels)).iter(P + "Relationship")
            if rel.get("TargetMode") == "External"]


def word_links(path, included=INCLUDED_NOW):
    """What a saved DOCX still links, and whether INCLUDETEXT updated."""
    with zipfile.ZipFile(path) as archive:
        xml = archive.read("word/document.xml").decode("utf-8")
        targets = external(archive, "word/document.xml")
    return {"INCLUDETEXT field kept": "INCLUDETEXT" in xml and "include.docx" in xml,
            "INCLUDETEXT shows the source's text": included in text_of(path),
            "linked picture kept": any(t.endswith("picture.png") for t in targets),
            "OLE link kept": 'Type="Link"' in xml and any(t.endswith("source.xlsx") for t in targets)}


def book_links(path):
    """The external book, its formula and updated value."""
    with zipfile.ZipFile(path) as archive:
        books = [name for name in archive.namelist() if re.fullmatch(r"xl/externalLinks/externalLink\d+\.xml", name)]
        targets = [target for name in books for target in external(archive, name)]
    used = formula(path, "Links", "A1") or ""
    value = cell(path, "Links", "A1")
    return {"external book kept": any(t.endswith("source.xlsx") for t in targets),
            "Links!A1 formula kept": bool(re.search(r"\[\d+\]Rates!\$B\$1", used)),
            "Links!A1 shows the source's value": richfiles.number(value) is not None and abs(richfiles.number(value) - RATES_NOW[0]) < 1e-9}


def word_updates(path, folder):
    """Saved DOCX after its sources changed: links, shown text."""
    with zipfile.ZipFile(path) as archive:
        targets = external(archive, "word/document.xml")
    links = word_links(path, "Included changed")
    picture = "file:///" + os.path.abspath(os.path.join(folder, "picture.png")).replace("\\", "/")
    # What the picture and OLE preview show needs a screen.
    return {"INCLUDETEXT saved with the changed text": links["INCLUDETEXT shows the source's text"],
            "the linked picture keeps its link": picture in targets,
            "the OLE link stays": links["OLE link kept"]}
