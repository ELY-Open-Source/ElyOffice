"""Marimba ostinati, soft kick, brushes, band hits and glints."""
import numpy as np

import kit
import piano
from grid import BAR, BARS_TOTAL, BEAT, CHAPTERS, HITS, STRONG, bass_note, chord_tones, human, section, vary
from harmony import level

EIGHTHS = ("theme", "full", "lift", "build")
SIXTEENTHS = ("launch", "guides", "tutti")
ORDER = (0, 2, 4, 1, 3, 5, 2, 4, 1, 3, 5, 6, 4, 2, 3, 1)


def marimba(mix):
    for bar in range(4, BARS_TOTAL):
        t0 = bar * BAR
        name = section(t0)
        if name in EIGHTHS:
            step, n = BEAT / 2, 8
        elif name in SIXTEENTHS:
            step, n = BEAT / 4, 16
        elif name == "idle":
            step, n = BEAT, 4
        else:
            continue
        g = level(t0)
        for i in range(n):
            t = t0 + i * step
            tones = chord_tones(t, 67, 91)
            k = ORDER[(i + bar) % 16] % len(tones)
            if name == "launch":
                k = int((len(tones) - 1) * (0.5 - 0.5 * np.cos(np.pi * i / 8))) % len(tones)
            accent = 1.0 if i % (n // 4) == 0 else (0.8 if i % 2 == 0 else 0.62)
            v = vary(accent * (0.3 + 0.4 * g) * (1 + 0.6 * i / n if name == "build" else 1), 0.08)
            if name == "idle" and i % 2:
                continue
            ms = 1.5 if name == "guides" else 4.0
            hard = 0.55 if name in ("guides", "tutti") else 0.35
            mix.add("marimba", kit.marimba(tones[k], v, hard), human(t, ms), 1.0, 0.35 - 0.05 * (k % 5),
                    room=0.35, plate=0.15)


KICKS = {"theme": ((0, 0.75), (2, 0.6)), "full": ((0, 0.8), (1.75, 0.4), (2, 0.65)),
         "lift": ((0, 0.85), (1.75, 0.45), (2, 0.7), (3.5, 0.35)), "guides": ((0, 0.75), (2, 0.6)),
         "tutti": ((0, 1.0), (2, 0.8)), "pull": ((0, 0.4), (1, 0.3), (2, 0.4), (3, 0.3))}
SLAPS = {"theme": (1, 3), "full": (1, 3), "lift": (1, 3), "guides": (1, 3, 3.75), "tutti": (1, 3)}
TAPS = ("theme", "full", "lift", "guides", "tutti", "idle", "launch")


def drums(mix):
    for bar in range(4, BARS_TOTAL):
        t0 = bar * BAR
        name = section(t0)
        g = level(t0)
        for beat, v in KICKS.get(name, ()):
            t = t0 + beat * BEAT
            if name == "pull":
                v *= 0.6 + 0.4 * (bar - 53) / 2
            vv = vary(v * (0.75 + 0.3 * g), 0.05)
            mix.add("kick", kit.kick(vv), human(t, 2), 1.0)
            mix.kicks.append((t, vv))
        for beat in SLAPS.get(name, ()):
            vel = 0.35 if beat % 1 else 0.62
            mix.add("brush", kit.slap(vary(vel * (0.7 + 0.35 * g), 0.08)), human(t0 + beat * BEAT, 4), 1.0, 0.12,
                    room=0.4)
        if name in TAPS:
            soft = 0.45 if name in ("idle", "launch") else 1.0
            for i in range(16):
                accent = (0.5, 0.22, 0.32, 0.25)[i % 4]
                if name == "launch":
                    accent *= 0.5 + i / 16
                t = t0 + i * BEAT / 4 + (0.012 if i % 2 else 0.0)
                mix.add("brush", kit.tap(vary(accent * soft * (0.5 + 0.5 * g), 0.15)), human(t, 3), 1.0,
                        0.3 * np.sin(i), room=0.3)
            if name not in ("idle", "launch"):
                mix.add("brush", kit.swish(0.9, vary(0.32, 0.1), 0.7), t0 + 0.5, 1.0, room=0.3)


def transitions(mix):
    """Brush rolls into each chapter; the tutti's is longest."""
    for c in CHAPTERS[1:-1]:
        if c in (78.0, 106.0):
            continue
        length = 1.0 if c != 88.0 else 2.0
        mix.add("brush", kit.roll(length, 0.5), c - length, 1.0, room=0.4)
    mix.add("brush", kit.roll(6.0, 0.36), 106.0, 1.0, room=0.4, hall=0.3)
    mix.add("fx", kit.roll(1.2, 0.45), 6.8, 1.0, hall=0.4)


def hits(mix):
    """Strong hits get kick, sub and piano; others glint."""
    for t in STRONG:
        if t == 112.0:
            v = 1.0
        else:
            v = 0.7
        mix.add("kick", kit.kick(v), t, 1.0)
        mix.kicks.append((t, v))
        low = bass_note(t)
        mix.add("keys", piano.chord([low + 12, low + 24], 0.55 * v, 1.6, 0.0), t, 0.8, -0.1, hall=0.3)
    seen = set(STRONG) | {6.0, 118.5, 34.0}
    for t in HITS:
        if t in seen:
            continue
        tones = chord_tones(t, 84, 96)
        m = tones[int(t * 2) % len(tones)]
        mix.add("glint", kit.marimba(m, 0.5, 0.75), t, 1.0, 0.4 * np.sin(t), plate=0.5, hall=0.3)
