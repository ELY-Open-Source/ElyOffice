"""Strings: bowed section, harp, pizzicato, a warm sub."""
import numpy as np

from dsp import SR, butter, env, eq, hz, noise, ns, pan, rng, saw, secs, tv

TAU = 2 * np.pi


def ks(f0, n, x, t60, damp):
    """Karplus-Strong loop with fractional delay; damp 0.05-0.5."""
    p = SR / f0 - damp
    whole = int(p)
    fr = p - whole
    c0, c1, c2 = (1 - fr) * (1 - damp), fr * (1 - damp) + (1 - fr) * damp, fr * damp
    g = 10 ** (-3 / (f0 * t60))
    pad = whole + 2
    y = np.zeros(pad + n)
    y[pad:pad + len(x)] = x[:n]
    for s in range(pad, pad + n, whole):
        e = min(s + whole, pad + n)
        y[s:e] += g * (c0 * y[s - whole:e - whole] + c1 * y[s - whole - 1:e - whole - 1]
                       + c2 * y[s - whole - 2:e - whole - 2])
    return y[pad:]


def _excite(f0, cutoff, position):
    k = max(16, int(SR / f0))
    u = np.arange(k) / k
    shape = np.where(u < position, u / position, (1 - u) / (1 - position))
    x = shape - shape.mean() + 0.2 * noise(k) * np.hanning(k)
    return butter(np.concatenate([x, np.zeros(k)]), "low", cutoff, 2)


def harp(m, vel=0.6, ring=3.2):
    """Gut string plucked near the middle: round, singing."""
    f = hz(m)
    n = ns(ring + 0.3)
    t = secs(n)
    x = _excite(f, 1200 + 3800 * vel, 0.32 + 0.05 * rng().uniform())
    y = ks(f, n, x, float(np.clip(ring * (262 / f) ** 0.3, 1.0, 5.0)), 0.3)
    y /= np.abs(y).max() + 1e-12
    y = eq(eq(y, "peak", 300, 1.0, 2.0), "highshelf", 5000, 0.7, -5)
    finger = butter(noise(n), "band", (400, 2400)) * np.exp(-t / 0.004) * 0.08 * vel
    return (y + finger) * env(n, 0.002, 0.3) * vel


def pizz(m, vel=0.7, dur=0.8):
    """Section pizzicato: thumb pluck, woody body, short."""
    f = hz(m)
    ring = min(dur, 1.4)
    n = ns(ring + 0.12)
    t = secs(n)
    y = ks(f, n, _excite(f, 600 + 1600 * vel, 0.2), 1.1, 0.45)
    y /= np.abs(y).max() + 1e-12
    body = np.sin(TAU * np.cumsum(f * (1 + 0.01 * np.exp(-t / 0.03))) / SR) * np.exp(-t / 0.35)
    y = eq(0.85 * y + 0.3 * body, "peak", 420, 1.1, 2.5)
    return butter(y, "low", 4200) * env(n, 0.0015, 0.08, hold=ring) * vel


BODIES = {
    "violin": ((450, 1.2, 2.5), (1100, 1.4, 2.0), (2800, 1.2, 3.0), (5200, 1.0, -6.0)),
    "viola": ((330, 1.1, 2.5), (900, 1.3, 2.0), (2200, 1.2, 2.0), (4500, 1.0, -6.0)),
    "cello": ((180, 1.0, 3.0), (520, 1.3, 2.0), (1400, 1.2, 1.5), (3500, 1.0, -7.0)),
}


def bowed(m, dur, vel=0.5, body="viola", attack=0.35, release=0.6, players=4, swell=None, glide=None):
    """Bowed section on one note, with swell and glide."""
    n = ns(dur + release)
    t = secs(n)
    shape = np.ones(n) if swell is None else np.interp(t, *zip(*swell))
    bend = np.zeros(n) if glide is None else np.interp(t, *zip(*glide))
    level = shape * env(n, attack, release, hold=dur)
    out = np.zeros((n, 2))
    for k in range(players):
        rate = rng().uniform(4.8, 6.0)
        depth = rng().uniform(0.07, 0.14) * np.clip((t - 0.25) / 0.5, 0, 1)
        drift = np.cumsum(noise(n)) / SR
        drift = butter(drift - drift.mean(), "low", 1.0) * 6
        cents = rng().uniform(-7, 7) + drift
        f = hz(m + bend + depth * np.sin(TAU * rate * t + rng().uniform(0, TAU)) + cents / 100)
        out += pan(saw(f, n), -0.6 + 1.2 * k / max(1, players - 1))
    f0 = hz(m)
    corner = f0 * 2.5 + 700 + 3800 * vel * level
    out = tv(tv(out, "low", corner, 0.6, block=256), "low", corner * 1.6, 0.6, block=256)
    bow = butter(noise(n), "band", (1500, 4500)) * 0.018 * (1 + 2 * np.exp(-t / 0.08))
    out += pan(bow, 0.0)
    for f, q, g in BODIES[body]:
        out = eq(out, "peak", f, q, g)
    out = butter(out, "high", max(40, f0 * 0.7))
    return out * (level * vel / np.sqrt(players))[:, None]


def sub(m, dur, vel=0.8, release=0.25, slide=None):
    """Warm sub: sine, a little second harmonic, soft attack."""
    n = ns(dur + release)
    t = secs(n)
    semis = np.zeros(n) if slide is None else np.interp(t, *zip(*slide))
    f = hz(m + semis) * (1 + 0.02 * np.exp(-t / 0.03))
    ph = TAU * np.cumsum(f) / SR
    y = np.sin(ph) + 0.18 * np.sin(2 * ph + 0.4) + 0.05 * np.sin(3 * ph)
    y = np.tanh(1.4 * y) / np.tanh(1.4)
    return butter(y, "low", 420) * env(n, 0.012, release, hold=dur) * vel
