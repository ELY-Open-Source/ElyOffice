"""Writes alias.ttf: one font family named in two languages."""
import os
import sys

from fontTools import subset
from fontTools.ttLib import TTFont

HERE = os.path.dirname(os.path.abspath(__file__))
KEPT = {0, 13, 14}

# DejaVu Sans cut to ASCII, renamed as its license asks.
options = subset.Options()
options.name_IDs = sorted(KEPT)
options.hinting = False
face = TTFont(sys.argv[1])
cutter = subset.Subsetter(options)
cutter.populate(text="".join(chr(c) for c in range(32, 127)))
cutter.subset(face)
names = face["name"]
names.names = [n for n in names.names if n.nameID in KEPT]
for lang, family in ((0x409, "Ely Alias Sans"), (0x804, "别名黑体")):
    for name_id, value in ((1, family), (2, "Book"), (4, family), (16, family)):
        names.setName(value, name_id, 3, 1, lang)
names.setName("ElyAliasSans", 6, 3, 1, 0x409)
face.save(os.path.join(HERE, "alias.ttf"))
