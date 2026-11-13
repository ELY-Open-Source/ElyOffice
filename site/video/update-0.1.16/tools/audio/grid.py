"""The grid: tempo, swing, chords, sections, the mix bus."""
import numpy as np

from dsp import N, SR, ns, pan, rng

SEED = 1016
TAIL = 2 * SR
EIGHTH = 0.25
SWING = 0.035
TARGET_LUFS = -15.0
CEILING_DB = -1.5

PC = {"C": 0, "C#": 1, "D": 2, "D#": 3, "E": 4, "F": 5, "F#": 6, "G": 7, "G#": 8, "A": 9, "A#": 10, "Bb": 10, "B": 11}
QUALITY = {"maj7": (0, 4, 7, 11), "maj9": (0, 4, 7, 11, 14), "m7": (0, 3, 7, 10), "m9": (0, 3, 7, 10, 14),
           "7": (0, 4, 7, 10), "7sus": (0, 5, 7, 10), "9sus": (0, 5, 7, 10, 14), "m6": (0, 3, 7, 9),
           "6/9": (0, 4, 7, 9, 14)}

# a chord per bar; "X|Y" splits at beat 3
BARS = ["D:maj9", "D:maj9", "G:maj9", "A:9sus|A:7",                       # s1
        "D:maj7", "B:m7", "G:maj7", "A:7sus|A:7", "D:maj7", "A:9sus",      # s2
        "G:maj7", "F#:m7", "E:m9", "A:7", "D:maj7", "E:m7|A:7",            # s3
        "D:maj7", "B:m7", "G:maj7", "A:7sus|A:7", "D:maj9",                # s4
        "B:m9", "G:maj9", "A:9sus|A:7", "D:maj7", "B:m7", "G:maj7",        # s5
        "A:7sus|A:7", "D:maj7", "B:m7", "G:m6", "F#:m7|B:7",               # s6
        "E:maj7", "C#:m7", "A:maj7", "B:7sus|B:7", "E:maj7",               # s7
        "C#:m7", "A:maj9", "F#:m7|B:7", "E:maj7",                          # s8
        "C#:m7|F#:7", "A:maj7|B:7sus", "E:6/9", "E:6/9"]                   # s9
assert len(BARS) == 45

# texture per bar range, start bar inclusive
SECTIONS = [(0, "black"), (1, "spark"), (2, "intro"), (4, "A"), (10, "B"), (16, "A2"), (21, "break"),
            (23, "build"), (24, "A3"), (27, "C"), (30, "tear"), (31, "C"), (32, "lift"), (41, "montage"),
            (43, "tutti")]
GAINS = {"kick": 0.5, "brush": 0.55, "ride": 0.5, "bass": 0.5, "nylon": 0.36, "kalimba": 0.34, "whistle": 0.2,
         "glock": 0.09, "reed": 0.2, "perc": 0.5, "fx": 0.35,
         "post": 0.3}


def chord_name(t):
    bar = min(44, int(t // 2))
    parts = BARS[bar].split("|")
    return parts[1] if len(parts) > 1 and t - 2 * bar >= 1.0 - 1e-6 else parts[0]


def tones(t):
    root, q = chord_name(t).split(":")
    return PC[root], QUALITY[q]


def bass_root(t):
    r, _ = tones(t)
    return 28 + (r - 4) % 12      # E1..D#2


def guitar(t):
    """Open-ish voicing: thumb root, fingers on 3rd, 7th and colour."""
    r, q = tones(t)
    low = 40 + (r - 4) % 12
    upper = sorted(55 + (r + i - 7) % 12 for i in q[1:])
    return [low] + upper


def scale_tones(t, lo, hi):
    r, q = tones(t)
    pcs = {(r + i) % 12 for i in q}
    return [m for m in range(lo, hi + 1) if m % 12 in pcs]


def section(t):
    bar = int(t / 2 + 1e-9)
    name = SECTIONS[0][1]
    for b, s in SECTIONS:
        if b > bar:
            break
        name = s
    return name


def at(bar, eighth, swing=True):
    """Global seconds of an eighth in a bar; offbeats swung."""
    return bar * 2.0 + eighth * EIGHTH + (SWING if swing and int(eighth) % 2 else 0.0)


def human(t, ms=4.0):
    return t + rng().normal(0, ms / 1000)


def vary(v, amount=0.08):
    return v * (1 + rng().uniform(-amount, amount))


def blank():
    return np.zeros((N + TAIL, 2))


class Mix:
    def __init__(self):
        self.stems, self.kicks = {}, []
        self.sends = {k: blank() for k in ("room", "hall", "delay")}

    def add(self, stem, sig, t, gain=1.0, p=0.0, **sends):
        sig = pan(sig, p) if sig.ndim == 1 else sig
        i = ns(t)
        if i < 0:
            sig, i = sig[-i:], 0
        bus = self.stems.setdefault(stem, blank())
        n = min(len(sig), len(bus) - i)
        if n <= 0:
            return
        part = sig[:n] * gain * GAINS[stem]
        bus[i:i + n] += part
        for name, amount in sends.items():
            self.sends[name][i:i + n] += part * amount
