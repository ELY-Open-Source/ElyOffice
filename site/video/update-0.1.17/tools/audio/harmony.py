"""Accompaniment: piano patterns, string pads, sub, harp and pizzicato."""
import numpy as np

import piano
import strings as st
from grid import (BAR, BARS_TOTAL, BEAT, CHAPTERS, LIFT, at, bass_note, chord, chord_tones, human, section,
                  vary)

FLOW = ("theme", "full", "lift")
INTENSITY = ((0, 0.3), (8, 0.62), (18, 0.72), (20, 0.45), (30, 0.5), (34, 0.32), (36, 0.62), (46, 0.72),
             (58, 0.78), (68, 0.82), (78, 0.42), (86, 0.66), (88, 0.88), (104, 0.9), (106, 0.55),
             (111.9, 0.95), (112, 1.0), (116, 0.78), (120, 0.6))


def level(t):
    ts, vs = zip(*INTENSITY)
    return float(np.interp(t, ts, vs))


def changes(bar):
    """Chord-change times inside a bar, with the bar's end."""
    t0 = bar * BAR
    out = [t0]
    if chord(t0 + 1.0) != chord(t0):
        out.append(t0 + 1.0)
    return out + [t0 + BAR]


def pedal(t):
    """Seconds until the pedal lifts at the next chord change."""
    bar = int(t // BAR)
    for c in changes(bar)[1:]:
        if c > t + 1e-6:
            return c - t
    return BAR


def _arp(t):
    lo = bass_note(t) + 12
    tones = chord_tones(t, lo, lo + 26)
    return [tones[min(i, len(tones) - 1)] for i in (0, 2, 4, 3, 5, 4, 2, 3)]


def piano_part(mix):
    for bar in range(4, BARS_TOTAL):
        name = section(bar * BAR)
        t0 = bar * BAR
        g = level(t0)
        if name in FLOW or name == "build":
            accents = (0.62, 0.4, 0.48, 0.42, 0.55, 0.4, 0.46, 0.4)
            for i in range(8):
                t = t0 + i * BEAT / 2
                m = _arp(t)[i]
                v = vary(accents[i] * (0.5 + 0.55 * g) * (1 + (0.5 * i / 8 if name == "build" else 0)), 0.07)
                mix.add("piano", piano.note(m, v, pedal(t) + 0.04), human(t, 5), 1.0, -0.25 + 0.06 * (i % 4),
                        plate=0.25, hall=0.12)
        elif name in ("idle", "launch"):
            r = bass_note(t0) + 12
            mix.add("piano", piano.note(r, vary(0.4 * g + 0.15), 2.0), human(t0, 4), 1.0, -0.3, plate=0.3)
            if name == "idle":
                mix.add("piano", piano.chord(_voice(t0 + 0.75, 64), 0.26, 1.2), human(t0 + 0.75), 1.0, 0.1,
                        plate=0.35, hall=0.25)
        elif name == "still":
            mix.add("piano", piano.chord([38, 45] + _voice(t0, 69), 0.3, 2.3, 0.02), t0, 1.0, 0.0, plate=0.4,
                    hall=0.4)
        elif name in ("guides", "free"):
            guides(mix, bar, name)
        elif name == "break":
            for i, k in enumerate((0, 2, 4, 2)):
                t = t0 + i * BEAT
                tones = chord_tones(t, bass_note(t) + 12, bass_note(t) + 30)
                mix.add("piano", piano.note(tones[min(k, len(tones) - 1)], vary(0.3 if i else 0.4, 0.06),
                                            pedal(t) + 0.3), human(t, 9), 1.0, -0.2 + 0.1 * i, plate=0.45, hall=0.35)
        elif name in ("pull", "tutti"):
            beats = range(4) if name == "pull" else (0, 2)
            for b in beats:
                t = t0 + b * BEAT
                vel = (0.32 + 0.12 * (bar - 53) + 0.03 * b) if name == "pull" else 0.72
                mix.add("piano", piano.chord([bass_note(t) + 12] + _voice(t, 67), vary(vel, 0.05), pedal(t) + 0.05),
                        human(t, 4), 1.0, 0.0, plate=0.3, hall=0.3)
            if name == "tutti":
                mix.add("piano", piano.note(bass_note(t0), 0.7, 1.9), t0, 1.0, -0.2, plate=0.2, hall=0.3)
    final = [31, 38, 47, 52, 57, 62, 71]
    mix.add("piano", piano.chord(final, 0.5, LIFT - 116.0, 0.03), 116.0, 1.0, 0.0, plate=0.4, hall=0.5)


def _voice(t, top):
    """Upper voicing under top; root not doubled low."""
    r, q, _ = chord(t)
    notes = sorted({top - (top - (r + i)) % 12 for i in q[1:]})
    return [m for m in notes if m > top - 12][-4:]


def guides(mix, bar, name):
    """c07: staccato stabs on the offbeats, accents on the snaps."""
    t0 = bar * BAR
    r = bass_note(t0) + 12
    mix.add("piano", piano.note(r, 0.48, 0.3), t0, 1.0, -0.25, plate=0.2)
    if name == "free":
        tones = chord_tones(t0, 62, 93)[::-1]
        when = np.cumsum([0.0] + [0.055 + 0.03 * np.sin(i / 3) for i in range(len(tones) - 1)])
        for m, dt in zip(tones, when):
            mix.add("piano", piano.note(m, vary(0.34, 0.1), 1.6), t0 + 0.25 + dt, 1.0, 0.4 - 0.05 * (m % 8),
                    plate=0.5, hall=0.4)
        return
    for b in (0.5, 1.5, 2.5, 3.5):
        t = t0 + b * BEAT
        mix.add("piano", piano.chord(_voice(t, 72), vary(0.3, 0.05), 0.14, 0.004), human(t, 2), 1.0, 0.15,
                plate=0.3)


def _lead(prev, t, lo=53, hi=70):
    """Voicing with the least motion from prev."""
    best, cost = None, 1e9
    for top in range(lo + 7, hi + 1):
        v = _voice(t, top)[-3:]
        c = sum(min(abs(a - b) for b in prev) for a in v) if prev else abs(top - 64)
        if c < cost:
            best, cost = v, c
    return best


def segments():
    """(start, end, section) of each held chord."""
    out = []
    for bar in range(2, BARS_TOTAL):
        cs = changes(bar)
        for a, b in zip(cs, cs[1:]):
            out.append((a, b, section(a)))
    return out


PADS = {"theme": 0.28, "idle": 0.42, "one": 0.3, "launch": 0.42, "still": 0.36, "full": 0.3, "guides": 0.22,
        "free": 0.4, "break": 0.4, "build": 0.5, "lift": 0.34, "pull": 0.5, "tutti": 0.62, "hold": 0.55}


def pads(mix):
    prev = None
    for a, b, name in segments():
        if a < 8.0 or name not in PADS:
            continue
        dur = (LIFT - a) if name == "hold" else b - a
        if name == "hold" and a > 116.0:
            continue
        base = PADS[name]
        grow = name in ("build", "pull", "launch") or b in CHAPTERS
        swell = [(0, 0.7), (dur * 0.6, 1.0 if grow else 0.85), (dur + 0.6, 1.15 if grow else 0.7)]
        voice = _lead(prev, a + 0.01)
        prev = voice
        rel = 0.6 if name == "hold" else 0.9
        for k, m in enumerate(voice):
            y = st.bowed(m, dur + 0.05, vary(base, 0.05), "viola", attack=0.5, release=rel, players=4, swell=swell)
            mix.add("bowed", y, a - 0.04, 1.0, -0.3 + 0.3 * k, hall=0.5)
        if name not in ("guides",):
            low = bass_note(a) + 12
            y = st.bowed(low, dur + 0.05, vary(base * 0.9, 0.05), "cello", attack=0.6, release=rel + 0.1, players=3,
                         swell=swell)
            mix.add("cello", y, a - 0.05, 1.0, -0.15, hall=0.4)
        if name in ("pull", "tutti", "hold"):
            top = voice[-1] + 12
            y = st.bowed(top, dur + 0.05, vary(base * 0.8, 0.05), "violin", attack=0.6, release=rel + 0.3, players=6,
                         swell=swell)
            mix.add("bowed", y, a - 0.03, 1.0, 0.2, hall=0.6)


SUB = {"theme": ((0, 1.8),), "idle": ((0, 1.9),), "one": ((0, 1.9),), "launch": ((0, 1.9),),
       "still": ((0, 2.2),), "full": ((0, 0.7), (1.5, 0.35), (2, 0.9)), "lift": ((0, 0.7), (1.5, 0.35), (2, 0.9)),
       "guides": ((0, 0.4), (1.5, 0.3), (2, 0.4), (3.5, 0.3)), "build": ((0, 1.9),),
       "pull": ((0, 1.95),), "tutti": ((0, 1.95),)}


def sub(mix):
    for bar in range(4, BARS_TOTAL):
        t0 = bar * BAR
        name = section(t0)
        for beat, dur in SUB.get(name, ()):
            t = t0 + beat * BEAT
            v = vary(0.55 + 0.4 * level(t), 0.04)
            mix.add("sub", st.sub(bass_note(t), dur, v), human(t, 2), 1.0)
    mix.add("sub", st.sub(bass_note(116.0), LIFT - 116.0, 0.75, release=0.5), 116.0, 1.0)


def harp(mix):
    """A glissando into each chapter; offbeat plucks while idle."""
    for c in CHAPTERS[1:-1]:
        if c in (78.0, 106.0):
            continue
        tones = chord_tones(c, 62, 86)
        n = len(tones)
        for i, m in enumerate(tones):
            u = i / max(1, n - 1)
            t = c - 0.5 * (1 - u) ** 0.7
            mix.add("harp", st.harp(m, vary(0.3 + 0.4 * u, 0.06), 2.4), t, 1.0, -0.5 + u, hall=0.5)
    for bar in range(10, 14):
        for beat in (1.5, 3.5):
            t = at(bar, beat)
            tones = chord_tones(t, 67, 84)
            m = tones[int(beat + bar) % len(tones)]
            mix.add("harp", st.harp(m, vary(0.32, 0.1), 2.0), human(t, 6), 1.0, 0.45, hall=0.5)


def pizz(mix):
    for bar in range(23, 56):
        t0 = bar * BAR
        name = section(t0)
        if name == "guides":
            steps = ((0, 0), (1, 2), (1.5, 3), (2.5, 2), (3, 1), (3.5, 3))
        elif name in ("full", "lift"):
            steps = ((1.5, 2), (3.5, 1))
        else:
            continue
        for beat, k in steps:
            t = t0 + beat * BEAT
            tones = chord_tones(t, bass_note(t) + 12, bass_note(t) + 30)
            mix.add("pizz", st.pizz(tones[min(k, len(tones) - 1)], vary(0.55, 0.1), 0.5), human(t, 3), 1.0,
                    -0.4 + 0.2 * k, room=0.3)
