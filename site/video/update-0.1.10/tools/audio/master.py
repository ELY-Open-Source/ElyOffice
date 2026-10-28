"""Mastering: ducking, sends, the underwater dip, loudness."""
import numpy as np

from dsp import butter, chorus, compress, convolve, eq, impulse, limiter, lufs, N, ns, pingpong, secs, tv, width
from grid import blank, CEILING_DB, GAINS, TAIL, TARGET_LUFS


def pump_curve(events, release=0.12, length=0.45):
    k = ns(length)
    t = secs(k)
    shape = np.minimum(1, t / 0.004) * np.exp(-t / release)
    p = np.zeros(N + TAIL)
    for t0, depth in events:
        i = ns(t0)
        n = min(k, len(p) - i)
        p[i:i + n] = np.maximum(p[i:i + n], depth * shape[:n])
    return p


def curve(points, log=False):
    ts, vs = np.array(points, float).T
    vs = np.log(vs) if log else vs

    def at_(t):
        v = np.interp(t, ts, vs)
        return np.exp(v) if log else v
    return at_


def muffle(x):
    """41.4-42.6: the music sinks underwater and pops back."""
    a, b = ns(41.2), ns(42.9)
    seg = x[a:b]
    t = 41.2 + secs(len(seg))
    cut = curve([(41.2, 19000), (41.38, 19000), (41.52, 520), (42.5, 520), (42.62, 2400), (42.72, 19000),
                 (42.9, 19000)], log=True)(t)
    wet = tv(tv(seg, "low", cut, q=1.1, block=32), "low", cut, q=0.7, block=32)
    gain = 10 ** (curve([(41.2, 0), (41.38, 0), (41.55, -3.5), (42.5, -3.5), (42.62, 0), (42.9, 0)])(t) / 20)
    fade = np.clip(np.minimum((t - 41.2) / 0.08, (42.9 - t) / 0.08), 0, 1)[:, None]
    y = x.copy()
    y[a:b] = seg * (1 - fade) + wet * gain[:, None] * fade
    return y


def stem_report(mix, window=(12.0, 26.0)):
    """Each stem's RMS and peak in a window, before mastering."""
    a, b = ns(window[0]), ns(window[1])
    return {k: (round(20 * np.log10(np.sqrt((v[a:b] ** 2).mean()) + 1e-12), 1),
                round(20 * np.log10(np.abs(v[a:b]).max() + 1e-12), 1)) for k, v in sorted(mix.stems.items())}


def master(mix):
    return finish(premaster(mix))


def premaster(mix):
    s = {k: mix.stems.get(k, blank()) for k in GAINS}
    p = pump_curve(mix.kicks)
    pb = pump_curve(mix.booms, release=0.45, length=1.6)
    duck = lambda amount: (1 - np.clip(amount * p + pb, 0, 0.85))[:, None]

    pad = width(chorus(eq(butter(s["pad"], "high", 120), "peak", 950, 0.8, -3.0)), 1.5, 180)
    synth = pad * duck(1.0) + butter(s["stab"], "high", 140) * duck(0.75) + s["ep"] * duck(0.5) \
        + width(butter(s["keys"], "high", 180), 1.25, 200) * duck(0.35)
    build_lp = curve([(0, 20000), (43.99, 20000), (44.0, 900), (46.95, 16000), (47.0, 20000), (90, 20000)], log=True)
    synth = tv(synth, "low", build_lp(secs(len(synth))), q=0.9, block=256)

    bass = butter(s["bass"], "low", 5000).mean(1, keepdims=True) * np.ones((1, 2)) * duck(0.8)
    lead = eq(butter(s["lead"], "high", 180), "peak", 3000, 1.0, -2.0)
    lead = compress(lead, -20, 2.5, 0.005, 0.12) * duck(0.12)
    glass = s["glass"] * duck(0.12) + s["whistle"] * duck(0.2)
    drums = s["kick"] + s["clap"] + s["snap"] + s["hats"] + s["perc"] + s["toms"]

    plate = convolve(mix.sends["plate"] * 0.35, impulse(1.6, (1.2, 1.1, 0.7), 21, damp=9000))
    hall = convolve(mix.sends["hall"] * 0.3, impulse(3.4, (3.0, 2.6, 1.4), 31, predelay=0.03, damp=7000))
    delay = pingpong(mix.sends["delay"] * 0.3, 0.375, 0.42)
    wet = butter(plate + hall + convolve(delay * 0.3, impulse(1.6, (1.2, 1.1, 0.7), 21)) + delay, "high", 220)
    wet *= duck(0.6)

    gap = curve([(0, 1), (11.5, 1), (11.53, 0.05), (11.97, 0.05), (12.0, 1), (90, 1)])(secs(N + TAIL))[:, None]
    x = (drums + bass + synth + glass + wet + s["fx"]) * gap + lead
    x = muffle(x) + s["post"]

    x = butter(x, "high", 22)
    x = eq(x, "peak", 2600, 0.9, -1.5)                 # room for the sound effects
    x = eq(x, "highshelf", 7000, 0.7071, 3.0)
    x = width(x, 1.0, 120)                             # mono below 120 Hz
    x = compress(x, -14, 1.7, 0.02, 0.2, knee=8)
    x *= 10 ** (RIDE(secs(len(x))) / 20)[:, None]
    x = x[:N]
    t = secs(N)
    x *= (np.cos(np.clip((t - 89.3) / 0.58, 0, 1) * np.pi / 2) ** 2)[:, None]
    x *= np.minimum(1, t / 0.005)[:, None]
    x[t >= 89.88] = 0.0
    return x


RIDE = curve([(0, -5.0), (4.0, -5.0), (4.95, -2.5), (5.0, -4.0), (9.5, -4.0), (11.5, -3.0), (11.99, -1.0), (12.0, 0.0),
               (25.9, 0.0), (26.0, -2.5), (29.9, -2.5), (30.0, -1.8), (33.9, -1.8), (34.0, -1.0), (43.9, -1.0),
               (46.9, 0.0), (47.0, -0.5), (49.9, -0.8), (50.0, -0.3), (59.9, 0.2), (60.0, 1.4), (77.9, 1.4),
               (78.0, 1.6), (79.9, 0.5), (80.0, -3.5), (88.0, -4.5), (90.0, -5.0)])


def finish(x, target=TARGET_LUFS, ceiling=CEILING_DB):
    g = target - lufs(x)
    for _ in range(6):
        y = x * 10 ** (g / 20)
        y = np.tanh(y * 0.9) / 0.9
        y = limiter(y, ceiling)
        err = target - lufs(y)
        if abs(err) < 0.05:
            break
        g += err
    return y
