"""The tunes: theme, answers, breakdown song, finale."""
import numpy as np

import piano
from dsp import SR
import strings as st
from grid import BEAT, at, human, vary

# (bar, beat, midi, beats) in F; shift 2 for G
THEME = [(0, 0, 69, 1.0), (0, 1, 72, 0.5), (0, 1.5, 79, 1.5), (0, 3, 77, 0.5), (0, 3.5, 76, 0.5),
         (1, 0, 76, 1.5), (1, 1.5, 72, 0.5), (1, 2, 74, 1.0), (1, 3, 76, 1.0),
         (2, 0, 77, 2.0), (2, 2, 74, 1.0), (2, 3, 72, 1.0),
         (3, 0, 74, 1.0), (3, 1, 70, 0.5), (3, 1.5, 69, 1.5), (3, 3, 67, 0.5), (3, 3.5, 69, 0.5),
         (4, 0, 70, 1.0), (4, 1, 72, 0.5), (4, 1.5, 74, 0.5), (4, 2, 77, 1.5), (4, 3.5, 79, 0.5)]
LAND = [(5, 0, 81, 3.0)]
TURN = [(5, 0, 76, 1.0), (5, 1, 74, 0.5), (5, 1.5, 72, 0.5), (5, 2, 78, 1.0), (5, 3, 81, 1.0)]
IDLE = [(0, 0.5, 69, 1.0), (0, 1.5, 72, 0.5), (0, 2, 76, 2.0), (1, 2, 74, 1.0), (1, 3, 72, 1.0),
        (2, 0.5, 70, 1.0), (2, 1.5, 74, 0.5), (2, 2, 81, 2.0),
        (3, 0, 77, 0.5), (3, 0.5, 79, 0.5), (3, 1, 81, 1.0), (3, 2, 84, 2.0), (4, 0, 74, 4.0)]
LAUNCH = [(1, 0, 81, 2.0), (3, 0, 74, 1.0), (3, 1, 77, 1.0), (3, 2, 79, 2.0)]
BANNER = [(0, 0, 72, 1.5), (0, 1.5, 74, 0.5), (0, 2, 76, 1.0), (0, 3, 79, 1.0),
          (1, 0, 76, 2.0), (1, 2, 72, 1.0), (1, 3, 74, 1.0),
          (2, 0, 77, 1.0), (2, 1, 74, 1.0), (2, 2, 70, 1.0), (2, 3, 69, 1.0),
          (3, 0, 70, 1.0), (3, 1, 77, 1.0), (3, 2, 76, 2.0)]
SONG = [(0, 0, 71, 1.5), (0, 1.5, 74, 0.5), (0, 2, 78, 2.0), (1, 0, 76, 3.0), (1, 3, 74, 1.0),
        (2, 0, 74, 2.0), (2, 2, 71, 1.0), (2, 3, 69, 1.0), (3, 0, 72, 1.5), (3, 1.5, 71, 0.5), (3, 2, 67, 2.0)]
ANSWER = [(0, 0, 83, 1.5), (0, 1.5, 81, 0.5), (0, 2, 79, 1.0), (0, 3, 78, 1.0),
          (1, 0, 79, 2.0), (1, 2, 76, 1.0), (1, 3, 74, 1.0),
          (2, 0, 74, 1.5), (2, 1.5, 71, 0.5), (2, 2, 69, 2.0),
          (3, 0, 72, 1.0), (3, 1, 71, 1.0), (3, 2, 74, 1.0), (3, 3, 76, 1.0)]
CROWN = [(0, 0, 71, 1.0), (0, 1, 74, 0.5), (0, 1.5, 81, 1.5), (0, 3, 79, 0.5), (0, 3.5, 78, 0.5),
         (1, 0, 76, 1.5), (1, 1.5, 79, 0.5), (1, 2, 81, 1.0), (1, 3, 76, 1.0), (2, 0, 83, 5.0)]


def shift(phrase, semis):
    return [(b, beat, m + semis, d) for b, beat, m, d in phrase]


def _shape(phrase, i, base):
    """Phrase dynamics: rise to the top, lean on long notes."""
    ms = [m for _, _, m, _ in phrase]
    lo, hi = min(ms), max(ms) + 1e-9
    _, beat, m, d = phrase[i]
    lift = 0.82 + 0.22 * (m - lo) / (hi - lo)
    lean = 1.06 if d >= 1.5 else (0.9 if d <= 0.5 else 1.0)
    down = 1.04 if beat == 0 else 1.0
    return vary(base * lift * lean * down, 0.06)


def play_piano(mix, phrase, bar0, base=0.55, octave=False, legato=1.25, send=0.32):
    for i, (b, beat, m, d) in enumerate(phrase):
        t = human(at(bar0 + b, beat), 6)
        v = _shape(phrase, i, base)
        dur = d * BEAT * legato
        notes = [m, m - 12] if octave else [m]
        for k, mm in enumerate(notes):
            y = piano.note(mm, v * (0.72 if k else 1.0), dur)
            mix.add("keys", y, t + 0.006 * k, 1.0, 0.08, plate=send, hall=0.2)


def play_strings(mix, phrase, bar0, base=0.6, body="violin", octave=0, send=0.45):
    """Legato: each note overlaps the next by a bow."""
    for i, (b, beat, m, d) in enumerate(phrase):
        t = at(bar0 + b, beat) - 0.03
        v = _shape(phrase, i, base)
        dur = d * BEAT + 0.08
        swell = [(0, 0.75), (dur * 0.5, 1.0), (dur + 0.5, 0.85)] if d >= 1.5 else None
        y = st.bowed(m + octave, dur, v, body, attack=0.09, release=0.35, players=5, swell=swell)
        mix.add("bowed", y, t, 1.0, 0.25, hall=send)


def tunes(mix):
    play_piano(mix, THEME + LAND, 4, 0.5)
    play_piano(mix, IDLE, 10, 0.34, legato=1.6)
    play_piano(mix, LAUNCH, 15, 0.44)
    play_piano(mix, BANNER, 19, 0.46)
    play_piano(mix, THEME + TURN, 23, 0.58)
    play_piano(mix, [(0, 0, 83, 0.5), (1, 0, 81, 0.5), (1, 2, 78, 0.5), (2, 0, 76, 1.0), (3, 2, 81, 2.0)],
               29, 0.36)
    play_strings(mix, shift(THEME, 2) + [(5, 0, 81, 3.5)], 34, 0.5)
    play_piano(mix, SONG, 39, 0.42, legato=1.8, send=0.45)
    play_piano(mix, shift(THEME, 2), 44, 0.6, octave=True)
    play_strings(mix, shift(THEME, 2), 44, 0.36, octave=12, send=0.5)
    play_piano(mix, ANSWER, 49, 0.56)
    play_strings(mix, ANSWER, 49, 0.32, "viola", -12)
    play_piano(mix, CROWN, 56, 0.72, octave=True, send=0.42)
    play_strings(mix, CROWN, 56, 0.62, octave=0, send=0.55)
    play_strings(mix, shift(CROWN, -12), 56, 0.5, "viola", send=0.5)
    lift(mix)


def lift(mix):
    """The pen lifts: one high note rings into the tail."""
    y = piano.note(86, 0.3, 1.6)
    mix.add("keys", y, 118.5, 1.0, 0.25, plate=0.6, hall=0.6)
    y = st.harp(98, 0.25, 2.2)
    mix.add("harp", y, 118.5 + 0.01, 0.6, 0.3, hall=0.7)


def touch(mix):
    """1.0: the pen lands; held line rises six, seven, eight."""
    rev = piano.note(74, 0.32, 1.2)[::-1][-int(0.9 * SR):]
    mix.add("keys", rev * np.linspace(0.2, 1, len(rev)) ** 2, 1.0 - len(rev) / SR, 0.15, -0.1, hall=0.5)
    mix.add("keys", piano.note(62, 0.36, 4.5), 1.0, 1.0, -0.05, plate=0.4, hall=0.5)
    mix.add("keys", piano.note(50, 0.26, 4.5), 1.012, 0.8, -0.2, hall=0.4)
    glide = [(0, 0), (4.92, 0), (5.08, 2), (6.92, 2), (7.04, 3), (9.0, 3)]
    swell = [(0, 0.35), (2.0, 0.55), (4.8, 0.7), (5.2, 0.85), (6.9, 0.9), (7.1, 1.0), (9.0, 0.8)]
    y = st.bowed(62, 8.2, 0.42, "viola", attack=1.6, release=1.2, players=4, swell=swell, glide=glide)
    mix.add("bowed", y, 1.0, 1.0, 0.0, hall=0.55)
    y = st.bowed(50, 7.2, 0.3, "cello", attack=2.5, release=1.0, players=3,
                 swell=[(0, 0.2), (4.0, 0.6), (7.2, 0.9)],
                 glide=[(0, 0), (3.92, 0), (4.08, -2), (6.0, -2), (6.12, -9), (8.0, -9)])
    mix.add("cello", y, 2.0, 1.0, -0.1, hall=0.4)
    mix.add("keys", piano.chord([52, 55, 62, 67], 0.34, 1.9), 6.0, 1.0, 0.0, plate=0.4, hall=0.4)
    mix.add("keys", piano.note(76, 0.3, 1.8), 6.02, 1.0, 0.2, plate=0.5, hall=0.4)
