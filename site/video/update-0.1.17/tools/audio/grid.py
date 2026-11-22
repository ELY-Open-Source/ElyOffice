"""The grid: tempo, chords per bar, sections, the mix bus."""
import numpy as np

from dsp import N, SR, ns, pan, rng

SEED = 1017
TAIL = 3 * SR
BEAT = 0.5
BAR = 2.0
BARS_TOTAL = 60

PC = {"C": 0, "C#": 1, "Db": 1, "D": 2, "Eb": 3, "E": 4, "F": 5, "F#": 6, "G": 7, "Ab": 8, "A": 9, "Bb": 10,
      "B": 11}
QUALITY = {"maj7": (0, 4, 7, 11), "maj9": (0, 4, 7, 11, 14), "m7": (0, 3, 7, 10), "m9": (0, 3, 7, 10, 14),
           "m11": (0, 3, 7, 10, 14, 17), "7": (0, 4, 7, 10), "9sus": (0, 5, 7, 10, 14), "6/9": (0, 4, 7, 9, 14),
           "add9": (0, 4, 7, 14), "sus2": (0, 2, 7), "unison": (0,)}

# one chord per bar; "X|Y" changes at beat 3
CHART = ["D:unison", "D:unison", "D:unison", "C:add9/E",            # c01  0-8
         "F:maj9", "A:m7", "Bb:maj9", "G:m9", "C:9sus", "F:6/9",       # c02  8-20
         "D:m9", "Bb:maj9", "G:m9", "F:maj9", "D:m9",                 # c03 20-30
         "Bb:maj9", "F:maj9", "D:m11", "Bb:maj9|C:9sus",               # c04 30-38
         "F:maj9", "A:m7", "Bb:maj9|G:m9", "C:9sus|C:7",               # c05 38-46
         "F:maj9", "A:m7", "Bb:maj9", "G:m9", "C:9sus", "A:m7|D:7",    # c06 46-58
         "G:maj9", "B:m7", "C:maj9", "A:m9", "D:9sus",                 # c07 58-68
         "G:maj9", "E:m9", "C:maj9", "A:m9", "D:9sus|D:7",             # c08 68-78
         "E:m9", "C:maj9", "G:add9/B", "A:m9", "D:9sus|D:7",           # c09 78-88
         "G:maj9", "B:m7", "C:maj9", "A:m9", "D:9sus",                 # c10 88-98
         "E:m9", "C:maj9", "G:maj9/B", "A:m9|D:9sus",                  # c11 98-106
         "C:maj9", "A:m9", "D:9sus|D:7", "G:maj9", "C:maj9|D:9sus",    # c12 106-116
         "G:6/9", "G:6/9"]                                             # held to the lift
assert len(CHART) == BARS_TOTAL

# texture by first bar of each stretch
SECTIONS = [(0, "touch"), (4, "theme"), (10, "idle"), (14, "one"), (15, "launch"), (17, "still"), (18, "theme"),
            (23, "full"), (29, "guides"), (33, "free"), (34, "full"), (39, "break"), (43, "build"),
            (44, "lift"), (49, "full"), (53, "pull"), (56, "tutti"), (58, "hold")]

# storyboard hits; strong ones get the band
HITS = (6.0, 8.0, 12.0, 18.0, 20.0, 26.0, 28.0, 30.0, 32.0, 34.0, 36.0, 38.0, 40.0, 42.0, 43.0, 44.5, 46.0,
        48.0, 51.0, 54.0, 56.0, 58.0, 60.0, 61.0, 62.0, 64.0, 66.0, 68.0, 70.0, 72.0, 74.0, 75.5, 77.0, 78.0,
        80.0, 82.0, 84.0, 85.5, 87.0, 88.0, 90.0, 91.5, 93.0, 94.5, 96.5, 98.0, 100.0, 101.5, 103.0, 104.5,
        105.5, 106.0, 112.0, 114.0, 118.5)
STRONG = (8.0, 12.0, 18.0, 20.0, 26.0, 32.0, 36.0, 38.0, 46.0, 48.0, 56.0, 58.0, 68.0, 88.0, 98.0, 100.0, 112.0)
CHAPTERS = (0.0, 8.0, 20.0, 30.0, 38.0, 46.0, 58.0, 68.0, 78.0, 88.0, 98.0, 106.0, 120.0)
LIFT = 118.5

GAINS = {"piano": 0.3, "keys": 0.75, "harp": 0.17, "pizz": 0.33, "bowed": 0.29, "cello": 0.3, "marimba": 0.19,
         "sub": 0.18, "kick": 0.32, "brush": 0.42, "fx": 0.4, "glint": 0.2}


def _parse(name):
    head, _, bass = name.rpartition("/")
    if bass not in PC:
        head, bass = name, ""
    root, q = head.split(":")
    return PC[root], QUALITY[q], PC[bass] if bass else PC[root]


def chord(t):
    """(root pc, intervals, bass pc) sounding at film time t."""
    bar = int(np.clip(t // BAR, 0, BARS_TOTAL - 1))
    parts = CHART[bar].split("|")
    late = len(parts) > 1 and t - BAR * bar >= 1.0 - 1e-6
    return _parse(parts[1] if late else parts[0])


def pcs(t):
    r, q, b = chord(t)
    return sorted({(r + i) % 12 for i in q} | {b})


def bass_note(t, low=29):
    """Bass pitch class placed in F1..E2."""
    _, _, b = chord(t)
    return low + (b - low) % 12


def chord_tones(t, lo, hi):
    keep = set(pcs(t))
    return [m for m in range(lo, hi + 1) if m % 12 in keep]


def voicing(t, top=72, spread=4):
    """Close upper voicing under top: third, seventh, colour."""
    r, q, _ = chord(t)
    colour = [i for i in q if i not in (0, 7)] or [0]
    notes = sorted({top - (top - (r + i)) % 12 for i in colour})
    while len(notes) < spread:
        notes.append(notes[0] - 12 + (7 if len(notes) % 2 else 0))
        notes = sorted(set(notes))
    return notes[-spread:]


def section(t):
    bar = int(t / BAR + 1e-9)
    name = SECTIONS[0][1]
    for b, s in SECTIONS:
        if b > bar:
            break
        name = s
    return name


def at(bar, beat):
    return bar * BAR + beat * BEAT


def human(t, ms=5.0):
    return t + rng().normal(0, ms / 1000)


def vary(v, amount=0.1):
    return float(np.clip(v * (1 + rng().uniform(-amount, amount)), 0.02, 1.0))


def blank():
    return np.zeros((N + TAIL, 2))


class Mix:
    def __init__(self):
        self.stems, self.kicks = {}, []
        self.sends = {k: blank() for k in ("room", "hall", "plate")}

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
