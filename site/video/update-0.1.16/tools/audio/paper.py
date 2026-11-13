"""Paper and ink foley: rustle, strokes, tears, landings, splashes, pops."""
import numpy as np

from dsp import SR, butter, edges, env, eq, noise, ns, pan, rng, saw, secs, tv
from fire import at

TAU = 2 * np.pi


def _ph(f):
    return TAU * np.cumsum(f) / SR


def crinkles(n, rate, lo=1800, hi=7000, q=3.0):
    """Crisp paper crackle: resonant ticks of random brightness."""
    out = np.zeros(n + ns(0.01))
    for _ in range(rng().poisson(rate * n / SR)):
        k = ns(rng().uniform(0.002, 0.007))
        t = secs(k)
        f = rng().uniform(lo, hi)
        tick = np.sin(TAU * f * t + rng().uniform(0, TAU)) * np.exp(-t / (0.0006 * q))
        tick += rng().standard_normal(k) * np.exp(-t / 0.0005) * 0.6
        i = int(rng().integers(0, n))
        out[i:i + k] += tick * min(3.0, rng().pareto(2.5) + 0.2)
    return out[:n]


def _motion(n, moves):
    """A hand's gestures: a few overlapping swells."""
    u = np.arange(n) / n
    m = np.zeros(n)
    for _ in range(moves):
        c, w = rng().uniform(0.1, 0.9), rng().uniform(0.08, 0.3)
        m += np.exp(-((u - c) / w) ** 2) * rng().uniform(0.5, 1.0)
    return m / m.max()


def rustle(seconds=0.6):
    n = ns(seconds)
    mot = _motion(n, int(rng().integers(2, 5)))
    out = np.zeros((n, 2))
    for ch in range(2):
        air = tv(noise(n), "band", 1500 + 2500 * mot, q=0.8, block=64) * 0.35
        out[:, ch] = (air + crinkles(n, 900) * 0.35) * mot
    return edges(out, 0.01, 0.05)


def stroke(seconds, soft):
    """Graphite or charcoal on paper: stick-slip with tooth grit."""
    n = ns(seconds)
    u = secs(n) / seconds
    speed = np.sin(np.pi * np.clip(u, 0, 1)) ** 0.7 * (1 + 0.25 * np.sin(TAU * rng().uniform(2, 5) * u))
    rate = (180 + 380 * speed) * rng().uniform(0.8, 1.3)
    ph = np.cumsum(rate) / SR
    slip = np.diff(np.floor(ph), prepend=0) * (0.5 + 0.5 * rng().uniform(size=n))
    slip = butter(slip, "band", (1200, 6500 - 2500 * soft)) * 3.0
    hiss = tv(noise(n), "band", 2600 - 900 * soft + 1400 * speed, q=1.1, block=64) * 0.45
    grit = crinkles(n, 300 + 600 * soft, 1500, 6000 - 2000 * soft) * 0.3
    y = (slip + hiss + grit) * speed
    y = eq(y, "peak", 3800 - 1200 * soft, 1.2, 3.0)
    return pan(edges(y, 0.004, 0.02), rng().uniform(-0.15, 0.15))


def pencil():
    return stroke(rng().uniform(0.12, 0.4), 0.0)


def charcoal():
    return stroke(rng().uniform(0.15, 0.5), 1.0)


def tear(seconds=0.9):
    """Paper torn: fibres snapping under a pull, then the flap."""
    n = ns(seconds)
    u = secs(n) / seconds
    pull = np.clip(u / 0.85, 0, 1) ** 0.6 * np.cos(np.pi / 2 * np.clip((u - 0.85) / 0.15, 0, 1))
    out = np.zeros((n, 2))
    for ch in range(2):
        fibre = np.zeros(n)
        for _ in range(int(900 * seconds)):
            i = int(rng().integers(0, n))
            if rng().uniform() < pull[i]:
                k = ns(0.003)
                fibre[i:i + k] += rng().standard_normal(min(k, n - i)) * np.exp(-secs(min(k, n - i)) / 0.0007)
        body = tv(noise(n), "band", 900 + 1800 * pull, q=1.0, block=64) * 0.4
        out[:, ch] = butter(fibre, "band", (700, 7000)) * 1.5 + body
    out *= pull[:, None]
    return edges(out, 0.005, 0.03)


def fold():
    """A crease pressed, crisp; the sheet settles."""
    total = 0.5
    out = at(rustle(0.35) * 0.6, 0.0, total)
    n = ns(0.08)
    t = secs(n)
    crease = (crinkles(n, 4000, 2500, 8000) + butter(noise(n), "band", (1500, 6000)) * 0.5) * np.exp(-t / 0.02)
    return out + at(pan(crease, 0), 0.18, total)


def fray():
    """A thread fraying: a few fine fibres give."""
    total = 0.7
    n = ns(total)
    y = np.zeros(n)
    t0 = 0.0
    for _ in range(int(rng().integers(5, 9))):
        t0 += rng().uniform(0.03, 0.09)
        k = ns(0.004)
        y += at(rng().standard_normal(k) * np.exp(-secs(k) / 0.0008), t0, total) * rng().uniform(0.4, 1.0)
    return pan(butter(y, "band", (2000, 8000)), rng().uniform(-0.2, 0.2))


def slide(seconds=0.6):
    """Something dragged across the page."""
    n = ns(seconds)
    u = secs(n) / seconds
    v = np.sin(np.pi * u) ** 0.8
    y = tv(noise(n), "band", 900 + 1600 * v, q=1.5, block=64) * v + crinkles(n, 150) * v * 0.2
    return pan(edges(y, 0.01, 0.05), 0) * 0.8


def thud(weight=1.0):
    """Landing: a soft body on paper, a squash."""
    n = ns(0.4)
    t = secs(n)
    f0 = rng().uniform(70, 110) / (0.7 + 0.3 * weight)
    body = np.sin(_ph(f0 * (1 + 0.8 * np.exp(-t / 0.02)))) * np.exp(-t / (0.06 + 0.05 * weight))
    pat = butter(noise(n), "low", 700) * np.exp(-t / 0.015) * 0.6
    slap = butter(noise(n), "band", (1500, 5000)) * np.exp(-t / 0.004) * 0.15
    return pan((body + pat + slap) * env(n, 0.001, 0.06), 0)


def cardboard():
    """A thing tumbles into a cardboard box."""
    n = ns(0.6)
    t = secs(n)
    y = butter(noise(n), "low", 900) * np.exp(-t / 0.02)
    for f, d in ((rng().uniform(140, 180), 0.07), (rng().uniform(260, 330), 0.05), (rng().uniform(520, 640), 0.03)):
        y += np.sin(TAU * f * t + rng().uniform(0, TAU)) * np.exp(-t / d) * 0.5
    y += butter(noise(n), "band", (1200, 4000)) * np.exp(-t / 0.01) * 0.2
    return pan(y * env(n, 0.001, 0.08), rng().uniform(-0.2, 0.2))


def splash():
    """Ink lands: a wet splat and a few droplets."""
    total = 0.9
    n = ns(total)
    t = secs(n)
    out = np.zeros((n, 2))
    for ch in range(2):
        splat = tv(noise(n), "low", 400 + 3500 * np.exp(-t / 0.05), q=1.3, block=64) * np.exp(-t / 0.08)
        out[:, ch] = splat * np.minimum(1, t / 0.002)
    out += pan(np.sin(_ph(90 + 120 * np.exp(-t / 0.02))) * np.exp(-t / 0.05) * 0.5, 0)
    for _ in range(int(rng().integers(5, 10))):
        s = rng().uniform(0.04, 0.45)
        k = ns(0.06)
        tt = secs(k)
        f = rng().uniform(900, 2200)
        drop = np.sin(_ph(f * (1 + 0.5 * (1 - np.exp(-tt / 0.01))))) * np.exp(-tt / 0.015)
        out += at(pan(drop, rng().uniform(-0.7, 0.7)), s, total) * rng().uniform(0.1, 0.3) * np.exp(-s / 0.3)
    return out


def snap():
    """Wooden snap as a gap shuts."""
    n = ns(0.25)
    t = secs(n)
    f = rng().uniform(900, 1300)
    y = np.sin(TAU * f * t) * np.exp(-t / 0.018) * 0.6 + np.sin(TAU * f * 2.7 * t) * np.exp(-t / 0.008) * 0.2
    y += butter(noise(n), "band", (1500, 7000)) * np.exp(-t / 0.002) * 0.8
    y += np.sin(_ph(160 + 80 * np.exp(-t / 0.01))) * np.exp(-t / 0.03) * 0.4
    return pan(y * env(n, 0.0005, 0.05), 0)


def flick():
    """A kick or fling: a flick, then air."""
    total = 0.4
    n = ns(0.05)
    t = secs(n)
    tick = butter(noise(n), "band", (1200, 6000)) * np.exp(-t / 0.004) + np.sin(TAU * 420 * t) * np.exp(-t / 0.01) * 0.4
    m = ns(0.3)
    u = secs(m) / 0.3
    air = tv(noise(m), "band", 600 + 2500 * u, q=1.2, block=64) * np.sin(np.pi * u) ** 2 * 0.35
    return pan(at(tick, 0, total) + at(air, 0.01, total), rng().uniform(-0.2, 0.2))


def stretch():
    """Rubbery stretch up: the flame leaps tall."""
    n = ns(0.45)
    u = secs(n) / 0.45
    f = rng().uniform(170, 230) * 2 ** (1.4 * u ** 1.3)
    src = saw(f) * 0.5 + np.sin(_ph(f))
    y = tv(src, "band", f * 2.5, q=3.0, block=32) + np.sin(_ph(f)) * 0.4
    return pan(butter(y, "low", 3000) * env(n, 0.03, 0.08, decay=0.3, sustain=0.6), 0)


def squash():
    """Shrinking down, a falling bwup."""
    n = ns(0.3)
    u = secs(n) / 0.3
    f = rng().uniform(300, 380) * 2 ** (-1.3 * u)
    y = np.sin(_ph(f)) + 0.3 * tv(saw(f), "band", f * 2, q=3.0, block=32)
    return pan(y * env(n, 0.01, 0.06, decay=0.12, sustain=0.4), 0)


def pop():
    """A soft round pop (a bubble or a cork)."""
    n = ns(0.16)
    t = secs(n)
    f = rng().uniform(380, 640)
    y = np.sin(_ph(f * (1 + 1.2 * (1 - np.exp(-t / 0.008))))) * np.exp(-t / 0.03)
    y += butter(noise(n), "band", (800, 4000)) * np.exp(-t / 0.0015) * 0.35
    return pan(y * env(n, 0.0006, 0.03), 0)


def scribble():
    """A quick burst of strokes, as when the flame draws."""
    total = 0.8
    out = np.zeros((ns(total), 2))
    s = 0.0
    while s < total - 0.2:
        x = stroke(rng().uniform(0.06, 0.16), rng().uniform(0, 1))
        out += at(x, s, total) * rng().uniform(0.6, 1.0)
        s += rng().uniform(0.07, 0.15)
    return out
