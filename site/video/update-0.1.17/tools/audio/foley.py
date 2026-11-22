"""Foley voices; each returns (stereo signal, hit point in seconds)."""
import numpy as np

from dsp import SR, butter, edges, hz, noise, ns, pan, rng, secs, tv

TAU = 2 * np.pi
PRE = 0.01


def _ring(f, tau, n, phase=0.0):
    t = secs(n)
    return np.sin(TAU * f * t + phase) * np.exp(-t / tau) * np.minimum(1, t / 0.0004)


def _burst(n, lo, hi, tau):
    return butter(noise(n), "band", (lo, hi)) * np.exp(-secs(n) / tau)


def _at(x, t, total):
    out = np.zeros(total)
    i = ns(t)
    k = min(len(x), total - i)
    out[i:i + k] += x[:k]
    return out


def _press(pitch, vel=1.0, n=None):
    """One plastic click: tick, two body modes, a small thock."""
    n = n or ns(0.06)
    y = 0.5 * _burst(n, 3000, 11000, 0.0006)
    y += 0.5 * _ring(2200 * pitch, 0.005, n) + 0.25 * _ring(4300 * pitch, 0.0025, n, 1.0)
    y += 0.35 * _ring(230 * pitch, 0.009, n)
    return y * vel


def click(pitch):
    n = ns(0.2)
    y = _at(_press(pitch), PRE, n) + _at(_press(pitch * 1.12, 0.22), PRE + 0.075, n)
    return pan(edges(y, 0.0005, 0.02), 0.0), PRE


def double(pitch):
    n = ns(0.32)
    y = (_at(_press(pitch), PRE, n) + _at(_press(pitch * 1.1, 0.18), PRE + 0.06, n)
         + _at(_press(pitch * 1.03, 0.92), PRE + 0.125, n) + _at(_press(pitch * 1.12, 0.2), PRE + 0.19, n))
    return pan(edges(y, 0.0005, 0.02), 0.0), PRE


def tick(m):
    """Checkbox: a crisp tk and a tiny tuned tink."""
    n = ns(0.45)
    f = hz(m)
    tink = _ring(f, 0.07, n) + 0.3 * _ring(f * 2.76, 0.02, n, 0.5) + 0.1 * _ring(f * 5.4, 0.006, n, 1.3)
    y = _at(0.7 * _press(1.25, 1.0), PRE, n) + _at(0.38 * tink, PRE + 0.004, n)
    return pan(edges(y, 0.0005, 0.05), 0.0), PRE


def snap(f):
    """Guide snap: a woody tock, a tiny overshoot, settled."""
    n = ns(0.35)

    def tock(g, v):
        k = ns(0.12)
        return v * (_ring(g, 0.028, k) + 0.45 * _ring(g * 2.57, 0.011, k, 0.7) + 0.2 * _ring(g * 4.1, 0.004, k)
                    + 0.6 * _burst(k, 1500, 7000, 0.0008))
    y = _at(tock(f, 1.0), PRE, n) + _at(tock(f * 1.06, 0.3), PRE + 0.042, n) + _at(tock(f * 1.02, 0.08), PRE + 0.07, n)
    y += _at(0.35 * _ring(110, 0.03, ns(0.15)), PRE, n)
    return pan(edges(y, 0.0004, 0.05), 0.0), PRE


def slide(length):
    """A shape dragged: speeds up, glides, settles; grain follows speed."""
    n = ns(length + 0.08)
    u = np.clip(secs(n) / length, 0, 1)
    speed = np.sin(np.pi * u) ** 1.4
    x = noise(n)
    grain = np.abs(butter(noise(n), "low", 60)) * 3
    y = tv(x * (0.6 + 0.6 * grain), "band", 700 + 2600 * speed, q=0.9, block=128) * speed
    y = butter(y, "low", 5200) + 0.3 * butter(noise(n), "band", (150, 600)) * speed
    out = pan(y, 0.0)
    out[:, 0] *= 1 - 0.3 * u
    out[:, 1] *= 0.7 + 0.3 * u
    return edges(out, 0.01, 0.06), 0.0


def key(pitch, space=False):
    """Key cap: scissor click, bottom-out thock, faint return."""
    n = ns(0.3)
    thock = (_ring((420 if not space else 260) * pitch, 0.009 if not space else 0.02, ns(0.1))
             + 0.4 * _ring(1700 * pitch, 0.004, ns(0.1), 0.6))
    y = _at(0.5 * _burst(ns(0.03), 2500, 9000, 0.001), PRE, n) + _at(thock, PRE + 0.008, n)
    if space:
        y += _at(0.15 * _burst(ns(0.06), 3000, 8000, 0.01) * np.sin(TAU * 90 * secs(ns(0.06))), PRE + 0.012, n)
    y += _at(0.25 * _press(pitch * 1.4, 0.6), PRE + 0.095, n)
    return pan(edges(y, 0.0005, 0.04), 0.0), PRE


def peel(length):
    """Tape peeled: stick-slip crackle quickening, then it comes free."""
    n = ns(length + 0.25)
    t = secs(n)
    u = np.clip(t / length, 0, 1)
    rate = 60 + 900 * u ** 1.6
    clicks = np.zeros(n)
    acc = rng().uniform(0, 1)
    for i in range(ns(length)):
        acc += rate[i] / SR
        if acc >= 1:
            acc -= 1
            clicks[i] = rng().uniform(0.3, 1.0) * rng().choice((-1, 1))
    crackle = tv(clicks, "band", 1800 + 3500 * u, q=1.2, block=128)
    hiss = butter(noise(n), "band", (2000, 7000)) * 0.12 * u * (t < length)
    k = ns(0.07)
    thwip = tv(noise(k), "band", np.geomspace(4500, 900, k), q=2.5, block=32) * np.exp(-secs(k) / 0.02)
    free = _at(1.6 * thwip, length, n)
    y = crackle + hiss + free
    return pan(edges(y, 0.02, 0.05), 0.0), length


def pop(f):
    """A round pop: a quick downward pitch, a breath."""
    n = ns(0.2)
    t = secs(n)
    pitch = f * (1 + 0.9 * np.exp(-t / 0.012))
    y = np.sin(TAU * np.cumsum(pitch) / SR) * np.exp(-t / 0.03)
    y += 0.15 * _burst(n, 600, 4000, 0.003)
    return pan(edges(_at(y, PRE, n), 0.0005, 0.03), 0.0), PRE


def whoosh(length, way):
    """Camera flight: air swells to midpoint, then passes."""
    n = ns(length)
    u = secs(n) / length
    shape = np.exp(-((u - 0.5) / 0.2) ** 2)
    out = np.zeros((n, 2))
    for ch in range(2):
        y = tv(noise(n), "band", 350 + 2600 * shape ** 1.5 * (1.0 + 0.1 * ch), q=1.1, block=128)
        y += 0.5 * tv(noise(n), "low", 200 + 500 * shape, block=128)
        out[:, ch] = butter(y, "low", 9000) * shape
    p = way * (2 * u - 1) * 0.7
    th = (np.clip(p, -1, 1) + 1) * np.pi / 4
    out *= np.stack([np.cos(th), np.sin(th)], 1) * np.sqrt(2)
    return edges(out, 0.02, 0.05), length / 2


def lock(pitch):
    """Latch slides, then a firm metallic click with a ring."""
    n = ns(0.5)
    scrape = tv(noise(ns(0.06)), "band", np.linspace(2000, 4500, ns(0.06)), q=2.0) * np.hanning(ns(0.06)) * 0.25
    metal = sum(a * _ring(f * pitch, d, ns(0.4), ph)
                for f, a, d, ph in ((2150, 0.5, 0.06, 0.0), (3420, 0.35, 0.045, 1.0), (5730, 0.25, 0.03, 2.0),
                                    (7810, 0.12, 0.02, 0.5)))
    y = _at(scrape, PRE, n) + _at(metal + 0.8 * _burst(ns(0.4), 2000, 10000, 0.0012), PRE + 0.07, n)
    y += _at(0.5 * _ring(180 * pitch, 0.02, ns(0.15)), PRE + 0.07, n)
    return pan(edges(y, 0.001, 0.05), 0.0), PRE + 0.07


def error(notes):
    """Error blip: two soft tones falling."""
    n = ns(0.5)
    y = np.zeros(n)
    for i, m in enumerate(notes):
        k = ns(0.3)
        t = secs(k)
        f = hz(m)
        tone = np.sin(TAU * f * t) + 0.25 * np.sin(TAU * 2 * f * t) + 0.08 * np.sin(TAU * 3 * f * t)
        tone *= np.minimum(1, t / 0.004) * np.exp(-t / 0.09)
        y += _at(tone * (1.0 if i == 0 else 0.85), PRE + 0.09 * i, n)
    return pan(edges(butter(y, "low", 5000), 0.0005, 0.05), 0.0), PRE


def paper(kind):
    """Paper handled: flick, slide, crinkle or a page turned."""
    length = {"flick": 0.3, "slide": 0.6, "crinkle": 0.8, "turn": 0.5}[kind]
    n = ns(length + 0.1)
    u = np.clip(secs(n) / length, 0, 1)
    if kind == "crinkle":
        env = np.sin(np.pi * u) ** 0.7
        grains = noise(n) * (rng().uniform(size=n) < 0.012)
        y = butter(grains, "band", (1500, 8000)) + 0.25 * butter(noise(n), "band", (2500, 6000))
        y *= env * (0.5 + np.abs(butter(noise(n), "low", 18)) * 3)
    else:
        env = np.sin(np.pi * u ** (0.6 if kind == "turn" else 1.0)) ** 1.5
        corner = 1200 + 4500 * env if kind != "slide" else 900 + 2000 * env
        y = tv(noise(n), "band", corner, q=0.7, block=128) * env
        if kind == "flick":
            y += _at(_burst(ns(0.05), 1500, 9000, 0.004), length * 0.7, n)
    return pan(edges(y, 0.005, 0.05), 0.0), 0.0
