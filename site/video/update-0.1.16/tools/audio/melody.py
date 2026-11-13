"""Tunes: the kalimba theme, the whistled bridge, and their placements."""
import numpy as np

import instruments as ins
from dsp import rng
from grid import at, EIGHTH, human, scale_tones, vary

# (eighth, midi, length in eighths, velocity); one list per bar
THEME = [
    [(0, 81, 1, .9), (1, 78, 1, .7), (2, 81, 1, .8), (4, 86, 3, .95), (7, 85, 1, .7)],
    [(0, 83, 2, .9), (2, 81, 1, .75), (3, 78, 3, .85), (6, 74, 1, .7), (7, 76, 1, .75)],
    [(0, 78, 3, .9), (3, 79, 1, .7), (4, 81, 1, .8), (5, 83, 2, .85), (7, 86, 1, .8)],
    [(0, 88, 2, .95), (2, 86, 1, .8), (3, 83, 1, .75), (4, 85, 2, .9), (6, 76, 2, .7)],
]
ANSWER = [
    [(0, 81, 1, .9), (1, 78, 1, .7), (2, 81, 1, .8), (4, 90, 3, 1.0), (7, 88, 1, .75)],
    [(0, 86, 2, .9), (2, 83, 1, .75), (3, 81, 2, .8), (5, 79, 1, .7), (6, 76, 2, .75)],
]
SLIDE = [
    [(1, 81, 1, .8), (3, 78, 1, .75), (5, 81, 1, .8), (6, 86, 2, .9)],
    [(1, 83, 1, .8), (3, 81, 1, .75), (5, 78, 1, .75), (6, 74, 2, .8)],
]
TEAR = [(0, 79, 3, .7), (3, 82, 3, .6), (6, 86, 2, .55)]
RUN = [(0, 78, 1, .7), (1, 81, 1, .75), (2, 85, 1, .8), (3, 88, 1, .85), (4, 87, 1, .8), (5, 85, 1, .75),
       (6, 83, 1, .7), (7, 78, 1, .7)]
HOLD_D = [(0, 86, 4, .9), (4, 90, 2, .7), (6, 88, 2, .6)]

BRIDGE = [   # whistle, bars 10-15
    [(0, 83, 3, .8), (3, 86, 1, .75), (4, 85, 4, .85)],
    [(0, 81, 4, .8), (4, 78, 2, .7), (6, 76, 2, .7)],
    [(2, 79, 1, .7), (3, 83, 1, .75), (4, 86, 4, .85)],
    [],
    [(0, 90, 3, .85), (3, 88, 1, .75), (4, 86, 2, .8), (6, 85, 2, .75)],
    [(0, 83, 2, .75), (2, 86, 2, .8), (4, 85, 3, .8), (7, 81, 1, .65)],
]
COUNTER = [(16, 2, 85, 6), (17, 2, 86, 6), (18, 0, 90, 8), (19, 0, 88, 4), (19, 4, 85, 4), (20, 0, 86, 7),
           (36, 2, 87, 6), (37, 2, 88, 6), (38, 0, 92, 8), (39, 0, 90, 4), (39, 4, 87, 4), (40, 0, 88, 7)]


def place(table, first_bar, shift=0):
    return [(first_bar + i, e, m + shift, d, v) for i, bar in enumerate(table) for e, m, d, v in bar]


def theme_notes():
    notes = place(THEME, 4) + place(ANSWER, 8) + place(THEME, 16) + place([HOLD_D], 20)
    notes += place(THEME, 24) + place(SLIDE, 28) + place([TEAR], 30) + place([RUN], 31)
    lift = place(THEME, 32, 2) + place([ANSWER[0]] + THEME[1:], 36, 2) + place([HOLD_D], 40, 2)
    return notes + [n for n in lift if not (n[0] == 35 and n[1] < 4)]


def below(t, m):
    pool = scale_tones(t, m - 9, m - 3)
    return pool[-1] if pool else None


def kalimba(mix):
    for bar, e, m, d, v in theme_notes():
        t = human(at(bar, e), 3)
        ring = 0.45 + 0.12 * d
        mix.add("kalimba", ins.kalimba(m, vary(v), ring), t, p=0.12, room=0.25, hall=0.2, delay=0.12)
        if d >= 2 and bar >= 16:
            h = below(t, m)
            if h:
                mix.add("kalimba", ins.kalimba(h, vary(v * 0.5), ring), t + 0.012, p=-0.1, room=0.25, hall=0.2)

    # 2.0: the match catches; a rolled sparkle
    for k, m in enumerate((81, 86, 88, 90, 93, 98)):
        mix.add("kalimba", ins.kalimba(m, 0.55 - 0.05 * k, 0.9), 2.0 + k * 0.045, p=-0.5 + 0.2 * k, hall=0.5,
                delay=0.3)
    for t, m, v in ((3.45, 81, .35), (3.5, 86, .4), (4.45, 78, .35), (4.6, 83, .4), (7.535, 76, .55),
                    (7.75, 78, .6)):                   # leap, a question, the pickup
        mix.add("kalimba", ins.kalimba(m, v, 0.5), t, p=0.1, room=0.3, hall=0.3)

    for bar in range(10, 16):                          # bridge answers in the whistle's gaps
        for e, idx in ((5, 2), (7, 3)):
            if bar in (12, 13):
                continue
            t = at(bar, e)
            pool = scale_tones(t, 74, 90)
            mix.add("kalimba", ins.kalimba(pool[idx], vary(0.42), 0.5), human(t, 3), p=0.4, room=0.3, delay=0.3)
    for k, m in enumerate((85, 88, 91, 93)):           # 26-28: one glyph a beat
        mix.add("kalimba", ins.kalimba(m, 0.75, 0.8), 26.0 + 0.5 * k, p=-0.3 + 0.2 * k, room=0.3, hall=0.35)
    for k, m in enumerate((98, 93, 90, 86)):
        mix.add("kalimba", ins.kalimba(m, 0.4, 0.4), 28.0 + 0.06 * k, p=0.3, hall=0.3)

    for bar in (21, 22):                               # breakdown: slow, open arpeggios
        for e in (0, 3, 6):
            t = at(bar, e)
            pool = scale_tones(t, 74, 93)
            mix.add("kalimba", ins.kalimba(pool[(e // 3) * 2 + 1], vary(0.38), 1.0), human(t, 5), p=-0.35 + 0.15 * e,
                    hall=0.45, delay=0.35)
    for k in range(8):                                 # 47-48 climbs into the formula
        t = 47.0 + k * 0.125
        pool = scale_tones(t, 76, 98)
        mix.add("kalimba", ins.kalimba(pool[min(k + 2, len(pool) - 1)], 0.3 + 0.05 * k, 0.4), t, p=0.5 * np.sin(k),
                room=0.3)
    for t, m in ((66.0, 88), (66.25, 88)):             # two ghost characters
        mix.add("kalimba", ins.kalimba(m, 0.6, 0.6), t, p=-0.3 if t == 66.0 else 0.3, room=0.3, hall=0.3)
    for k, m in enumerate((83, 88, 92)):               # three commits lift
        mix.add("kalimba", ins.kalimba(m, 0.5, 0.5), 72.0 + 0.25 * k + (0.035 if k % 2 else 0), p=0.2, room=0.3)

    for k in range(28):                                # montage: sixteenths rising, a frame each
        t = 82.0 + k * 0.125
        if t >= 85.75:
            break
        pool = scale_tones(t, 74, 100)
        idx = (k % 4) + (k // 4) * 1
        mix.add("kalimba", ins.kalimba(pool[min(idx, len(pool) - 1)], 0.32 + 0.012 * k, 0.35), human(t, 2),
                p=0.6 * np.sin(k * 1.3), room=0.3, delay=0.15)
    roll = (76, 80, 83, 85, 88, 90, 92)
    for k, m in enumerate(roll):                       # 86: the tutti, then a slow roll down
        mix.add("kalimba", ins.kalimba(m, 0.7, 1.2), 86.0 + k * 0.03, p=-0.6 + 0.2 * k, hall=0.5, delay=0.2)
    for k, m in enumerate((92, 88, 85, 83, 80)):
        mix.add("kalimba", ins.kalimba(m, 0.32 - 0.03 * k, 1.0), 87.0 + 0.25 * k, p=0.4 - 0.2 * k, hall=0.5)
    mix.add("kalimba", ins.kalimba(95, 0.55, 0.9), 88.5, p=0.25, hall=0.6, delay=0.4)


def whistle(mix):
    for bar, e, m, d, v in place(BRIDGE, 10):
        t = at(bar, e, swing=False)
        mix.add("whistle", ins.whistle(m, d * EIGHTH * 0.92, vary(v, 0.05)), t, p=-0.15, room=0.3, hall=0.35,
                delay=0.15)
    for bar, e, m, d in COUNTER:
        t = at(bar, e, swing=False)
        mix.add("whistle", ins.whistle(m, d * EIGHTH * 0.95, 0.5, scoop=-0.6), t, p=-0.35, room=0.3, hall=0.4)
    mix.add("whistle", ins.whistle(83, 2.6, 0.6, scoop=-2.0), 86.25, p=-0.2, hall=0.5, delay=0.2)


def glock(mix):
    for t, notes in ((8.0, (90, 93)), (36.0, (91, 95)), (40.0, (93, 98)), (48.0, (90, 98)), (64.0, (92, 95)),
                     (86.0, (92, 95, 100))):
        for k, m in enumerate(notes):
            mix.add("glock", ins.glock(m, 0.7, 0.8), t + 0.02 * k, p=0.4 - 0.4 * k, hall=0.5, delay=0.25)
    for k in range(6):                                 # 40.0: the splash blooms
        m = (86, 90, 93, 95, 98, 102)[k]
        mix.add("glock", ins.glock(m, 0.35, 0.6), 40.0 + 0.07 * k, p=rng().uniform(-0.7, 0.7), hall=0.5)
    mix.add("glock", ins.glock(107, 0.45, 0.6), 88.5, p=0.25, hall=0.6, delay=0.4)
    mix.add("post", ins.kalimba(86, 0.16, 1.4), 60.0, p=-0.1)    # the saved value glows on
    mix.add("post", ins.glock(98, 0.04, 1.2), 60.02, p=0.2)
