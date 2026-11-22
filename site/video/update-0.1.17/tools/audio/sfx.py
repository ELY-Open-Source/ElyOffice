"""The foley library: variants per sound, levelled, with hit points."""
import json
from functools import partial

import numpy as np

import foley as fx
from dsp import SR, butter, reseed, write_wav

# name: (peak dBFS, makers)
LIBRARY = {
    "click": (-9.0, [partial(fx.click, p) for p in (1.0, 0.9, 1.1, 0.95, 1.05)]),
    "double-click": (-9.0, [partial(fx.double, p) for p in (1.0, 0.92, 1.08)]),
    "tick": (-10.0, [partial(fx.tick, m) for m in (89, 91, 93, 96, 98, 101)]),
    "snap": (-7.0, [partial(fx.snap, f) for f in (1180.0, 1060.0, 1320.0, 990.0, 1250.0)]),
    "slide": (-12.0, [partial(fx.slide, s) for s in (0.35, 0.5, 0.75, 1.0)]),
    "key": (-11.0, [partial(fx.key, p) for p in (1.0, 0.94, 1.07, 0.9, 1.12)] + [partial(fx.key, 1.0, True)]),
    "peel": (-10.0, [partial(fx.peel, s) for s in (0.45, 0.6, 0.85)]),
    "pop": (-9.0, [partial(fx.pop, f) for f in (520.0, 620.0, 740.0, 440.0, 880.0)]),
    "whoosh": (-11.0, [partial(fx.whoosh, s, w) for s, w in ((0.6, 1), (0.8, -1), (1.0, 1), (1.25, -1))]),
    "lock": (-9.0, [partial(fx.lock, p) for p in (1.0, 0.9, 1.1)]),
    "error": (-12.0, [partial(fx.error, n) for n in ((71, 67), (69, 64), (74, 68))]),
    "paper": (-12.0, [partial(fx.paper, k) for k in ("flick", "slide", "crinkle", "turn")]),
}


def build_sfx(out):
    """Writes every variant as <name>-<n>.wav and index.json."""
    reseed(7117)
    index = {}
    for name, (peak, makers) in LIBRARY.items():
        anchors, lengths = [], []
        for i, make in enumerate(makers, 1):
            y, hit = make()
            y = butter(y, "high", 40)
            y *= 10 ** (peak / 20) / (np.abs(y).max() + 1e-12)
            write_wav(out / f"{name}-{i}.wav", y)
            anchors.append(round(hit, 4))
            lengths.append(round(len(y) / SR, 4))
        index[name] = {"variants": len(makers), "anchors": anchors, "lengths": lengths, "peak": peak}
    (out / "index.json").write_text(json.dumps(index, indent=1) + "\n")
    return index
