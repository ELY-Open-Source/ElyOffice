"""Checks a saved PPTX video's effects without LibreOffice."""
import os
import sys
import zipfile
import xml.etree.ElementTree as ET

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import ooxml  # noqa: E402

P = "{http://schemas.openxmlformats.org/presentationml/2006/main}"
SLIDE = "ppt/slides/slide1.xml"


def video(path):
    """A fade-in, then a click trigger that toggles play."""
    with zipfile.ZipFile(path) as archive:
        root = ET.fromstring(archive.read(SLIDE))
    pics = [p for p in root.iter(f"{P}pic") if p.find(f"{P}nvPicPr/{P}nvPr") is not None]
    if not ooxml.check(len(pics) == 1, f"one picture: {len(pics)}"):
        return
    spid = pics[0].find(f"{P}nvPicPr/{P}cNvPr").get("id")

    def targets(node):
        return {t.get("spid") for t in node.iter(f"{P}spTgt")}

    seqs = {s.find(f"{P}cTn").get("nodeType"): s for s in root.iter(f"{P}seq")}
    main, trigger = seqs.get("mainSeq"), seqs.get("interactiveSeq")
    if not ooxml.check(main is not None and trigger is not None, f"both sequences: {sorted(seqs)}"):
        return
    fades = [n for n in main.iter(f"{P}cTn") if n.get("presetClass") == "entr" and n.get("presetID") == "10"]
    ooxml.check(len(fades) == 1, f"one fade-in: {len(fades)}")
    effects = [(e.get("transition"), e.get("filter"), e.find(f"{P}cBhvr/{P}cTn").get("dur"), targets(e))
               for f in fades for e in f.iter(f"{P}animEffect")]
    ooxml.check(effects == [("in", "fade", "500", {spid})], f"the video fades in over 500 ms: {effects}")
    ooxml.check(trigger.find(f"{P}prevCondLst") is None, "the trigger has no onPrev")
    ends = [(c.get("evt"), targets(c)) for c in trigger.findall(f"{P}nextCondLst/{P}cond")]
    ooxml.check(ends == [("onClick", {spid})], f"the trigger ends on its click: {ends}")
    calls = [n for n in trigger.iter(f"{P}cTn") if n.get("presetClass") == "mediacall"]
    named = [(n.get("presetID"), [c.get("cmd") for c in n.iter(f"{P}cmd")]) for n in calls]
    ooxml.check(named == [("2", ["togglePause"])], f"the trigger toggles pause: {named}")


CASES = {"video": video}
MUTATIONS = {
    "video": [
        (SLIDE, r'(nodeType="interactiveSeq">.*?)(<p:nextCondLst>)',
         r'\1<p:prevCondLst><p:cond evt="onPrev"><p:tgtEl><p:sldTgt/></p:tgtEl></p:cond></p:prevCondLst>\2'),
        (SLIDE, r'(nodeType="interactiveSeq">.*?<p:nextCondLst><p:cond )[^>]*>', r'\1evt="onNext" delay="0">'),
        (SLIDE, r'presetID="2" (presetClass="mediacall")', r"\1"),
        (SLIDE, r'presetID="10" presetClass="entr"', 'presetID="1" presetClass="entr"'),
        (SLIDE, r'<p:animEffect .*?</p:animEffect>', ""),
        (SLIDE, r'(<p:animEffect [^>]*><p:cBhvr[^>]*><p:cTn id="\d+") dur="500"', r'\1 dur="1"'),
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
