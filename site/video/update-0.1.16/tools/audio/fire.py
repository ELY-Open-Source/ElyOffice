"""Fire foley: match, ignition, crackle beds, sparks, roars, snuffs."""
import numpy as np

from dsp import SR, butter, edges, env, noise, ns, pan, rng, secs, tv

TAU = 2 * np.pi


def _ph(f):
    return TAU * np.cumsum(f) / SR


def at(x, start, total):
    """x placed at start in a buffer of total seconds."""
    out = np.zeros((ns(total),) + x.shape[1:])
    i = ns(start)
    k = min(len(x), len(out) - i)
    out[i:i + k] = x[:k]
    return out


def crackles(n, rate, lo=1400, hi=7500, size=1.0, circular=False):
    """Poisson pops: tiny resonant bursts, heavy-tailed sizes."""
    out = np.zeros(n + (0 if circular else ns(0.02)))
    count = rng().poisson(rate * n / SR)
    for _ in range(count):
        k = ns(rng().uniform(0.0006, 0.004))
        f = rng().uniform(lo, hi)
        t = secs(k)
        pop = np.sin(TAU * f * t + rng().uniform(0, TAU)) * np.exp(-t / (k / SR / 3))
        pop += rng().standard_normal(k) * np.exp(-t / (k / SR / 5)) * 0.8
        a = min(4.0, rng().pareto(2.2) + 0.15) * size
        i = int(rng().integers(0, n))
        if circular:
            idx = (i + np.arange(k)) % n
            out[idx] += pop * a
        else:
            out[i:i + k] += pop * a
    if circular:
        return _circ_band(out[:n], 300, 8000)
    return butter(out[:n], "band", (300, 8000), 4)


def _circ_band(y, lo, hi, tilt=0.0):
    """Frequency-domain band filter; loops stay seamless."""
    spec = np.fft.rfft(y)
    f = np.fft.rfftfreq(len(y), 1 / SR)
    spec *= ((f >= lo) & (f <= hi)) * (np.maximum(f, 20) / lo) ** -tilt
    return np.fft.irfft(spec, len(y))


def _circ_noise(n, lo, hi, tilt=0.25):
    y = _circ_band(rng().standard_normal(n), lo, hi, tilt)
    return y / np.sqrt((y ** 2).mean())


def _flicker(n, rate, depth):
    """Slow random flame breathing, looped."""
    k = max(4, int(n / SR * rate))
    pts = rng().uniform(-1, 1, k)
    x = np.interp(np.arange(n) * k / n, np.arange(k + 1), np.append(pts, pts[0]))
    return 1 + depth * x


def crackle_bed(kind, seconds=12.0):
    """Seamless loop; kinds soft, mid, roar."""
    n = ns(seconds)
    rate, breath, roar = {"soft": (6, 0.05, 0.0), "mid": (22, 0.07, 0.1), "roar": (55, 0.1, 0.4)}[kind]
    out = np.zeros((n, 2))
    for ch in range(2):
        fl = _flicker(n, 5.0, 0.5)
        y = _circ_noise(n, 150, 2400, 0.5) * breath * fl
        y += _circ_noise(n, 40, 320) * roar * _flicker(n, 3.0, 0.6)
        y += crackles(n, rate, circular=True) * 0.13
        y += crackles(n, rate * 0.15, 400, 1500, 1.4, circular=True) * 0.1
        out[:, ch] = y
    return out


def spark():
    """A spit from the flame: snap, then a fizz."""
    n = ns(0.35)
    t = secs(n)
    y = crackles(ns(0.02), 900, 2500, 9000, 1.0)
    y = np.concatenate([y, np.zeros(n - len(y))])
    y += butter(noise(n), "band", (3000, 10000)) * np.exp(-t / rng().uniform(0.04, 0.12)) * 0.25
    return pan(y * env(n, 0.0004, 0.05), rng().uniform(-0.3, 0.3))


def scrape(seconds=0.17):
    """Match head dragged on the striker; no flame."""
    n = ns(seconds)
    u = secs(n) / seconds
    grit = crackles(n, 2600, 1500, 8000, 0.6) * (np.sin(np.pi * u) ** 0.6)
    rub = tv(noise(n), "band", 1800 + 2500 * u, q=1.2, block=64) * np.sin(np.pi * u) * 0.8
    return pan(butter((grit + rub) * env(n, 0.004, 0.01), "high", 700), rng().uniform(-0.1, 0.1))


def match_strike():
    """Scratch on the box, flare at 0.16 s, sulphur fizz."""
    total = 1.6
    out = at(scrape(), 0.0, total)
    m = ns(total - 0.16)
    t = secs(m)
    flare = butter(noise(m, 2), "band", (500, 9000)) * (np.exp(-t / 0.09) * np.minimum(1, t / 0.004))[:, None] * 1.6
    fizz = butter(noise(m, 2), "band", (2500, 9000)) * (0.5 * np.exp(-t / 0.5))[:, None]
    foom = np.sin(_ph(70 + 120 * np.exp(-t / 0.03))) * np.exp(-t / 0.12) * 0.7
    burst = np.stack([crackles(m, 140, size=1.0), crackles(m, 140, size=1.0)], 1) * np.exp(-t / 0.25)[:, None]
    out += at(flare + fizz + pan(foom, 0) + burst, 0.16, total)
    return out


def ignite(size=1.0):
    """The flame catches: a soft woof that opens up."""
    total = 0.6 + 0.6 * size
    n = ns(total)
    t = secs(n)
    cut = 180 + (900 + 1500 * size) * np.sin(np.clip(t / 0.25, 0, 1) * np.pi / 2) * np.exp(-t / (0.35 + 0.3 * size))
    out = np.zeros((n, 2))
    for ch in range(2):
        out[:, ch] = tv(noise(n), "low", cut, q=0.9, block=64)
    shape = np.minimum(1, t / (0.04 + 0.03 * size)) ** 1.5 * np.exp(-t / (0.25 + 0.3 * size))
    out *= shape[:, None] * 1.5
    out += pan(np.sin(_ph(55 + 50 * np.exp(-t / 0.05))) * np.exp(-t / 0.15) * 0.6 * size, 0)
    fade = np.minimum(1, t / 0.08) * np.exp(-t / 0.5) * 0.6
    out += np.stack([crackles(n, 60 * size), crackles(n, 60 * size)], 1) * fade[:, None]
    return out


def roar():
    """A thing the flame touches comes alive."""
    total = 1.8
    n = ns(total)
    t = secs(n)
    shape = np.minimum(1, t / 0.12) ** 2 * np.exp(-np.maximum(t - 0.25, 0) / 0.6)
    out = np.zeros((n, 2))
    for ch in range(2):
        body = tv(noise(n), "low", 250 + 1800 * shape, q=0.8, block=64) * 1.4
        out[:, ch] = body * shape * _flicker(n, 9.0, 0.3)
    out += np.stack([crackles(n, 90), crackles(n, 90)], 1) * shape[:, None] * 0.7
    out += pan(np.sin(_ph(48 + 30 * np.exp(-t / 0.1))) * shape * 0.5, 0)
    return out


def catch():
    """A glyph catches: pff, a pop, a short fizz."""
    total = 0.45
    n = ns(total)
    t = secs(n)
    f = rng().uniform(700, 1300)
    puff = butter(noise(n), "band", (300, 2500)) * np.minimum(1, t / 0.02) * np.exp(-t / 0.05) * 0.8
    pop = np.sin(_ph(f * (1 + 0.6 * np.exp(-t / 0.006)))) * np.exp(-np.maximum(t - 0.03, 0) / 0.02) * (t > 0.03) * 0.7
    fizz = butter(noise(n), "band", (2500, 8000)) * np.exp(-np.maximum(t - 0.03, 0) / 0.12) * (t > 0.03) * 0.2
    y = puff + pop + fizz + np.concatenate([np.zeros(ns(0.03)), crackles(n - ns(0.03), 40)]) * 0.5
    return pan(y * env(n, 0.002, 0.05), rng().uniform(-0.2, 0.2))


def sizzle(seconds=1.3):
    """Burning something away: rising fizz and crowded crackle."""
    n = ns(seconds)
    u = secs(n) / seconds
    shape = np.sin(np.pi * np.clip(u * 1.4, 0, 1) / 2) ** 2 * np.cos(np.pi / 2 * np.clip((u - 0.6) / 0.4, 0, 1)) ** 2
    out = np.zeros((n, 2))
    for ch in range(2):
        hiss = tv(noise(n), "band", 2500 + 3000 * u, q=0.9, block=64) * 0.5
        out[:, ch] = (hiss + crackles(n, 150 + 200 * rng().uniform(), size=0.8)) * shape
    return out


def snuff():
    """Puffed out: a breath, then the fizz cut short."""
    total = 0.6
    n = ns(total)
    t = secs(n)
    breath = tv(noise(n), "low", 600 + 2000 * np.exp(-t / 0.06), q=0.8, block=64) * np.exp(-t / 0.09)
    hiss = butter(noise(n), "band", (3000, 8000)) * np.exp(-t / 0.03) * 0.3
    pops = crackles(n, 60) * np.exp(-t / 0.05) * 0.5
    return pan((breath * 1.2 + hiss + pops) * env(n, 0.006, 0.08), 0)


def ember():
    """The last ember winks: a tick, a faint glow."""
    total = 0.9
    n = ns(total)
    t = secs(n)
    f = rng().uniform(2800, 3600)
    glow = np.sin(TAU * f * t) * np.exp(-t / 0.25) * 0.15 + np.sin(TAU * f * 1.5 * t) * np.exp(-t / 0.12) * 0.05
    tick = crackles(ns(0.01), 500, 3000, 9000, 1.0)
    y = glow + np.concatenate([tick, np.zeros(n - len(tick))]) * 0.6
    return pan(y * env(n, 0.001, 0.2), rng().uniform(-0.15, 0.15))


def whoosh(length=0.45):
    """The flame leaps: moving air with a fiery flutter."""
    n = ns(length)
    u = secs(n) / length
    out = np.zeros((n, 2))
    centre = 350 * 6 ** np.sin(np.pi * u ** 0.8)
    for ch in range(2):
        y = tv(noise(n), "band", centre * (1 + 0.04 * ch), q=1.0, block=64)
        out[:, ch] = y * (1 + 0.35 * np.sin(TAU * 23 * secs(n) + ch))
    out *= (np.sin(np.pi * u ** 0.75) ** 2)[:, None]
    lr = np.stack([np.cos(u * np.pi / 2), np.sin(u * np.pi / 2)], 1) * 0.9 + 0.35
    return edges(out * lr, 0.002, 0.02)
