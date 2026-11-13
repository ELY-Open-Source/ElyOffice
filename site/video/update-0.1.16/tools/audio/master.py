"""Music bus: sends, sidechain, the ride of levels, loudness."""
import numpy as np

from dsp import (butter, chorus, compress, convolve, db, eq, impulse, limiter, lufs, N, ns, pingpong, secs,
                 true_peak, width)
from grid import blank, CEILING_DB, GAINS, TAIL, TARGET_LUFS


def curve(points):
    ts, vs = np.array(points, float).T
    return lambda t: np.interp(t, ts, vs)


# level ride in dB, hushed before the match
RIDE = curve([(0, -14), (1.9, -14), (2.0, -6), (3.9, -6), (4.0, -4.5), (5.9, -4.5), (6.0, -2.5), (7.9, -1.5),
              (8.0, 0), (41.9, 0), (42.0, -2.5), (45.9, -2.5), (47.9, -0.5), (48.0, 0), (59.9, 0), (60.0, -1.5),
              (61.9, -1.5), (62.0, -0.5), (64.0, 0.5), (81.9, 0.5), (82.0, 0), (85.7, 1.0), (86.0, 1.5),
              (88.0, 0.5), (90, -1)])


# music silent from, to, and its rise back
GATES = ((60.0, 60.75, 0.05), (70.0, 70.98, 0.02), (85.75, 85.96, 0.01))
FADE = (89.3, 89.92)


def gate(t):
    pts = [(0, 1)]
    for a, b, rise in GATES:
        pts += [(a - 0.015, 1), (a, 0), (b, 0), (b + rise, 1)]
    return curve(pts + [(90, 1)])(t)


def pump(events, release=0.16, length=0.5):
    k = ns(length)
    t = secs(k)
    shape = np.minimum(1, t / 0.005) * np.exp(-t / release)
    p = np.zeros(N + TAIL)
    for t0, depth in events:
        i = ns(t0)
        n = min(k, len(p) - i)
        p[i:i + n] = np.maximum(p[i:i + n], depth * shape[:n])
    return p


def stem_report(mix, window=(8.0, 20.0)):
    a, b = ns(window[0]), ns(window[1])
    return {k: round(db(np.sqrt((v[a:b] ** 2).mean())), 1) for k, v in sorted(mix.stems.items())}


def premaster(mix):
    s = {k: mix.stems.get(k, blank()) for k in GAINS}
    p = pump(mix.kicks)

    def duck(depth):
        return (1 - depth * p)[:, None]

    drums = butter(s["kick"], "high", 30) + s["brush"] + butter(s["ride"], "high", 400) + s["perc"]
    drums = np.tanh(drums * 1.2) / 1.2
    bass = compress(butter(s["bass"], "low", 3500).mean(1, keepdims=True) * np.ones((1, 2)), -18, 2.5, 0.01, 0.15)
    nylon = compress(eq(butter(s["nylon"], "high", 75), "peak", 3200, 1.0, -1.5), -20, 2.0, 0.008, 0.15)
    keys = width(butter(s["kalimba"], "high", 150), 1.2, 200) + s["glock"]
    reed = chorus(butter(s["reed"], "high", 120), 0.3, 0.002, 0.012, 0.35)
    lead = eq(s["whistle"], "peak", 2800, 1.0, -2.0)

    room = convolve(mix.sends["room"] * 0.4, impulse(0.9, (0.75, 0.6, 0.35), 116, predelay=0.006, damp=6500))
    hall = convolve(mix.sends["hall"] * 0.3, impulse(2.6, (2.2, 1.9, 1.0), 216, predelay=0.025, damp=6000))
    delay = pingpong(mix.sends["delay"] * 0.3, 0.375, 0.38, lp=3800)
    wet = butter(room + hall + delay, "high", 180)

    x = drums + bass * duck(0.25) + nylon * duck(0.12) + keys + reed * duck(0.3) + lead + s["fx"] + wet * duck(0.2)
    x = butter(x, "high", 25)
    x = eq(x, "lowshelf", 180, 0.7071, 1.0)
    x = eq(x, "peak", 3200, 0.8, -1.5)                 # leave the flame its band
    x = eq(x, "highshelf", 9000, 0.7071, -1.0)
    x = width(x, 1.0, 120)
    x = compress(x, -16, 1.6, 0.02, 0.25, knee=8)
    t = secs(N)
    x = x[:N] * (10 ** (RIDE(t) / 20) * gate(t))[:, None]
    post = s["post"] + butter(convolve(s["post"] * 0.3, impulse(2.6, (2.2, 1.9, 1.0), 216, damp=6000)), "high", 180)
    x += post[:N]
    x *= (np.cos(np.clip((t - FADE[0]) / (FADE[1] - FADE[0]), 0, 1) * np.pi / 2) ** 2)[:, None]
    x[t >= FADE[1]] = 0.0
    return x


def finish(x, target, ceiling, max_tp, drive=0.8):
    """Loudness and true peak met together, or an error."""
    g = target - lufs(x)
    for _ in range(14):
        y = np.tanh(x * 10 ** (g / 20) * drive) / drive
        y = limiter(butter(y, "low", 19000, 8), ceiling)     # nothing above AAC's band
        tp, err = db(true_peak(y)), target - lufs(y)
        if tp <= max_tp and abs(err) < 0.05:
            return y
        if tp > max_tp:
            ceiling -= 0.2
        g += err
    raise ValueError(f"cannot reach {target} LUFS under {max_tp} dBTP: {target - err:.2f} LUFS, {tp:.2f} dBTP")


def master(mix):
    return finish(premaster(mix), TARGET_LUFS, CEILING_DB, -1.0)
