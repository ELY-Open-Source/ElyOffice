"""Writes video.pptx: a video, its fade-in and click trigger."""
import os
import sys
import zipfile

HEAD = '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>\n'
A = 'xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"'
P = 'xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"'
R = 'xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"'
NS = f"{A} {R} {P}"
REL = "http://schemas.openxmlformats.org/officeDocument/2006/relationships/"
MS = "http://schemas.microsoft.com/office/2007/relationships/"
TYPE = "application/vnd.openxmlformats-officedocument.presentationml."
GROUP = ('<p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr><a:xfrm>'
         '<a:off x="0" y="0"/><a:ext cx="0" cy="0"/><a:chOff x="0" y="0"/><a:chExt cx="0" cy="0"/></a:xfrm></p:grpSpPr>')
PNG = bytes.fromhex(
    "89504e470d0a1a0a0000000d4948445200000001000000010806000000"
    "1f15c4890000000d49444154789c636060606000000005000173a3c46f0000000049454e44ae426082")


def rels(*links):
    body = "".join(f'<Relationship Id="rId{n}" Type="{kind}" Target="{target}"{mode}/>'
                   for n, (kind, target, mode) in enumerate(links, 1))
    return HEAD + f'<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">{body}</Relationships>'


def link(kind, target, base=REL):
    return (base + kind, target, "")


THEME = HEAD + (
    f'<a:theme {A} name="Office"><a:themeElements><a:clrScheme name="Office">'
    + "".join(f'<a:{n}><a:srgbClr val="{v}"/></a:{n}>' for n, v in [
        ("dk1", "000000"), ("lt1", "FFFFFF"), ("dk2", "44546A"), ("lt2", "E7E6E6"), ("accent1", "4472C4"),
        ("accent2", "ED7D31"), ("accent3", "A5A5A5"), ("accent4", "FFC000"), ("accent5", "5B9BD5"),
        ("accent6", "70AD47"), ("hlink", "0563C1"), ("folHlink", "954F72")])
    + '</a:clrScheme><a:fontScheme name="Office"><a:majorFont><a:latin typeface="Calibri"/><a:ea typeface=""/>'
    '<a:cs typeface=""/></a:majorFont><a:minorFont><a:latin typeface="Calibri"/><a:ea typeface=""/><a:cs typeface=""/>'
    '</a:minorFont></a:fontScheme><a:fmtScheme name="Office"><a:fillStyleLst>'
    + '<a:solidFill><a:schemeClr val="phClr"/></a:solidFill>' * 3
    + '</a:fillStyleLst><a:lnStyleLst>' + '<a:ln w="6350"><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:ln>' * 3
    + '</a:lnStyleLst><a:effectStyleLst>' + '<a:effectStyle><a:effectLst/></a:effectStyle>' * 3
    + '</a:effectStyleLst><a:bgFillStyleLst>' + '<a:solidFill><a:schemeClr val="phClr"/></a:solidFill>' * 3
    + "</a:bgFillStyleLst></a:fmtScheme></a:themeElements></a:theme>")

MASTER = HEAD + (
    f'<p:sldMaster {NS}><p:cSld><p:spTree>{GROUP}</p:spTree></p:cSld>'
    '<p:clrMap bg1="lt1" tx1="dk1" bg2="lt2" tx2="dk2" accent1="accent1" accent2="accent2" accent3="accent3" '
    'accent4="accent4" accent5="accent5" accent6="accent6" hlink="hlink" folHlink="folHlink"/>'
    '<p:sldLayoutIdLst><p:sldLayoutId id="2147483649" r:id="rId1"/></p:sldLayoutIdLst></p:sldMaster>')
LAYOUT = HEAD + (f'<p:sldLayout {NS} type="blank"><p:cSld name="Blank"><p:spTree>{GROUP}</p:spTree></p:cSld>'
                 '<p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr></p:sldLayout>')
PRESENTATION = HEAD + (
    f'<p:presentation {NS}><p:sldMasterIdLst><p:sldMasterId id="2147483648" r:id="rId1"/></p:sldMasterIdLst>'
    '<p:sldIdLst><p:sldId id="256" r:id="rId2"/></p:sldIdLst><p:sldSz cx="12192000" cy="6858000"/>'
    '<p:notesSz cx="6858000" cy="9144000"/></p:presentation>')

TRIGGER = '<p:tgtEl><p:spTgt spid="2"/></p:tgtEl>'
TIMING = (
    '<p:timing><p:tnLst><p:par><p:cTn id="1" dur="indefinite" restart="never" nodeType="tmRoot"><p:childTnLst>'
    '<p:seq concurrent="1" nextAc="seek"><p:cTn id="2" dur="indefinite" nodeType="mainSeq"><p:childTnLst>'
    '<p:par><p:cTn id="3" fill="hold"><p:stCondLst><p:cond delay="indefinite"/></p:stCondLst><p:childTnLst>'
    '<p:par><p:cTn id="4" fill="hold"><p:stCondLst><p:cond delay="0"/></p:stCondLst><p:childTnLst>'
    '<p:par><p:cTn id="5" presetID="10" presetClass="entr" presetSubtype="0" fill="hold" nodeType="clickEffect">'
    '<p:stCondLst><p:cond delay="0"/></p:stCondLst><p:childTnLst><p:set><p:cBhvr><p:cTn id="6" dur="1" fill="hold">'
    f'<p:stCondLst><p:cond delay="0"/></p:stCondLst></p:cTn>{TRIGGER}<p:attrNameLst><p:attrName>style.visibility'
    '</p:attrName></p:attrNameLst></p:cBhvr><p:to><p:strVal val="visible"/></p:to></p:set>'
    f'<p:animEffect transition="in" filter="fade"><p:cBhvr><p:cTn id="7" dur="500"/>{TRIGGER}</p:cBhvr>'
    '</p:animEffect></p:childTnLst></p:cTn></p:par></p:childTnLst></p:cTn></p:par></p:childTnLst></p:cTn></p:par>'
    '</p:childTnLst></p:cTn><p:prevCondLst><p:cond evt="onPrev" delay="0"><p:tgtEl><p:sldTgt/></p:tgtEl></p:cond>'
    '</p:prevCondLst><p:nextCondLst><p:cond evt="onNext" delay="0"><p:tgtEl><p:sldTgt/></p:tgtEl></p:cond>'
    '</p:nextCondLst></p:seq><p:seq concurrent="1" nextAc="seek"><p:cTn id="8" restart="whenNotActive" fill="hold" '
    f'evtFilter="cancelBubble" nodeType="interactiveSeq"><p:stCondLst><p:cond evt="onClick" delay="0">{TRIGGER}'
    '</p:cond></p:stCondLst><p:endSync evt="end" delay="0"><p:rtn val="all"/></p:endSync><p:childTnLst>'
    '<p:par><p:cTn id="9" fill="hold"><p:stCondLst><p:cond delay="0"/></p:stCondLst><p:childTnLst>'
    '<p:par><p:cTn id="10" fill="hold"><p:stCondLst><p:cond delay="0"/></p:stCondLst><p:childTnLst>'
    '<p:par><p:cTn id="11" presetID="2" presetClass="mediacall" presetSubtype="0" fill="hold" nodeType="clickEffect">'
    '<p:stCondLst><p:cond delay="0"/></p:stCondLst><p:childTnLst><p:cmd type="call" cmd="togglePause"><p:cBhvr>'
    f'<p:cTn id="12" dur="1" fill="hold"/>{TRIGGER}</p:cBhvr></p:cmd></p:childTnLst></p:cTn></p:par></p:childTnLst>'
    '</p:cTn></p:par></p:childTnLst></p:cTn></p:par></p:childTnLst></p:cTn><p:nextCondLst>'
    f'<p:cond evt="onClick" delay="0">{TRIGGER}</p:cond></p:nextCondLst></p:seq><p:video><p:cMediaNode vol="80000">'
    '<p:cTn id="13" fill="hold" display="0"><p:stCondLst><p:cond delay="indefinite"/></p:stCondLst></p:cTn>'
    f'{TRIGGER}</p:cMediaNode></p:video></p:childTnLst></p:cTn></p:par></p:tnLst></p:timing>')
SLIDE = HEAD + (
    f'<p:sld {NS}><p:cSld><p:spTree>{GROUP}<p:pic><p:nvPicPr><p:cNvPr id="2" name="clip">'
    '<a:hlinkClick r:id="" action="ppaction://media"/></p:cNvPr><p:cNvPicPr><a:picLocks noChangeAspect="1"/>'
    '</p:cNvPicPr><p:nvPr><a:videoFile r:link="rId2"/><p:extLst><p:ext uri="{DAA4B4D4-6D71-4841-9C94-3DE7FCFB9230}">'
    '<p14:media xmlns:p14="http://schemas.microsoft.com/office/powerpoint/2010/main" r:embed="rId3"/></p:ext></p:extLst>'
    '</p:nvPr></p:nvPicPr><p:blipFill><a:blip r:embed="rId4"/><a:stretch><a:fillRect/></a:stretch></p:blipFill>'
    '<p:spPr><a:xfrm><a:off x="1270000" y="1270000"/><a:ext cx="4064000" cy="2286000"/></a:xfrm>'
    f'<a:prstGeom prst="rect"><a:avLst/></a:prstGeom></p:spPr></p:pic></p:spTree></p:cSld>{TIMING}</p:sld>')


def parts(clip):
    over = [("/ppt/presentation.xml", "presentation.main+xml"), ("/ppt/slideMasters/slideMaster1.xml", "slideMaster+xml"),
            ("/ppt/slideLayouts/slideLayout1.xml", "slideLayout+xml"), ("/ppt/slides/slide1.xml", "slide+xml")]
    return {
        "[Content_Types].xml": HEAD + '<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">'
        '<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>'
        '<Default Extension="xml" ContentType="application/xml"/><Default Extension="png" ContentType="image/png"/>'
        '<Default Extension="mp4" ContentType="video/mp4"/>'
        '<Override PartName="/ppt/theme/theme1.xml" ContentType="application/vnd.openxmlformats-officedocument.theme+xml"/>'
        + "".join(f'<Override PartName="{n}" ContentType="{TYPE}{k}"/>' for n, k in over) + "</Types>",
        "_rels/.rels": rels(link("officeDocument", "ppt/presentation.xml")),
        "ppt/presentation.xml": PRESENTATION,
        "ppt/_rels/presentation.xml.rels": rels(link("slideMaster", "slideMasters/slideMaster1.xml"),
                                                 link("slide", "slides/slide1.xml"), link("theme", "theme/theme1.xml")),
        "ppt/theme/theme1.xml": THEME,
        "ppt/slideMasters/slideMaster1.xml": MASTER,
        "ppt/slideMasters/_rels/slideMaster1.xml.rels": rels(link("slideLayout", "../slideLayouts/slideLayout1.xml"),
                                                             link("theme", "../theme/theme1.xml")),
        "ppt/slideLayouts/slideLayout1.xml": LAYOUT,
        "ppt/slideLayouts/_rels/slideLayout1.xml.rels": rels(link("slideMaster", "../slideMasters/slideMaster1.xml")),
        "ppt/slides/slide1.xml": SLIDE,
        "ppt/slides/_rels/slide1.xml.rels": rels(link("slideLayout", "../slideLayouts/slideLayout1.xml"),
                                                 link("video", "../media/media1.mp4"), link("media", "../media/media1.mp4", MS),
                                                 link("image", "../media/image1.png")),
        "ppt/media/media1.mp4": clip,
        "ppt/media/image1.png": PNG,
    }


here = os.path.dirname(os.path.abspath(__file__))
with open(os.path.join(here, "..", "media", "clip.mp4"), "rb") as f:
    clip = f.read()
with zipfile.ZipFile(os.path.join(sys.argv[1], "video.pptx"), "w", zipfile.ZIP_DEFLATED) as out:
    for name, body in parts(clip).items():
        info = zipfile.ZipInfo(name, (2026, 11, 1, 0, 0, 0))
        info.compress_type = zipfile.ZIP_STORED if name.endswith(".mp4") else zipfile.ZIP_DEFLATED
        out.writestr(info, body if isinstance(body, bytes) else body.encode("utf-8"))
