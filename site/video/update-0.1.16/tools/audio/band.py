"""The band: brushes, kick, upright bass, nylon guitar, reed organ."""
import numpy as np

import instruments as ins
from dsp import env, hz, ns, rng, SR, secs
from grid import at, bass_root, BARS, guitar, human, scale_tones, section, vary

FULL = ("A", "A2", "A3", "C", "lift")
STOPS = ((24.02, 24.5), (60.0, 60.75), (70.0, 71.0), (85.75, 86.0))


def stopped(t):
    return any(a <= t < b for a, b in STOPS)


def kick(mix, t, vel):
    if not stopped(t):
        mix.kicks.append((t, vel))


def render_kicks(mix):
    """One kick per moment; coinciding hits keep the loudest."""
    merged = {}
    for t, v in mix.kicks:
        key = round(t / 0.02)
        if v > merged.get(key, (0, 0))[1]:
            merged[key] = (t, v)
    mix.kicks = sorted(merged.values())
    for t, v in mix.kicks:
        mix.add("kick", ins.kick(vary(v, 0.05)), t)


# Drums -------------------------------------------------------------------

def brush_bar(mix, bar, sec):
    for beat in range(4):
        t = bar * 2 + beat * 0.5
        if stopped(t):
            continue
        v = {"B": 0.32, "C": 0.36, "tear": 0.22}.get(sec, 0.4) * (1.1 if beat % 2 == 0 else 0.85)
        mix.add("brush", ins.swish(0.52, vary(v), 0.55 if beat % 2 == 0 else 0.4), human(t - 0.03, 4),
                p=-0.25 + 0.5 * (beat % 2), room=0.25)
    if sec == "tear":
        return
    for e in (2, 6):
        t = at(bar, e, False)
        if not stopped(t):
            mix.add("brush", ins.slap(vary(0.62 if sec != "B" else 0.5)), human(t, 3), p=0.08, room=0.35)
    for e in range(1, 8, 2):
        t = at(bar, e)
        if rng().uniform() < 0.3 and not stopped(t):
            mix.add("brush", ins.tap(vary(0.22)), human(t, 4), p=0.1, room=0.2)


RIDE = {"swing": (0, 2, 3, 4, 6, 7), "B": (0, 2, 3, 4, 5, 6, 7), "eighths": tuple(range(8))}


def ride_bar(mix, bar, pattern, vel):
    for e in RIDE[pattern]:
        t = at(bar, e)
        if not stopped(t):
            v = vel * (1.0 if e % 2 == 0 else 0.65)
            mix.add("ride", ins.ride(vary(v)), human(t, 3), p=0.45, room=0.3)


def drums(mix):
    for bar in range(45):
        sec = section(bar * 2)
        if sec in FULL or sec in ("B", "tear", "montage"):
            brush_bar(mix, bar, sec)
        if sec in FULL:
            kick(mix, at(bar, 0, False), 0.75)
            kick(mix, at(bar, 4, False), 0.55)
            if bar % 2:
                kick(mix, at(bar, 7), 0.3)
            ride_bar(mix, bar, "swing", 0.42)
            if sec == "C":
                for e in range(8):
                    mix.add("perc", ins.shaker(vary(0.3 if e % 2 else 0.2)), human(at(bar, e), 3), p=-0.4)
        elif sec == "B":
            kick(mix, at(bar, 0, False), 0.65)
            kick(mix, at(bar, 3), 0.35)
            ride_bar(mix, bar, "B", 0.5)
        elif sec == "montage":
            for e in (0, 2, 4, 6):
                kick(mix, at(bar, e, False), 0.65 + 0.05 * (bar - 41))
            ride_bar(mix, bar, "eighths", 0.45)

    for k in range(16):                                # 6-8: brushes wake up
        t = 6.0 + k * 0.125
        mix.add("brush", ins.tap(0.06 + 0.2 * k / 16), human(t, 3), p=0.1 * np.sin(k), room=0.3)
    mix.add("brush", ins.swish(2.0, 0.5, 0.95), 6.0, room=0.3)
    roll(mix, 46.0, 48.0, 0.06, 0.45)
    roll(mix, 84.0, 85.75, 0.08, 0.6)
    roll(mix, 62.5, 64.0, 0.05, 0.35)
    for t, length in ((6.0, 2.0), (46.0, 2.0), (62.0, 2.0)):
        mix.add("fx", ins.swell(length, 0.5), t, room=0.2, hall=0.2)
    mix.add("fx", ins.swell(3.75, 0.7)[:ns(3.75)], 82.0, room=0.2, hall=0.2)


def roll(mix, t0, t1, v0, v1):
    """Brush buzz roll, thirty-seconds, swelling."""
    k, t = 0, t0
    while t < t1 - 1e-6:
        u = (t - t0) / (t1 - t0)
        mix.add("brush", ins.tap(vary(v0 + (v1 - v0) * u ** 1.6, 0.12)), human(t, 1.5), p=0.15 * (-1) ** k, room=0.3)
        t += 0.0625
        k += 1


# Bass --------------------------------------------------------------------

def pizz(mix, m, t, dur, vel):
    """Muted early when a stop follows."""
    dur = min([dur] + [a - t for a, _ in STOPS if t < a < t + dur])
    if not stopped(t):
        mix.add("bass", ins.pizz(m, vary(vel, 0.06), dur), human(t, 3), room=0.15)


def fifth(r):
    return r + 7 if r + 7 <= 43 else r - 5


def bass(mix):
    for bar in range(45):
        sec = section(bar * 2)
        t0 = bar * 2.0
        nxt = bass_root(t0 + 2.0) if bar < 44 else None
        if sec in ("A", "A2", "A3", "lift"):
            split = "|" in BARS[bar]
            pizz(mix, bass_root(t0), t0, 0.9, 0.85)
            pizz(mix, bass_root(t0 + 1) if split else fifth(bass_root(t0)), t0 + 1, 0.9, 0.7)
            if nxt is not None and rng().uniform() < 0.6:
                pizz(mix, nxt + rng().choice([-1, 1]), at(bar, 7), 0.2, 0.55)
        elif sec == "B":
            r = bass_root(t0)
            pizz(mix, r, t0, 0.7, 0.85)
            pizz(mix, fifth(r), at(bar, 3), 0.35, 0.6)
            pizz(mix, bass_root(t0 + 1), t0 + 1, 0.7, 0.75)
            if nxt is not None:
                pizz(mix, nxt - 1, at(bar, 7), 0.2, 0.55)
        elif sec == "C":
            walk(mix, bar)
        elif sec == "break":
            pizz(mix, bass_root(t0), t0, 1.9, 0.6)
        elif sec == "montage":
            for e in range(8):
                r = bass_root(at(bar, e, False))
                pizz(mix, r + (12 if e % 2 else 0), at(bar, e), 0.2, 0.75 + 0.02 * e)

    for t, m, d, v in SPOTS:
        mix.add("bass", ins.pizz(m, v, d), t, room=0.15)
    mix.add("post", slide(35, 47, 0.5, 0.3), 70.0)


# landing, walk-ups, the tear, the tutti
SPOTS = ((4.0, 43, 0.9, 0.75), (5.0, 38, 0.9, 0.6), (6.0, 33, 0.9, 0.85), (7.0, 33, 0.45, 0.65),
         (7.5, 35, 0.22, 0.6), (7.75, 37, 0.22, 0.65), (46.0, 33, 0.45, 0.75), (46.5, 35, 0.45, 0.7),
         (47.0, 37, 0.45, 0.75), (47.5, 39, 0.45, 0.8), (60.75, 31, 1.2, 0.75), (62.0, 30, 0.45, 0.75),
         (62.5, 33, 0.45, 0.7), (63.0, 35, 0.45, 0.75), (63.5, 39, 0.45, 0.8), (86.0, 28, 1.6, 1.0))


def walk(mix, bar):
    """Walking quarters: root, chord tones, approach to the next root."""
    t0 = bar * 2.0
    r = bass_root(t0)
    target = bass_root(t0 + 2.0)
    pool = scale_tones(t0 + 1, 28, 45)
    line = [r, rng().choice([m for m in pool if 3 <= abs(m - r) <= 7] or [r + 7]),
            bass_root(t0 + 1) if "|" in BARS[bar] else rng().choice(pool[1:-1]), target + rng().choice([-1, 1])]
    for k, m in enumerate(line):
        pizz(mix, m, t0 + 0.5 * k, 0.45, 0.8 if k == 0 else 0.68)


def slide(m0, m1, dur, vel):
    """Bass slides up: the titles stretch."""
    n = ns(dur + 0.1)
    t = secs(n)
    u = np.clip(t / dur, 0, 1)
    f = hz(m0 + (m1 - m0) * (1 - np.cos(np.pi * u)) / 2)
    ph = 2 * np.pi * np.cumsum(f) / SR
    y = np.sin(ph) + 0.35 * np.sin(2 * ph) + 0.12 * np.sin(3 * ph)
    return y * env(n, 0.01, 0.1, hold=dur) * vel


# Guitar ------------------------------------------------------------------

TRAVIS = [(0, "low", .7), (0, "top", .45), (1, 1, .4), (2, "fifth", .55), (3, 2, .45), (4, "low", .6), (5, "top", .45),
          (6, "fifth", .5), (7, 1, .4)]


def pick(v, which):
    if isinstance(which, int):
        return v[which]
    return {"low": v[0], "fifth": v[0] + 7, "top": v[-1]}[which]


def guitar_part(mix):
    for bar in range(45):
        sec = section(bar * 2)
        if sec in ("intro", "A", "A2", "A3", "C", "lift", "break"):
            soft = {"intro": 0.6, "break": 0.65}.get(sec, 1.0)
            for e, which, v in TRAVIS:
                t = at(bar, e)
                if stopped(t):
                    continue
                m = pick(guitar(t), which)
                mix.add("nylon", ins.nylon(m, vary(v * soft), 2.2 if sec == "break" else 1.4), human(t, 5),
                        p=-0.3 if which in ("low", "fifth") else 0.25, room=0.3, hall=0.1)
        elif sec == "B":
            for e, v, down, ring in ((0, .65, True, 1.2), (3, .45, False, 0.25), (4, .55, True, 0.6),
                                     (6, .45, False, 0.25), (7, .35, True, 0.2)):
                t = at(bar, e)
                if not stopped(t):
                    mix.add("nylon", ins.strum(guitar(t), vary(v), ring, 0.012, down), human(t, 4), room=0.3)
        elif sec == "tear":
            for e, idx in ((0, 1), (2, 2), (3, 3), (5, 2), (6, 1)):
                t = at(bar, e)
                m = guitar(t)[idx]
                mix.add("nylon", ins.nylon(m, 0.4, 2.0, 0.2), human(t, 6), p=0.2, room=0.3, hall=0.3)
        elif sec == "build":
            for e in range(8):
                t = at(bar, e)
                mix.add("nylon", ins.strum(guitar(t), 0.35 + 0.05 * e, 0.3, 0.01, e % 2 == 0), t, room=0.3)
        elif sec == "montage":
            for e in range(8):
                t = at(bar, e)
                if not stopped(t):
                    mix.add("nylon", ins.strum(guitar(t), 0.5 + 0.03 * e, 0.35, 0.009, e % 2 == 0), human(t, 3),
                            room=0.3)


def reed(mix):
    for t0, t1, name_t, v in ((2.0, 4.6, 2.0, 0.45), (42.0, 44.0, 42.0, 0.5), (44.0, 46.0, 44.0, 0.5),
                              (46.0, 47.0, 46.0, 0.55), (47.0, 48.0, 47.0, 0.6), (60.75, 62.2, 60.75, 0.5)):
        mix.add("reed", ins.reed(guitar(name_t)[1:], t1 - t0, v, attack=0.6 if t0 == 2.0 else 0.25), t0, hall=0.3)
    for bar in range(32, 41):                          # the lift: a warm bed
        starts = (0, 1) if "|" in BARS[bar] else (0,)
        for k in starts:
            t = bar * 2 + k
            mix.add("reed", ins.reed(guitar(t)[1:], 2.0 / len(starts) - 0.05, 0.28, 0.08, 0.3), t, hall=0.25)
    mix.add("reed", ins.reed([52] + guitar(86.0)[1:], 3.9, 0.7, 0.03, 0.4), 86.0, hall=0.4)


HITS = ((6.0, 1), (8.0, 2), (16.0, 1), (24.0, 1), (36.0, 2), (40.0, 2), (48.0, 2), (52.0, 0), (56.0, 1), (64.0, 2),
        (66.0, 0), (76.0, 1), (80.0, 1), (86.0, 3))


def hits(mix):
    """Band hits on the storyboard's beats."""
    for t, size in HITS:
        ring = 0.3 if t == 24.0 else 1.6 + 0.6 * size
        mix.add("nylon", ins.strum(guitar(t) + [guitar(t)[-1] + 12], 0.7 + 0.1 * min(size, 2), ring, 0.014), t - 0.03,
                room=0.35, hall=0.15 * size)
        mix.add("brush", ins.slap(0.55 + 0.12 * min(size, 2)), t, p=0.05, room=0.4)
        kick(mix, t, 0.85 + 0.05 * min(size, 2))
        if size >= 2:
            mix.add("fx", ins.crash(0.35 + 0.1 * size, 3.5), t, p=0.3, room=0.2, hall=0.2)
