"""Acoustic-ish voices: nylon, kalimba, upright pizz, brushes, reed, whistle."""
import numpy as np

from dsp import SR, butter, env, eq, hz, noise, ns, pan, pulse, rng, secs, tv

TAU = 2 * np.pi


def _ph(f):
    return TAU * np.cumsum(f) / SR


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
    """A finger's pluck: the string's shape, a little grit."""
    k = max(16, int(SR / f0))
    u = np.arange(k) / k
    shape = np.where(u < position, u / position, (1 - u) / (1 - position))
    x = shape - shape.mean() + 0.25 * noise(k) * np.hanning(k)
    return butter(np.concatenate([x, np.zeros(k)]), "low", cutoff, 2)


# Strings -------------------------------------------------------------------

def nylon(m, vel=0.7, ring=2.4, tone=0.5):
    """Nylon-string guitar pluck: warm flesh attack, wooden body."""
    f = hz(m)
    n = ns(ring + 0.25)
    t = secs(n)
    x = _excite(f, 900 + 3200 * vel * (0.5 + tone), 0.13 + 0.06 * rng().uniform())
    t60 = float(np.clip(ring * (196 / f) ** 0.4, 0.6, 4.5))
    y = ks(f, n, x, t60, 0.46 - 0.16 * tone)
    y /= np.abs(y).max() + 1e-12
    y = eq(eq(y, "peak", 102, 1.6, 4.5), "peak", 215, 1.3, 2.5)
    y = eq(eq(y, "peak", 2600, 0.9, -2.5), "highshelf", 6000, 0.7, -4)
    nail = butter(noise(n), "band", (1500, 5000)) * np.exp(-t / 0.0025) * 0.05 * vel
    return (y + nail) * env(n, 0.0015, 0.18) * vel


def strum(chord, vel=0.7, ring=2.4, spread=0.016, down=True, tone=0.5):
    """Strummed chord; returns stereo, strings fanned across the image."""
    notes = sorted(chord, reverse=not down)
    width = len(notes) - 1 or 1
    voices = [pan(nylon(m, vel * rng().uniform(0.85, 1.0), ring, tone), -0.35 + 0.7 * i / width)
              for i, m in enumerate(sorted(chord))]
    order = [sorted(chord).index(m) for m in notes]
    total = max(len(v) for v in voices) + ns(spread * len(notes) + 0.01)
    out = np.zeros((total, 2))
    for k, idx in enumerate(order):
        i = ns(k * spread * rng().uniform(0.8, 1.2))
        out[i:i + len(voices[idx])] += voices[idx]
    return out / np.sqrt(len(chord))


def pizz(m, vel=0.8, dur=1.0):
    """Upright bass pizzicato: thumb pluck, woody fundamental, muted at dur."""
    f = hz(m)
    ring = min(dur, 1.6)
    n = ns(ring + 0.12)
    t = secs(n)
    y = ks(f, n, _excite(f, 500 + 900 * vel, 0.22), 1.6, 0.5)
    y /= np.abs(y).max() + 1e-12
    fund = np.sin(_ph(f * (1 + 0.012 * np.exp(-t / 0.04)))) * np.exp(-t / 0.55)
    thump = butter(noise(n), "low", 380) * np.exp(-t / 0.011) * 0.5
    y = 0.8 * y + 0.4 * fund + thump
    y = butter(eq(y, "peak", 85, 1.2, 3.0), "low", 2600)
    return np.tanh(1.3 * y) / np.tanh(1.3) * env(n, 0.002, 0.07, hold=ring) * vel


def kalimba(m, vel=0.8, ring=0.7):
    """Kalimba tine: sine with shimmer, struck overtone, box knock."""
    f = hz(m)
    n = ns(ring * 2.8)
    t = secs(n)
    y = np.sin(TAU * f * t) * np.exp(-t / ring)
    y += 0.35 * np.sin(TAU * f * 1.0018 * t + 1.1) * np.exp(-t / (ring * 0.8))
    for r, a, d in ((5.62, 0.24, 0.03), (2.01, 0.06, 0.15), (11.4, 0.05, 0.008)):
        if r * f < 16000:
            y += a * np.sin(TAU * r * f * t + rng().uniform(0, TAU)) * np.exp(-t / d)
    y += butter(noise(n), "band", (300, 1400)) * np.exp(-t / 0.006) * 0.12
    y += butter(noise(n), "band", (2500, 7000)) * np.exp(-t / 0.0015) * 0.06
    y = eq(y, "peak", 380, 1.0, 2.0)
    return y * env(n, 0.001, 0.25) * vel / 1.35


def glock(m, vel=1.0, ring=0.9):
    f = hz(m)
    n = ns(ring * 2.8)
    t = secs(n)
    y = np.sin(TAU * f * t + np.exp(-t / 0.08) * np.sin(TAU * 3.5 * f * t)) * np.exp(-t / ring)
    if 2.76 * f < 17000:
        y += 0.18 * np.sin(TAU * 2.76 * f * t) * np.exp(-t / 0.2)
    return y * env(n, 0.0008, 0.25) * vel


def whistle(m, dur, vel=0.8, scoop=-1.2):
    """Human whistle: breathy sine, scoop into pitch, delayed vibrato."""
    n = ns(dur + 0.25)
    t = secs(n)
    vib = 0.25 * np.sin(TAU * 5.6 * t + rng().uniform(0, TAU)) * np.clip((t - 0.18) / 0.3, 0, 1)
    bend = scoop * np.exp(-t / 0.045)
    f = hz(m + bend + vib)
    ph = _ph(f)
    breath = tv(noise(n), "band", f * 1.02, q=6.0, block=64) * 0.18 + butter(noise(n), "band", (3000, 9000)) * 0.01
    y = np.sin(ph) + 0.04 * np.sin(2 * ph) + breath
    return y * env(n, 0.045, 0.16, hold=dur) * vel


def reed(notes, dur, vel=0.5, attack=0.4, release=0.8, bright=1.0):
    """Reed organ: two beating reeds per note, slow bellows; stereo."""
    n = ns(dur + release)
    t = secs(n)
    bellows = 1 + 0.05 * np.sin(TAU * 0.6 * t + rng().uniform(0, TAU))
    out = np.zeros((n, 2))
    for m in notes:
        for cents, p in ((-3.5, -0.45), (3.5, 0.45)):
            out += pan(pulse(hz(m) * 2 ** (cents / 1200), 0.3, n), p)
    out = butter(out, "low", 1300 * bright, 2)
    out = eq(eq(out, "peak", 850, 1.1, 3.0), "high", 110)
    return out * (bellows * env(n, attack, release, hold=dur))[:, None] * vel / (2 * np.sqrt(len(notes)))


# Kit ---------------------------------------------------------------------

def kick(vel=0.8):
    """Felt-beater kick: round, no click."""
    n = ns(0.45)
    t = secs(n)
    y = np.sin(_ph(47 + 55 * np.exp(-t / 0.028))) * (0.7 * np.exp(-t / 0.22) + 0.3 * np.exp(-t / 0.06))
    y = np.tanh(1.5 * y) / np.tanh(1.5) + butter(noise(n), "low", 260) * np.exp(-t / 0.009) * 0.35
    return y * env(n, 0.0025, 0.05) * vel


def swish(length=0.5, vel=0.4, rise=0.6):
    """Brush swirled across the head; stereo drift."""
    n = ns(length)
    u = secs(n) / length
    shape = np.where(u < rise, np.sin(np.pi / 2 * u / rise) ** 2, np.cos(np.pi / 2 * (u - rise) / (1 - rise)) ** 2)
    out = np.zeros((n, 2))
    for ch in range(2):
        y = tv(noise(n), "band", 2600 + 2400 * np.sin(np.pi * u), q=0.7, block=128)
        y = butter(y, "high", 1500) * (1 + 0.35 * np.sin(TAU * 13 * secs(n) + ch))
        out[:, ch] = y * shape
    return out * vel * 0.5


def slap(vel=0.7):
    """Brush slap on the snare: bristles, wires, a little drum."""
    n = ns(0.4)
    t = secs(n)
    bristle = butter(noise(n), "band", (1200, 9500)) * np.exp(-t / 0.014)
    wires = butter(noise(n), "band", (2500, 7500)) * np.exp(-t / 0.1) * 0.32
    body = np.sin(TAU * 188 * t) * np.exp(-t / 0.04) * 0.3
    y = (bristle + wires + body) * env(n, 0.0018, 0.06)
    return y / np.abs(y).max() * vel


def tap(vel=0.3):
    n = ns(0.12)
    t = secs(n)
    y = butter(noise(n), "band", (2000, 8000)) * np.exp(-t / 0.01) + np.sin(TAU * 190 * t) * np.exp(-t / 0.02) * 0.2
    return y * env(n, 0.001, 0.03) * vel


_METAL = []


def _metal():
    if not _METAL:
        g = np.random.default_rng(116)
        n = ns(4.0)
        y = sum(pulse(f, 0.5, n, start=g.uniform()) for f in (213.0, 297.6, 371.2, 517.9, 563.1, 811.0))
        y = butter(y, "band", (3500, 11000)) + butter(g.standard_normal(n), "band", (5000, 12000)) * 0.4
        _METAL.append(y / np.abs(y).max())
    return _METAL[0]


def ride(vel=0.3, length=1.2):
    """Brush tip on a dark ride."""
    m = _metal()
    n = ns(length)
    t = secs(n)
    off = int(rng().integers(0, len(m) - n))
    y = m[off:off + n] * (0.6 * np.exp(-t / 0.05) + 0.4 * np.exp(-t / 0.5))
    y += sum(a * np.sin(TAU * f * t + rng().uniform(0, TAU)) * np.exp(-t / d)
             for f, a, d in ((3380, 0.12, 0.4), (5120, 0.07, 0.3)))
    return butter(y, "low", 9000) * env(n, 0.0008, 0.2) * vel


def swell(length, vel=0.6):
    """Mallet roll on a cymbal, rising into a downbeat; stereo."""
    m = _metal()
    n = ns(length)
    u = secs(n) / length
    out = np.zeros((n, 2))
    for ch in range(2):
        off = int(rng().integers(0, len(m) - n))
        out[:, ch] = tv(m[off:off + n], "low", 3000 + 7000 * u, block=128) + butter(noise(n), "band", (500, 3000)) * 0.2
    out *= (u ** 2.4 * np.cos(np.clip((u - 0.97) / 0.03, 0, 1) * np.pi / 2))[:, None]
    return out * vel


def crash(vel=0.6, length=3.0):
    """Soft cymbal hit, felt mallet; stereo."""
    m = _metal()
    n = ns(length)
    t = secs(n)
    out = np.zeros((n, 2))
    for ch in range(2):
        off = int(rng().integers(0, len(m) - n))
        out[:, ch] = butter(m[off:off + n], "low", 8000) * (0.4 * np.exp(-t / 0.08) + 0.6 * np.exp(-t / 1.1))
    return out * env(n, 0.004, length * 0.3)[:, None] * vel


def shaker(vel=0.3):
    n = ns(0.1)
    t = secs(n)
    y = butter(noise(n), "band", (4000, 10000)) * np.sin(np.clip(t / 0.014, 0, 1) * np.pi / 2) ** 2 * np.exp(-t / 0.03)
    return y * env(n, 0.001, 0.02) * vel
