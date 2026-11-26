"""Writes gap.ttf: alias.ttf with a line gap of one em."""
import os

from fontTools.ttLib import TTFont

HERE = os.path.dirname(os.path.abspath(__file__))
USE_TYPO_METRICS = 1 << 7

face = TTFont(os.path.join(HERE, "..", "office", "alias.ttf"))
em = face["head"].unitsPerEm
face["hhea"].lineGap = em
face["OS/2"].sTypoLineGap = em
face["OS/2"].fsSelection &= ~USE_TYPO_METRICS
names = face["name"]
names.names = [n for n in names.names if n.langID == 0x409]
for name_id, value in ((1, "Ely Gap Sans"), (4, "Ely Gap Sans"), (16, "Ely Gap Sans")):
    names.setName(value, name_id, 3, 1, 0x409)
names.setName("ElyGapSans", 6, 3, 1, 0x409)
face.save(os.path.join(HERE, "gap.ttf"))
