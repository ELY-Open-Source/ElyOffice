"""Foley library: variants per sound, levelled, with hit points."""
import json

import numpy as np

import fire
import paper
from dsp import _kweighted_blocks, butter, db, edges, lufs, reseed, SR, write_wav

SEED = 2016
REF_LUFS = -16.0      # loudest 100 ms of a sound at gain 1
PEAK_DB = -1.0
BED_LUFS = {"soft": -36.0, "mid": -31.0, "roar": -26.0}


def _vary(fn, *values):
    return [lambda v=v: fn(v) for v in values]


def _same(fn, k):
    return [fn] * k


# name: (variants, anchor, trim dB); anchor is seconds or "peak"
LIBRARY = {
    "match-strike": (_same(fire.match_strike, 3), 0.16, 2),
    "scrape": (_vary(fire.scrape, 0.1, 0.14, 0.18), 0.0, -3),
    "ignite": (_vary(fire.ignite, 0.6, 1.0, 1.4), "peak", 0),
    "roar": (_same(fire.roar, 2), "peak", 1),
    "catch": (_same(fire.catch, 6), 0.03, -3),
    "sizzle": (_vary(fire.sizzle, 0.9, 1.3, 1.8), 0.0, -4),
    "snuff": (_same(fire.snuff, 2), 0.0, -2),
    "ember": (_same(fire.ember, 3), 0.0, -8),
    "spark": (_same(fire.spark, 6), 0.0, -7),
    "whoosh": (_vary(fire.whoosh, 0.3, 0.4, 0.5, 0.6, 0.75), "peak", -4),
    "rustle": (_vary(paper.rustle, 0.35, 0.5, 0.7, 0.9, 1.2), 0.0, -7),
    "pencil": (_same(paper.pencil, 8), 0.0, -8),
    "charcoal": (_same(paper.charcoal, 8), 0.0, -8),
    "scribble": (_same(paper.scribble, 4), 0.0, -7),
    "tear": (_vary(paper.tear, 0.6, 0.9, 1.3), 0.0, -2),
    "fold": (_same(paper.fold, 3), 0.18, -5),
    "fray": (_same(paper.fray, 3), 0.0, -8),
    "slide": (_vary(paper.slide, 0.35, 0.5, 0.7, 1.0), 0.0, -8),
    "thud": (_vary(paper.thud, 0.6, 0.8, 1.0, 1.2, 1.4), 0.0, 0),
    "cardboard": (_same(paper.cardboard, 4), 0.0, -2),
    "splash": (_same(paper.splash, 4), 0.0, 0),
    "snap": (_same(paper.snap, 3), 0.0, -3),
    "flick": (_same(paper.flick, 4), 0.0, -4),
    "stretch": (_same(paper.stretch, 3), 0.0, -6),
    "squash": (_same(paper.squash, 3), 0.0, -5),
    "pop": (_same(paper.pop, 6), 0.0, -5),
}
BEDS = ("soft", "mid", "roar")


def peak_loudness(x):
    """Loudest 100 ms, K-weighted."""
    x = np.concatenate([x, np.zeros((max(0, int(0.1 * SR) - len(x) + 1), 2))])
    _, z = _kweighted_blocks(x, 0.1, 0.01)
    return -0.691 + 10 * np.log10(z.max() + 1e-20)


def anchor_of(x, anchor):
    if anchor != "peak":
        return float(anchor)
    e = np.convolve(np.abs(x).max(1), np.ones(int(0.02 * SR)) / (0.02 * SR), "same")
    return round(float(e.argmax()) / SR, 4)


def level(x, trim):
    x = edges(butter(x, "high", 35), 0.0005, 0.01)
    x = x * 10 ** ((REF_LUFS + trim - peak_loudness(x)) / 20)
    over = db(np.abs(x).max()) - PEAK_DB
    return x * 10 ** (-max(0.0, over) / 20)


def build_sfx(out_dir):
    reseed(SEED)
    index = {}
    for name, (makers, anchor, trim) in LIBRARY.items():
        anchors = []
        for k, make in enumerate(makers, 1):
            x = level(make(), trim)
            anchors.append(anchor_of(x, anchor))
            write_wav(out_dir / f"{name}-{k}.wav", x)
        index[name] = {"variants": len(makers), "anchors": anchors}
    for kind in BEDS:
        x = fire.crackle_bed(kind)
        x *= 10 ** ((BED_LUFS[kind] - lufs(x)) / 20)
        write_wav(out_dir / f"crackle-{kind}.wav", x)
    index["crackle"] = {"beds": [f"crackle-{k}" for k in BEDS], "loop": True}
    (out_dir / "index.json").write_text(json.dumps(index, indent=1) + "\n")
    return index
