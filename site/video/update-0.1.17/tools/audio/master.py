"""Music bus: rooms, sidechain, glue, the ride, loudness."""
import numpy as np

from dsp import (butter, chorus, compress, convolve, db, eq, impulse, limiter, lufs, N, ns, secs, true_peak,
                 width)
from grid import GAINS, TAIL, blank

MUSIC_LUFS = -15.0
CEILING_DB = -1.8
FADE = (119.25, 119.95)

# level ride in dB over the film
RIDE = ((0, -2.0), (7.9, -2.0), (8.0, 0.0), (19.9, 0.0), (20.0, -3.0), (27.9, -2.5), (28.0, -4.0), (29.9, -3.5),
        (30.0, -2.0), (33.9, -0.5), (34.0, -4.0), (35.9, -4.0), (36.0, 0.0), (77.9, 0.5), (78.0, -4.0),
        (85.9, -3.5), (87.9, 0.0), (88.0, 0.8), (105.9, 0.8), (106.0, -5.0), (109.0, -3.0), (111.9, 0.5),
        (112.0, 2.0), (116.0, 1.0), (120, 0.0))


def ride(t):
    ts, vs = zip(*RIDE)
    return np.interp(t, ts, vs)


def pump(events, release=0.18, length=0.55):
    k = ns(length)
    t = secs(k)
    shape = np.minimum(1, t / 0.006) * np.exp(-t / release)
    p = np.zeros(N + TAIL)
    for t0, depth in events:
        i = ns(t0)
        n = min(k, len(p) - i)
        p[i:i + n] = np.maximum(p[i:i + n], min(1.0, depth) * shape[:n])
    return p


def stem_report(mix, window=(46.0, 58.0)):
    """Loudness of each stem over a window, before the bus."""
    a, b = ns(window[0]), ns(window[1])
    return {k: round(float(lufs(v[a:b])), 1) for k, v in sorted(mix.stems.items())}


def spaces(mix):
    room = convolve(mix.sends["room"] * 0.35, impulse(0.8, (0.6, 0.5, 0.3), 117, predelay=0.005, damp=7000))
    plate = convolve(mix.sends["plate"] * 0.3, impulse(1.8, (1.6, 1.5, 1.1), 217, predelay=0.018, damp=9000,
                                                       build=0.004))
    hall = convolve(mix.sends["hall"] * 0.32, impulse(3.6, (3.2, 2.7, 1.5), 317, predelay=0.03, damp=6000))
    return butter(eq(room + plate + hall, "lowshelf", 300, 0.7, -3.0), "high", 140)


def premaster(mix):
    s = {k: mix.stems.get(k, blank()) for k in GAINS}
    p = pump(mix.kicks)

    def duck(depth):
        return (1 - depth * p)[:, None]

    drums = butter(s["kick"], "high", 32) + butter(s["brush"], "high", 300)
    drums = np.tanh(drums * 1.1) / 1.1
    low = compress(butter(s["sub"], "low", 900).mean(1, keepdims=True) * np.ones((1, 2)), -16, 2.5, 0.008, 0.12)
    keys = s["piano"] + s["keys"]
    keys = compress(eq(butter(keys, "high", 55), "peak", 180, 0.8, -1.0), -18, 1.8, 0.01, 0.2, knee=8)
    strings = chorus(butter(s["bowed"], "high", 110), 0.18, 0.0018, 0.013, 0.25) + butter(s["cello"], "high", 50)
    plucks = width(butter(s["harp"] + s["pizz"], "high", 70), 1.15, 200)
    mallets = width(butter(s["marimba"] + s["glint"], "high", 120), 1.1, 200)

    x = (drums + low * duck(0.45) + keys * duck(0.08) + strings * duck(0.22) + plucks + mallets * duck(0.1)
         + s["fx"] + spaces(mix) * duck(0.18))
    x = butter(x, "high", 24)
    x = eq(x, "lowshelf", 120, 0.7071, -0.5)
    x = eq(x, "peak", 2600, 0.7, 1.5)
    x = eq(x, "peak", 380, 0.9, -1.2)
    x = eq(x, "highshelf", 10000, 0.7071, 1.0)
    x = compress(x, -18, 1.8, 0.025, 0.3, knee=10)
    t = secs(len(x))
    x = x * (10 ** (ride(t) / 20))[:, None]
    x = x[:N]
    t = t[:N]
    x *= (np.cos(np.clip((t - FADE[0]) / (FADE[1] - FADE[0]), 0, 1) * np.pi / 2) ** 2)[:, None]
    x[t >= FADE[1]] = 0.0
    return x


def finish(x, target, ceiling, max_tp, drive=0.8):
    """Loudness and true peak met together, or an error."""
    g = target - lufs(x)
    for _ in range(16):
        y = np.tanh(x * 10 ** (g / 20) * drive) / drive
        y = limiter(butter(y, "low", 19000, 8), ceiling)
        tp, err = db(true_peak(y)), target - lufs(y)
        if tp <= max_tp and abs(err) < 0.05:
            return y
        if tp > max_tp:
            ceiling -= 0.2
        g += err
    raise ValueError(f"cannot reach {target} LUFS under {max_tp} dBTP: {target - err:.2f} LUFS, {tp:.2f} dBTP")


def master(mix):
    return finish(premaster(mix), MUSIC_LUFS, CEILING_DB, -1.5)
