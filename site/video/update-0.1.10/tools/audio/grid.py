"""The score's grid: tempo, chords, sections and the mix bus."""
import numpy as np

import instruments as ins
from dsp import N, ns, pan, rng, SR

TAIL = SR
STEP = 0.125
SWING = 0.011
TARGET_LUFS = -15.0
CEILING_DB = -2.0
SEED = 1010

# root (bass midi), rootless voicing (G#3..A#4)
VOICINGS = {
    "Amaj9": (33, (56, 59, 61, 64)), "B13": (35, (57, 61, 63, 68)), "B9sus": (35, (57, 61, 64, 66)),
    "B7b9": (35, (57, 60, 63, 66)), "G#m11": (32, (59, 61, 63, 66)), "C#m9": (37, (59, 63, 64, 68)),
    "F#m9": (30, (57, 61, 64, 68)), "Emaj9": (40, (56, 59, 63, 66)), "C#7sus": (37, (59, 63, 66, 68)),
    "C#9": (37, (59, 63, 65, 68)), "Bmaj9": (35, (58, 61, 63, 66)), "C#13": (37, (59, 63, 65, 70)),
    "A#m11": (34, (61, 63, 65, 68)), "D#m9": (39, (61, 65, 66, 70)), "F#maj9": (30, (58, 61, 65, 68)),
}
CHANGES = [
    (0, "Amaj9"), (4, "B9sus"), (5, "Emaj9"), (6, "C#m9"), (8, "Amaj9"), (10, "B13"), (11, "B9sus"),
    (12, "Amaj9"), (14, "B13"), (16, "G#m11"), (18, "C#m9"), (20, "Amaj9"), (22, "B13"), (24, "C#m9"),
    (26, "F#m9"), (28, "G#m11"), (30, "Amaj9"), (32, "B13"), (34, "F#m9"), (36, "G#m11"), (38, "Amaj9"),
    (40, "B9sus"), (41, "B13"), (42, "Amaj9"), (44, "B9sus"), (46, "B7b9"), (47, "Emaj9"), (48, "C#m9"),
    (50, "Amaj9"), (52, "B13"), (54, "G#m11"), (56, "C#m9"), (58, "F#m9"), (59, "C#7sus"), (59.5, "C#9"),
    (60, "Bmaj9"), (62, "C#13"), (64, "A#m11"), (66, "D#m9"), (68, "Bmaj9"), (70, "C#13"), (72, "A#m11"),
    (74, "D#m9"), (76, "Bmaj9"), (77, "C#7sus"), (77.5, "C#9"), (78, "F#maj9"), (80, "Bmaj9"),
    (82, "C#13"), (84, "A#m11"), (86, "Bmaj9"), (87, "C#7sus"), (88, "F#maj9"),
]
SECTIONS = [(0, "pre"), (1, "intro"), (4, "rise"), (5, "groove"), (11.5, "gap"), (12, "A"), (26, "B1"),
            (30, "B2"), (34, "B3"), (42, "back"), (44, "build"), (47, "half"), (50, "C"), (58, "Cbuild"),
            (60, "peak"), (76, "peakbuild"), (78, "unison"), (80, "outro"), (88, "end")]
# kick-keyed ducking depth per section
PUMP = {"groove": 0.15, "A": 0.5, "B1": 0.25, "B2": 0.35, "B3": 0.45, "back": 0.45, "build": 0.45,
        "half": 0.35, "C": 0.5, "Cbuild": 0.5, "peak": 0.55, "peakbuild": 0.55, "unison": 0.6, "outro": 0.2}
GAINS = {"kick": 0.55, "clap": 0.75, "snap": 0.45, "hats": 1.7, "perc": 0.8, "toms": 0.42, "bass": 0.27,
         "pad": 0.22, "stab": 0.55, "ep": 0.3, "keys": 0.36, "lead": 1.05, "glass": 0.26, "whistle": 0.16,
         "fx": 0.3, "post": 0.3}


def chord_at(t):
    name = CHANGES[0][1]
    for s, c in CHANGES:
        if s > t + 1e-6:
            break
        name = c
    return name


def root(t):
    return VOICINGS[chord_at(t)][0]


def voicing(t):
    return VOICINGS[chord_at(t)][1]


def section(t):
    name = SECTIONS[0][1]
    for s, c in SECTIONS:
        if s > t + 1e-6:
            break
        name = c
    return name


def at(bar, step, swing=True):
    return bar * 2.0 + step * STEP + (SWING if swing and step % 2 else 0.0)


def human(t, ms=3.0):
    return t + rng().normal(0, ms / 1000)


def vary(v, amount=0.07):
    return v * (1 + rng().uniform(-amount, amount))


def blank():
    return np.zeros((N + TAIL, 2))


class Mix:
    def __init__(self):
        self.stems, self.kicks, self.booms = {}, [], []
        self.sends = {k: blank() for k in ("plate", "hall", "delay")}

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

    def kick(self, t, vel, punch=1.0):
        depth = PUMP.get(section(t), 0.0)
        self.kicks.append((t, depth * min(1.0, vel + 0.1)))
        self.add("kick", ins.kick(vel, punch), t)
