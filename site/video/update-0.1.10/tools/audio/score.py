"""The score: 120 BPM, 45 bars, E major, F# chorus."""
import numpy as np

import instruments as ins
from dsp import butter, env, ns, reseed, rng, secs, SR
from grid import at, CHANGES, human, Mix, root, section, SEED, STEP, SWING, vary, voicing, VOICINGS
from master import master, stem_report


# Drums ------------------------------------------------------------------

A_HATS = [.34, .16, None, .16] * 4          # None: open hat on the offbeat eighth
KITS = {
    "groove": dict(kick={0: .5, 10: .35}, snap={4: .7, 12: .75}, shaker=[.22, .08, .15, .08] * 4),
    "A": dict(kick={0: 1, 4: .95, 8: 1, 12: .95}, clap={4: .9, 12: .95}, hats=A_HATS, shaker=[.18, .07, .11, .07] * 4),
    "B1": dict(kick={0: .85, 10: .6}, snap={4: .8, 12: .85}, shaker=[.22, .1, .16, .1] * 4),
    "B2": dict(kick={0: .9, 8: .85, 10: .5}, clap={4: .8, 12: .85}, hats=[.3, 0, .18, 0] * 4,
               shaker=[.2, .08, .14, .08] * 4),
    "B3": dict(kick={0: 1, 4: .95, 8: 1, 12: .95}, clap={4: .9, 12: .95}, hats=[.32, .15, .26, .15] * 4,
               shaker=[.18, .07, .11, .07] * 4),
    "half": dict(),
    "peak": dict(kick={0: 1, 4: .95, 8: 1, 12: .95}, clap={4: 1, 12: 1}, snare={4: .45, 12: .5},
                 hats=[.38, .2, None, .2] * 4, shaker=[.2, .08, .13, .08] * 4, tamb=[0, 0, .3, 0] * 4),
    "outro": dict(kick={0: .55}, snap={4: .45, 12: .5}, shaker=[.16, .06, .1, .06] * 4),
}
KITS["back"] = KITS["C"] = KITS["Cbuild"] = KITS["A"]
KITS["build"] = dict(kick={0: 1, 4: .95, 8: 1, 12: .95}, clap={4: .85, 12: .9}, hats=[.25, .2, .28, .2] * 4)
KITS["peakbuild"] = KITS["peak"]


def roll(mix, t0, t1, v0, v1, segments):
    """Snare roll; segments are (span fraction, step seconds)."""
    t = t0
    while t < t1 - 1e-6:
        u = (t - t0) / (t1 - t0)
        step = [s for f, s in segments if u >= f - 1e-9][-1]
        mix.add("perc", ins.snare(vary(v0 + (v1 - v0) * u ** 1.5, 0.05)), human(t, 1.5), p=0.08,
                plate=0.25, hall=0.08)
        t += step


def toms(mix, times, pitches, vel=0.75):
    for i, (t, m) in enumerate(zip(times, pitches)):
        mix.add("toms", ins.tom(m, vary(vel)), human(t, 1.5), p=0.5 - i * 0.2, plate=0.2, hall=0.1)


def drums(mix):
    for bar in range(45):
        for step in range(16):
            t = at(bar, step)
            sec = section(at(bar, step, False))
            kit = KITS.get(sec)
            if not kit:
                continue
            if sec == "groove" and t < 5.45:
                continue
            if sec == "build" and t >= 46.0 and step not in (0, 4):
                continue
            if sec == "peakbuild" and t >= 77.0:
                continue
            if step in kit.get("kick", {}) and not (sec == "groove" and t < 6.0) and not (sec == "build" and t >= 46.5):
                mix.kick(human(at(bar, step, False), 1.0), vary(kit["kick"][step], 0.04))
            if step in kit.get("clap", {}) and not (sec == "build" and t >= 46.0):
                mix.add("clap", ins.clap(vary(kit["clap"][step])), human(t, 2), p=0.04, plate=0.3, hall=0.06)
            if step in kit.get("snap", {}):
                mix.add("snap", ins.snap(vary(kit["snap"][step])), human(t, 3), p=-0.12, plate=0.35, hall=0.12)
            if step in kit.get("snare", {}):
                mix.add("perc", ins.snare(vary(kit["snare"][step])), human(t, 2), p=0.05, plate=0.25)
            hats = kit.get("hats")
            if hats:
                v = hats[step]
                lift = 1 + 0.6 * max(0.0, (t - 44) / 2) if sec == "build" else 1.0
                if v is None:
                    mix.add("hats", ins.hat(vary(0.34), True), human(t, 3), p=0.3, plate=0.08)
                elif v:
                    mix.add("hats", ins.hat(vary(v * lift)), human(t, 3), p=0.3)
            shake = kit.get("shaker")
            if shake and not (sec == "groove" and t < 8.0):
                v = shake[step] * (1 + 0.8 * max(0.0, t - 10.0) if sec == "groove" else 1)
                if sec == "outro":
                    v *= max(0.0, 1 - (t - 80) / 7)
                mix.add("perc", ins.shaker(vary(v)), human(t, 3), p=-0.35)
            tamb = kit.get("tamb")
            if tamb and tamb[step]:
                mix.add("perc", ins.tamb(vary(tamb[step])), human(t, 3), p=0.45, plate=0.1)

    # hats lift into the drop
    for step in range(1, 12, 2):
        mix.add("hats", ins.hat(0.12 + 0.03 * step), at(5, step), p=0.3)
    # half time: a lazy pocket under the stat hit
    for t, v in ((48.0, .8), (48.75, .55), (49.25, .5)):
        mix.kick(t, v)
    mix.add("clap", ins.clap(0.85), 49.0, p=0.04, plate=0.45, hall=0.2)
    for k in range(6):
        mix.add("hats", ins.hat(vary(0.22)), 47.5 + k * 0.5 + SWING, p=0.3)

    roll(mix, 10.5, 11.5, 0.12, 0.5, [(0, 0.125), (0.5, 0.0625)])
    roll(mix, 44.0, 47.0, 0.12, 0.85, [(0, 0.25), (1 / 3, 0.125), (2 / 3, 0.0625), (11 / 12, 0.03125)])
    roll(mix, 59.0, 60.0, 0.2, 0.7, [(0, 0.125), (0.5, 0.0625)])
    roll(mix, 77.0, 78.0, 0.25, 0.85, [(0, 0.125), (0.5, 0.0625), (0.75, 0.03125)])
    toms(mix, (25.5, 25.625, 25.75, 25.875), (64, 61, 57, 52), 0.6)
    toms(mix, (59.0, 59.25, 59.5, 59.625, 59.75, 59.875), (66, 63, 61, 58, 54, 51))
    for t in (63.75, 63.875, 71.75, 71.875):
        mix.add("clap", ins.clap(0.5), human(t, 2), p=0.2, plate=0.3)
    toms(mix, (67.0, 67.25, 67.5, 67.625, 67.75, 67.875), (68, 66, 63, 61, 58, 54), 0.85)
    toms(mix, (75.0, 75.25, 75.5, 75.625, 75.75, 75.875), (70, 68, 66, 63, 61, 58), 0.9)
    toms(mix, (78.5, 79.0, 79.25, 79.5), (54, 58, 61, 66), 0.45)


# Bass -------------------------------------------------------------------

BOUNCE = [(0, 0, 2.5, 1.0), (3, 12, 1, .7), (6, 0, 1.5, .85), (8, 0, 2, .9), (10, 12, 1, .7),
          (12, 7, 2, .8), (14, 12, 1, .7)]
RUBBER = [(0, 0, 2, 1.0, -3), (2, 12, 1, .7, 0), (3, 0, 2, .8, 0), (6, 12, 1, .75, 0), (7, 0, 1, .65, 0),
          (8, 0, 2, .95, 0), (10, 12, 1, .7, 0), (11, 7, 1, .6, 0), (12, 0, 2, .85, 0), (14, 12, 1, .7, 0),
          (15, 7, 1, .6, 2)]
LIGHT = [(0, 0, 3, 1.0), (6, 0, 1, .7), (8, 0, 3, .9), (14, 12, 1, .7)]


def bass(mix):
    for bar in range(45):
        for pat, voice in ((BOUNCE, "sub"), (RUBBER, "rubber"), (LIGHT, "sub")):
            for row in pat:
                step, octave, length, vel = row[:4]
                t = at(bar, step, swing=False) + (0.006 if step % 2 else 0.0)
                sec = section(t)
                want = {"groove": BOUNCE, "B1": LIGHT, "B2": LIGHT}.get(
                    sec, RUBBER if sec in ("A", "B3", "back", "build", "C", "Cbuild", "peak", "peakbuild") else None)
                if want is not pat or (sec == "groove" and t < 5.1):
                    continue
                if (sec == "build" and t >= 46.0) or (sec == "Cbuild" and t >= 59.0) or (sec == "peakbuild" and t >= 77.0):
                    continue
                d = length * STEP * 0.9
                if voice == "sub":
                    mix.add("bass", ins.sub(root(t) + octave, d, vary(vel * (0.7 if sec == "groove" else 0.85), 0.05)), t)
                else:
                    mix.add("bass", ins.rubber(root(t) + octave, d, vary(vel, 0.05), glide=row[4],
                                               bite=1.15 if sec.startswith("peak") else 1.0), t)
    for t in np.arange(46.0, 46.75, 0.125):       # driving eighths into the impact
        mix.add("bass", ins.rubber(35 + (12 if int(t * 8) % 2 else 0), 0.1, 0.8), t)
    for t in np.arange(59.0, 60.0, 0.125):
        mix.add("bass", ins.rubber(root(t) + (12 if int(t * 8) % 2 else 0), 0.1, 0.8), t)
    for t in np.arange(77.0, 77.75, 0.125):
        mix.add("bass", ins.rubber(root(t) + (12 if int(t * 8) % 2 else 0), 0.1, 0.85), t)
    mix.add("bass", ins.sub(40, 0.45, 1.0, glide=-5), 5.0)
    for t, m, d, v in ((47.0, 28, 0.95, 1.0), (48.0, 37, 1.4, .9), (49.5, 49, 0.2, .7), (49.75, 37, 0.2, .75)):
        mix.add("bass", ins.sub(m, d, v, glide=-4 if t == 47.0 else 0), t)
    mix.add("bass", ins.sub(30, 1.6, 1.0, glide=-7), 78.0)
    for t in (80, 82, 84, 86):
        mix.add("bass", ins.sub(root(t), 1.6, 0.42), t)
    mix.add("bass", ins.sub(root(87), 0.8, 0.4), 87.0)
    mix.add("bass", ins.sub(30, 1.0, 0.35), 88.0)


# Harmony ----------------------------------------------------------------

PADS = {"pre": (1.4, 1.0, 1200), "intro": (1.4, 1.0, 1200), "rise": (0.3, 0.6, 2000), "groove": (0.08, 0.5, 2400),
        "A": (0.02, 0.4, 4200), "B1": (0.15, 0.6, 3200), "B2": (0.15, 0.6, 3200), "B3": (0.1, 0.5, 3600),
        "back": (0.02, 0.4, 4200), "build": (0.05, 0.3, 4200), "half": (0.4, 1.5, 3600), "C": (0.02, 0.4, 4200),
        "Cbuild": (0.02, 0.3, 4200), "peak": (0.02, 0.4, 5500), "peakbuild": (0.02, 0.3, 5500),
        "unison": (0.01, 1.6, 5500), "outro": (0.6, 1.2, 2600), "end": (0.2, 0.85, 2000)}
STABS = {"A": [(2, .9), (6, .7), (9, .8), (14, .85)], "back": [(2, .9), (6, .7), (9, .8), (14, .85)],
         "B3": [(2, .7), (10, .7)], "build": [(2, .75), (10, .75)], "C": [(2, .8), (9, .7), (14, .75)],
         "Cbuild": [(2, .8), (9, .7)],
         "peak": [(0, .55), (2, 1.0), (6, .8), (9, .9), (12, .55), (14, .9)],
         "peakbuild": [(2, 1.0), (6, .8)]}


def harmony(mix):
    for i, (start, name) in enumerate(CHANGES):
        end = CHANGES[i + 1][0] if i + 1 < len(CHANGES) else 89.0
        sec = section(start)
        attack, release, cutoff = PADS[sec]
        if start == 0:
            start = 0.5
        if start < 12 <= end:
            end = 11.5
            release = 0.15
        notes = VOICINGS[name][1]
        mix.add("pad", ins.pad(notes, end - start, attack, release, cutoff), start, hall=0.35)
        if sec in ("peak", "peakbuild", "unison"):
            mix.add("pad", ins.pad([m + 12 for m in notes], end - start, attack, release, 5200) * 0.35, start,
                    hall=0.5)

    for bar in range(45):
        for sec_name, pattern in STABS.items():
            for step, vel in pattern:
                t = at(bar, step)
                if section(at(bar, step, False)) != sec_name:
                    continue
                if (sec_name == "build" and t >= 46.0) or (sec_name == "peakbuild" and t >= 77.0):
                    continue
                mix.add("stab", ins.stab(voicing(t), vary(vel, 0.05), 0.16,
                                         1.2 if sec_name.startswith("peak") else 1.0), human(t, 2), plate=0.18, delay=0.1)

    for i, (start, name) in enumerate(CHANGES):
        end = CHANGES[i + 1][0] if i + 1 < len(CHANGES) else 89.0
        if 5.0 <= start < 11.5:
            mix.add("ep", ins.ep(VOICINGS[name][1], min(end, 11.45) - start, 0.45), start, hall=0.25)
        if 80.0 <= start < 88.0:
            mix.add("ep", ins.ep(VOICINGS[name][1], end - start - 0.05, 0.5), start, hall=0.35)
            if end - start >= 2:
                mix.add("ep", ins.ep(VOICINGS[name][1], 0.7, 0.32), start + 1.0 + SWING, hall=0.35)
    for bar in range(13, 21):
        for step, dur, vel in ((0, 3, .55), (6, 2, .42), (11, 3, .48)):
            t = at(bar, step)
            if t >= 41.4:
                continue
            mix.add("ep", ins.ep(voicing(t), dur * STEP * 0.9, vary(vel)), human(t, 3), hall=0.25, delay=0.08)
    mix.add("ep", ins.ep(VOICINGS["Emaj9"][1], 0.95, 0.6), 47.0, hall=0.5)
    mix.add("ep", ins.ep(VOICINGS["C#m9"][1], 1.9, 0.5), 48.0, hall=0.5)
    mix.add("ep", ins.ep(VOICINGS["F#maj9"][1], 1.6, 0.55), 88.0, hall=0.5)
    for t, name, vel in ((5.0, "Emaj9", 1.0), (12.0, "Amaj9", 1.0), (47.0, "Emaj9", 1.0), (78.0, "F#maj9", 1.0)):
        mix.add("stab", ins.stab([m + 12 for m in VOICINGS[name][1]] + list(VOICINGS[name][1]), vel, 0.3, 1.3),
                t, plate=0.3, hall=0.3, delay=0.15)


# Keys -------------------------------------------------------------------

def tones(t, lo, hi):
    pcs = {m % 12 for m in voicing(t)}
    return [m for m in range(lo, hi + 1) if m % 12 in pcs]


def keys(mix):
    for t, m in ((1.25, 80), (1.5, 83), (1.625, 85), (1.875, 88), (3.75, 83), (3.875, 85)):
        mix.add("keys", ins.bubble(m, vary(0.5)), t, p=rng().uniform(-0.6, 0.6), hall=0.4, delay=0.3)
    hint = [(2.0, 88), (2.375, 85), (2.75, 83), (3.0, 85), (3.25, 83), (3.5, 80)]
    for t, m in hint:
        sig = butter(ins.kalimba(m, 0.55), "low", 1700)
        mix.add("keys", sig, t, p=0.15, hall=0.45, delay=0.25)

    shape = [0, 1, 2, 3, 1, 2, 3, 2]
    for k in range(int((10.0 - 5.0) / 0.25)):          # eighth arpeggio, then sixteenths climbing into the drop
        t = 5.0 + k * 0.25
        pool = tones(t, 68, 83)
        m = pool[shape[k % 8] + (2 if k % 8 in (3, 6) else 0)]
        mix.add("keys", ins.kalimba(m, vary(0.62 if k % 2 == 0 else 0.48)), human(t + (SWING if k % 2 else 0), 2),
                p=0.35 * np.sin(k * 0.8), hall=0.25, delay=0.3)
    for k in range(12):
        t = 10.0 + k * STEP
        pool = tones(t, 68, 92)
        mix.add("keys", ins.kalimba(pool[min(k, len(pool) - 1)], 0.4 + 0.04 * k), t, p=0.4 * np.sin(k), hall=0.3)

    for bar in range(13, 21):                          # B: kalimba answers in the vocal's gaps
        for step, idx in ((3, 1), (4, 2), (7, 3), (11, 3), (13, 2)):
            t = at(bar, step)
            if 41.4 <= t:
                continue
            pool = tones(t, 71, 88)
            mix.add("keys", ins.kalimba(pool[idx + 2], vary(0.45)), human(t, 2), p=0.45 if step % 2 else -0.3,
                    hall=0.3, delay=0.35)

    for k in range(24):                                # build: rising glock arpeggio
        t = 44.0 + k * STEP
        pool = tones(t, 66, 94)
        m = pool[min(len(pool) - 1, k // 2 + (k % 2) * 2)]
        mix.add("keys", ins.glock(m, 0.07 + 0.11 * k / 24, 0.5), t, p=0.5 * np.sin(k * 0.7), hall=0.2, delay=0.1)
    for k in range(16):
        t = 47.0 - 16 * 0.0625 + k * 0.0625
        pool = tones(t, 78, 95)
        mix.add("keys", ins.glock(pool[k % len(pool)], 0.05 + 0.07 * k / 16, 0.4), t, p=0.5 * np.cos(k))
    for t, m in ((47.0, 88), (47.0, 92), (47.25, 95), (5.0, 88), (5.0, 95), (78.0, 90), (78.0, 85)):
        mix.add("keys", ins.glock(m, 0.6), t, p=0.3 if m > 90 else -0.3, hall=0.6, delay=0.3)

    shape = [0, 2, 1, 3, 2, 0, 3, 1]                  # C: marimba counter texture
    for bar in range(25, 30):
        for step in range(16):
            t = at(bar, step)
            if t >= 59.0:
                continue
            pool = tones(t, 61, 80)
            mix.add("keys", ins.marimba(pool[shape[step % 8] + (3 if step >= 8 else 0)],
                                        vary(0.5 if step % 4 == 0 else 0.34)), human(t, 2), p=0.35 * np.sin(step),
                    hall=0.15, delay=0.12)

    for bar in range(30, 39):                          # peak sparkle
        for step in (1, 6, 11, 14):
            t = at(bar, step)
            if t >= 77.0:
                continue
            pool = tones(t, 80, 94)
            mix.add("keys", ins.glock(pool[(bar * 3 + step) % len(pool)], vary(0.32), 0.5), human(t, 2),
                    p=0.6 * np.sin(bar + step), hall=0.3, delay=0.45)
    for k, (t, m) in enumerate(((78.5, 78), (78.75, 80), (79.0, 82), (79.25, 85), (79.5, 87), (79.75, 90))):
        mix.add("keys", ins.bubble(m, 0.42 + 0.04 * k), t, p=-0.6 + 0.24 * k, hall=0.4, delay=0.3)

    reprise = [(t + 12, m + 14, d) for t, m, d, _, _ in hook_bar(H1, 34)] + \
              [(t + 12, m + 14, d) for t, m, d, _, _ in hook_bar(H2, 35)] + \
              [(t + 12, m + 14, d) for t, m, d, _, _ in hook_bar(H3, 36)]
    reprise += [(86.0, 87, .3), (86.375, 85, .3), (86.75, 82, .3), (87.0, 83, .4), (87.5, 85, .4), (88.0, 82, 1.0)]
    for t, m, d in reprise:
        mix.add("keys", ins.kalimba(m, vary(0.55), 0.7), human(t, 2), p=0.2, hall=0.4, delay=0.3)
    mix.add("keys", ins.kalimba(90, 0.75, 0.5), 89.0, p=0.1, hall=0.3)
    mix.add("keys", ins.bubble(90, 0.55), 89.0, p=-0.1, hall=0.3)
    mix.add("keys", ins.glock(102 - 12, 0.25, 0.35), 89.0, hall=0.3)


# Melody -----------------------------------------------------------------

H1 = [(0, 76, 3, .95, "a"), (3, 73, 3, .85, "o"), (6, 71, 2, .9, "a"), (8, 73, 2, .75, "e"), (10, 71, 2, .7, "a"),
      (12, 68, 2, .75, "o"), (14, 64, 2, .7, "a")]
H2 = [(2, 66, 2, .8, "a"), (4, 68, 3, .85, "o"), (7, 71, 3, .9, "a"), (10, 69, 2, .75, "e"), (12, 66, 2, .75, "a"),
      (14, 71, 1, .7, "o"), (15, 73, 1, .75, "a")]
H3 = [(0, 75, 3, .95, "a"), (3, 73, 3, .85, "o"), (6, 71, 2, .9, "a"), (8, 73, 2, .75, "e"), (10, 71, 2, .7, "a"),
      (12, 68, 2, .75, "o"), (14, 66, 2, .7, "a")]
H4 = [(2, 64, 2, .8, "a"), (4, 66, 3, .85, "o"), (7, 68, 3, .9, "a"), (10, 71, 2, .8, "e"), (12, 73, 2, .85, "a"),
      (14, 75, 2, .9, "o")]
B1 = [(0, 73, 6, .85, "o"), (6, 71, 2, .7, "a"), (8, 69, 6, .8, "o"), (14, 68, 2, .7, "a")]
B2 = [(0, 66, 6, .8, "u"), (6, 68, 2, .7, "a"), (8, 71, 8, .85, "o")]
B3 = [(0, 73, 6, .85, "o"), (6, 71, 2, .7, "a"), (8, 76, 6, .9, "a"), (14, 75, 2, .75, "e")]
B4a = [(0, 73, 12, .85, "o"), (12, 71, 2, .7, "a"), (14, 69, 2, .7, "e")]
B4b = [(0, 71, 2, .8, "a"), (2, 73, 2, .8, "a"), (4, 76, 2, .85, "o"), (6, 78, 10, .95, "a")]
HOOK = (H1, H2, H3, H4)


def hook_bar(bar_notes, bar, shift=0, staccato=0.86):
    out = []
    for step, m, length, vel, vowel in bar_notes:
        d = length * STEP * (staccato if length <= 3 else 0.95)
        out.append((at(bar, step, swing=False), m + shift, d, vel, vowel))
    return out


def lead_notes():
    notes = [(11.5, 71, 0.21, 0.8, "a"), (11.75, 73, 0.21, 0.85, "a")]
    for i, bar in enumerate(range(6, 13)):
        notes += hook_bar((H1, H2, H3, H4, H1, H2, H4)[i], bar)
    for bar, phrase in zip(range(13, 21), (B1, B2, B3, B4a, B1, B2, B3, B4b)):
        notes += hook_bar(phrase, bar)
    notes += [(43.0, 73, .2, .8, "a"), (43.25, 71, .2, .75, "o"), (43.5, 68, .2, .75, "a"), (43.75, 64, .35, .7, "o")]
    notes += [(47.0, 76, 0.95, .9, "o"), (48.0, 75, 1.25, .8, "u"), (49.5, 71, .2, .8, "a"), (49.75, 73, .2, .85, "a")]
    for i, bar in enumerate(range(25, 29)):
        notes += hook_bar(HOOK[i], bar)
    notes += [(58.0, 69, .22, .8, "a"), (58.5, 73, .22, .85, "o"), (59.0, 71, .22, .85, "a"),
              (59.5, 73, .11, .85, "a"), (59.75, 75, .11, .9, "a")]
    for i, bar in enumerate(range(30, 38)):
        notes += hook_bar(HOOK[i % 4], bar, shift=2)
    notes += [(76.0, 73, .22, .85, "a"), (76.5, 75, .22, .85, "o"), (77.0, 78, .22, .9, "a"),
              (77.5, 80, .11, .9, "a"), (77.75, 82, .11, .95, "a"), (78.0, 85, 1.3, 1.0, "o")]
    return notes


def phrases(notes, gap=0.6):
    notes = sorted(notes)
    groups, cur = [], [notes[0]]
    for n in notes[1:]:
        if n[0] - (cur[-1][0] + cur[-1][2]) > gap:
            groups.append(cur)
            cur = []
        cur.append(n)
    return groups + [cur]


def melody(mix):
    notes = lead_notes()
    for group in phrases(notes):
        t0, y = ins.line(group, "vocal")
        mix.add("lead", y, t0, p=0.0, plate=0.22, hall=0.18, delay=0.22)
    up = [(t, m + 12, d, v, w) for t, m, d, v, w in notes if 12.0 <= t < 26.0 or 50.0 <= t < 60.0]
    peak = [(t, m + 12, d, v, w) for t, m, d, v, w in notes if 60.0 <= t <= 78.0]
    for group, gain in ((up, 0.6), (peak, 1.0)):
        for g in phrases(group):
            t0, y = ins.line(g, "glass", vib=0.12)
            mix.add("glass", y, t0, gain, p=0.0, hall=0.25, delay=0.3)
    for t, m, d, v, _ in notes:
        if 12.0 <= t < 26.0 or 50.0 <= t <= 78.0:
            if v >= 0.85:
                mix.add("keys", ins.gloss(m + 12, v * (0.5 if t < 60 else 0.7)), t, p=0.12, hall=0.2, delay=0.25)
    reprise = [(t + 12, m + 2, d, v * 0.7, w) for t, m, d, v, w in
               hook_bar(H1, 34) + hook_bar(H2, 35) + hook_bar(H3, 36)]
    t0, y = ins.line(reprise, "glass", vib=0.1)
    mix.add("glass", butter(y, "low", 2600), t0, 0.7, hall=0.45, delay=0.3)

    counter = [(50.25, 83, 1.3), (51.25, 80, 0.6), (52.25, 80, 1.3), (53.25, 78, 0.6), (54.25, 78, 1.3),
               (55.25, 75, 0.6), (56.25, 76, 0.9), (57.0, 75, 0.45), (57.5, 73, 0.45), (58.0, 80, 0.9),
               (59.0, 78, 0.9)]
    peak_counter = [(t + 18, m + 2, d) for t, m, d in counter[:9]]
    for t, m, d in counter + peak_counter:
        mix.add("whistle", ins.whistle(m, d, vary(0.8)), t, p=-0.3, hall=0.45, delay=0.25)


# Effects ----------------------------------------------------------------

def effects(mix):
    swell = ins.shimmer([85, 88, 92, 95, 80], 3.2, 16)
    t = secs(len(swell))
    swell *= (np.clip((t - 0.05) / 0.95, 0, 1) ** 2 * np.exp(-np.clip(t - 1.0, 0, None) / 1.2))[:, None]
    mix.add("fx", swell, 0.1, 0.7, hall=0.6)
    mix.add("fx", ins.reverse(ins.glock(88, 0.5, 0.6)), 1.0 - len(ins.glock(88, 0.5, 0.6)) / SR, 0.5, hall=0.5)

    for start, length, notes, gain in ((4.0, 1.0, VOICINGS["B9sus"][1], 0.7), (9.5, 2.0, VOICINGS["B13"][1], 0.9),
                                       (44.0, 3.0, VOICINGS["B9sus"][1], 1.0), (58.5, 1.5, VOICINGS["C#9"][1], 0.85),
                                       (76.0, 2.0, VOICINGS["C#9"][1], 1.0)):
        end = start + length
        if start == 9.5:
            length, end = 2.0, 11.5
        mix.add("fx", ins.riser(length) * 0.5, start, gain, hall=0.3)
        mix.add("fx", ins.tone_riser([m + 12 for m in notes], length) * 0.6, start, gain * 0.8, hall=0.3)
        crash_rev = ins.reverse(ins.crash(0.6, 1.4, 0.5))
        if start != 9.5:
            mix.add("fx", crash_rev, end - len(crash_rev) / SR, gain * 0.7, hall=0.2)

    for t, vel, length in ((5.0, 0.6, 1.6), (12.0, 1.0, 2.8), (26.0, 0.45, 2.0), (42.6, 0.4, 1.6), (47.0, 1.0, 3.5),
                           (50.0, 0.7, 2.4), (60.0, 1.0, 3.0), (68.0, 0.75, 2.4), (76.0, 0.7, 2.0), (78.0, 1.0, 4.0)):
        mix.add("perc", ins.crash(vel, length), t, hall=0.25)
    for t, vel, length in ((5.0, 0.55, 1.4), (12.0, 0.9, 2.0), (47.0, 1.0, 2.6), (60.0, 0.8, 2.0), (78.0, 1.0, 3.0)):
        mix.booms.append((t, 0.5 * vel))
        mix.add("fx", ins.boom(vel, length), t, hall=0.15)
    for t in (5.0, 47.0, 78.0):
        mix.kick(t, 1.0, 1.2)
    mix.add("clap", ins.clap(0.8), 5.0, plate=0.5, hall=0.3)
    for t in (25.0, 49.0, 79.0):
        rev = ins.reverse(ins.crash(0.5, 1.0, 0.45))
        mix.add("fx", rev, t + 1.0 - len(rev) / SR, 0.8, hall=0.2)

    # 42.6: back online; a rising bloop and swoosh, unmuffled.
    n = ns(0.22)
    tt = secs(n)
    f = 260 * (1400 / 260) ** (tt / 0.22) ** 1.4
    up = np.sin(2 * np.pi * np.cumsum(f) / SR) * env(n, 0.01, 0.04)
    mix.add("post", up * 0.6, 42.42, p=0.0, hall=0.3, delay=0.2)
    sw = ins.riser(0.25, 600, 9000, 1.2)
    mix.add("post", sw * 0.6, 42.37, hall=0.2)


def build_music():
    reseed(SEED)
    mix = Mix()
    drums(mix)
    bass(mix)
    harmony(mix)
    keys(mix)
    melody(mix)
    effects(mix)
    stems = stem_report(mix)
    x = master(mix)
    return x, stems
