"""Synthesized voices for the 0.1.10 score. Every voice returns a click-free array (mono, or stereo (n, 2))."""
import numpy as np

from dsp import SR, butter, env, eq, hz, noise, ns, pan, pulse, rng, saw, secs, sine, tv

TAU = 2 * np.pi


def _ph(f):
    return TAU * np.cumsum(f) / SR


def _tame(y, drive=1.6):
    """Peak-normalize a noisy one-shot and round off its spikes so its crest factor matches the kit."""
    y = y / np.abs(y).max()
    return np.tanh(drive * y) / np.tanh(drive)


# Drums -------------------------------------------------------------------

def kick(vel=1.0, punch=1.0, length=0.42):
    n = ns(length)
    t = secs(n)
    f = 50 + 120 * np.exp(-t / 0.022) + 70 * np.exp(-t / 0.004)
    body = np.sin(_ph(f)) * (0.65 * np.exp(-t / 0.2) + 0.35 * np.exp(-t / 0.05))
    body = np.tanh(1.8 * punch * body) / np.tanh(1.8 * punch)
    click = butter(noise(n), "band", (1500, 7000)) * np.exp(-t / 0.0025) * 0.5
    return (body + click) * env(n, 0.0006, 0.04) * vel


def clap(vel=1.0):
    n = ns(0.45)
    t = secs(n)
    bursts = sum(np.where(t >= d, np.exp(-(t - d) / 0.0045), 0.0) * (0.75 if i < 3 else 1.0)
                 for i, d in enumerate((0.0, 0.0085, 0.017, 0.027)))
    tail = np.where(t >= 0.027, np.exp(-(t - 0.027) / 0.12), 0.0) * 0.55
    out = np.zeros((n, 2))
    for ch in range(2):
        y = butter(noise(n), "band", (950, 5200)) * (bursts + tail)
        out[:, ch] = eq(eq(y, "peak", 1150, 1.4, 4.0), "peak", 2800, 1.0, -4.0)
    return _tame(out, 2.0) * env(n, 0.0005, 0.05)[:, None] * vel


def snap(vel=1.0):
    n = ns(0.16)
    t = secs(n)
    crack = butter(noise(n), "band", (1800, 7500)) * np.exp(-t / 0.0055)
    body = butter(noise(n), "band", (500, 1400)) * np.exp(-t / 0.018) * 0.45
    ping = np.sin(TAU * 1350 * t) * np.exp(-t / 0.01) * 0.18
    return _tame(eq(crack + body + ping, "peak", 2700, 1.0, -3.0)) * env(n, 0.0004, 0.03) * vel


_METAL = []


def _metal():
    if not _METAL:
        g = np.random.default_rng(77)
        n = ns(3.0)
        y = sum(pulse(f, 0.5, n, start=g.uniform()) for f in (205.3, 304.4, 369.6, 522.7, 540.0, 800.0))
        y = butter(y, "band", (6500, 13000)) + butter(g.standard_normal(n), "high", 8000) * 0.6
        _METAL.append(y / np.abs(y).max())
    return _METAL[0]


def hat(vel=1.0, open_=False):
    m = _metal()
    n = ns(0.32 if open_ else 0.07)
    off = int(rng().integers(0, len(m) - n))
    y = m[off:off + n] * np.exp(-secs(n) / (0.11 if open_ else 0.022))
    return y * env(n, 0.0005, 0.02) * vel


def shaker(vel=1.0):
    n = ns(0.1)
    t = secs(n)
    y = butter(noise(n), "band", (4500, 11000)) * np.sin(np.clip(t / 0.012, 0, 1) * np.pi / 2) ** 2 * np.exp(-t / 0.03)
    return y * env(n, 0.001, 0.02) * vel


def tamb(vel=1.0):
    n = ns(0.22)
    t = secs(n)
    y = butter(noise(n), "band", (6000, 12500)) * np.exp(-t / 0.05)
    for f, d in ((6200, 0.06), (7650, 0.05), (9100, 0.04)):
        y += np.sin(TAU * f * t + rng().uniform(0, TAU)) * np.exp(-t / d) * 0.08
    return y * env(n, 0.0008, 0.03) * vel


def tom(m, vel=1.0):
    """Liquid "bloop" tom: a sine that drops onto its pitch."""
    n = ns(0.38)
    t = secs(n)
    ph = _ph(hz(m) * (1 + 0.8 * np.exp(-t / 0.035)))
    y = np.sin(ph) * np.exp(-t / 0.15) + 0.2 * np.sin(2 * ph) * np.exp(-t / 0.05)
    y = np.tanh(1.4 * y) / np.tanh(1.4) + butter(noise(n), "low", 2500) * np.exp(-t / 0.004) * 0.15
    return y * env(n, 0.0008, 0.05) * vel


def snare(vel=1.0):
    n = ns(0.28)
    t = secs(n)
    tone = np.sin(TAU * 200 * t) * np.exp(-t / 0.035) * 0.55 + np.sin(TAU * 335 * t) * np.exp(-t / 0.022) * 0.3
    nz = butter(noise(n), "band", (1500, 9000)) * (0.75 * np.exp(-t / 0.045) + 0.25 * np.exp(-t / 0.14))
    return _tame(eq(tone + nz, "peak", 2900, 1.0, -4.0)) * env(n, 0.0006, 0.04) * vel


def crash(vel=1.0, length=2.6, tau=0.85):
    n = ns(length)
    t = secs(n)
    out = np.zeros((n, 2))
    for ch in range(2):
        w = noise(n)
        y = butter(w, "band", (3200, 13000)) * 0.8 + butter(w, "band", (600, 3200)) * 0.16
        y = eq(y, "peak", 3000, 0.8, -3.0) * np.exp(-t / tau)
        y += butter(noise(n), "high", 5000) * np.exp(-t / 0.03) * 0.25
        out[:, ch] = y
    return _tame(out, 1.3) * 0.5 * env(n, 0.001, length * 0.3)[:, None] * vel


# Bass --------------------------------------------------------------------

def sub(m, dur, vel=1.0, glide=0.0):
    n = ns(dur + 0.07)
    t = secs(n)
    ph = _ph(hz(m) * 2 ** (glide * np.exp(-t / 0.05) / 12))
    y = np.tanh(1.3 * (np.sin(ph) + 0.15 * np.sin(2 * ph))) / np.tanh(1.3)
    return y * env(n, 0.004, 0.06, hold=dur) * vel


def rubber(m, dur, vel=1.0, glide=0.0, bite=1.0):
    """Rubbery funk bass: saw and square through a resonant envelope filter, over a sine sub."""
    n = ns(dur + 0.08)
    t = secs(n)
    f = hz(m) * 2 ** (glide * np.exp(-t / 0.045) / 12)
    cut = 230 + 2200 * bite * vel * np.exp(-t / 0.075)
    y = tv(saw(f) + 0.5 * pulse(f, 0.5), "low", cut, q=2.6, block=32)
    y = np.tanh(1.6 * (0.55 * y + 0.6 * np.sin(_ph(f)))) / np.tanh(1.6)
    return y * env(n, 0.003, 0.05, hold=dur, decay=0.35, sustain=0.65) * vel


# Chords ------------------------------------------------------------------

def pad(notes, dur, attack=0.5, release=1.2, cutoff=1400.0):
    n = ns(dur + release)
    out = np.zeros((n, 2))
    for m in notes:
        f = hz(m)
        for cents, p in ((-10, -0.85), (0, 0.0), (10, 0.85)):
            out += pan(saw(f * 2 ** (cents / 1200), n), p) * 0.33
        out += pan(sine(f, n, rng().uniform()), 0) * 0.35
    out = butter(out, "low", cutoff)
    return out * env(n, attack, release, hold=dur)[:, None] / np.sqrt(len(notes))


def stab(notes, vel=1.0, length=0.2, bright=1.0):
    n = ns(length + 0.18)
    t = secs(n)
    out = np.zeros((n, 2))
    for m in notes:
        f = hz(m)
        out += pan(saw(f * 2 ** (-7 / 1200), n), -0.55) + pan(saw(f * 2 ** (7 / 1200), n), 0.55)
        out += pan(sine(2 * f, n, rng().uniform()), 0) * 0.3
    out = tv(out, "low", 700 + 5500 * bright * vel * np.exp(-t / 0.07), q=1.1, block=64)
    return out * env(n, 0.002, 0.1, hold=length, decay=0.18, sustain=0.55)[:, None] * vel / len(notes)


def ep(notes, dur, vel=0.7):
    """FM electric piano with a stereo tremolo."""
    n = ns(dur + 0.8)
    t = secs(n)
    trem = 0.12 * np.sin(TAU * 4.6 * t)
    out = np.zeros((n, 2))
    for m in notes:
        f = hz(m)
        idx = (0.6 + 1.6 * vel) * np.exp(-t / 0.3) + 0.25
        y = np.sin(TAU * f * t + idx * np.sin(TAU * f * t))
        if 14 * f < 18000:
            y += np.sin(TAU * 14 * f * t) * np.exp(-t / 0.012) * 0.12 * vel
        y *= np.exp(-t / (1.8 * (262 / f) ** 0.5))
        out[:, 0] += y * (1 + trem)
        out[:, 1] += y * (1 - trem)
    return out * env(n, 0.002, 0.25, hold=dur)[:, None] * vel / np.sqrt(len(notes))


# Mallets and plucks ------------------------------------------------------

def kalimba(m, vel=1.0, ring=0.55):
    f = hz(m)
    n = ns(ring * 2.6)
    t = secs(n)
    y = np.sin(TAU * f * t) * np.exp(-t / ring)
    for r, a, d in ((5.95, 0.22, 0.035), (2.0, 0.08, 0.12), (9.3, 0.05, 0.012)):
        if r * f < 17000:
            y += a * np.sin(TAU * r * f * t + rng().uniform(0, TAU)) * np.exp(-t / d)
    y += butter(noise(n), "band", (1500, 5000)) * np.exp(-t / 0.0018) * 0.08
    return y * env(n, 0.0012, 0.2) * vel


def marimba(m, vel=1.0):
    f = hz(m)
    n = ns(0.9)
    t = secs(n)
    y = np.sin(TAU * f * t) * np.exp(-t / (0.28 * (440 / f) ** 0.3))
    for r, a, d in ((3.93, 0.3, 0.05), (9.2, 0.08, 0.015)):
        if r * f < 17000:
            y += a * np.sin(TAU * r * f * t) * np.exp(-t / d)
    y += butter(noise(n), "low", 800) * np.exp(-t / 0.004) * 0.15
    return y * env(n, 0.001, 0.2) * vel


def bubble(m, vel=1.0):
    """A plink that scoops up onto its pitch, like a bubble surfacing."""
    n = ns(0.55)
    t = secs(n)
    ph = _ph(hz(m) * 2 ** (-9 * np.exp(-t / 0.02) / 12))
    y = np.sin(ph) * np.exp(-t / 0.16) + 0.25 * np.sin(2 * ph) * np.exp(-t / 0.04)
    return y * env(n, 0.001, 0.1) * vel


def glock(m, vel=1.0, ring=0.9):
    f = hz(m)
    n = ns(ring * 2.8)
    t = secs(n)
    y = np.sin(TAU * f * t + np.exp(-t / 0.08) * np.sin(TAU * 3.5 * f * t)) * np.exp(-t / ring)
    if 2.76 * f < 17000:
        y += 0.18 * np.sin(TAU * 2.76 * f * t) * np.exp(-t / 0.2)
    return y * env(n, 0.0008, 0.25) * vel


def gloss(m, vel=1.0):
    """Short glassy FM pluck that sharpens the lead's attacks."""
    f = hz(m)
    n = ns(0.5)
    t = secs(n)
    y = np.sin(TAU * f * t + 2.2 * np.exp(-t / 0.025) * np.sin(TAU * 2 * f * t)) * np.exp(-t / 0.14)
    return y * env(n, 0.001, 0.08) * vel


def whistle(m, dur, vel=1.0):
    n = ns(dur + 0.3)
    t = secs(n)
    vib = 0.22 * np.sin(TAU * 5.3 * t) * np.clip((t - 0.15) / 0.25, 0, 1)
    ph = _ph(hz(m) * 2 ** (vib / 12))
    breath = butter(noise(n), "band", (hz(m) * 1.5, hz(m) * 3)) * 0.04
    return (np.sin(ph) + 0.06 * np.sin(2 * ph) + breath) * env(n, 0.07, 0.25, hold=dur) * vel


# Lead lines --------------------------------------------------------------

VOWELS = {"a": (820, 1200, 2800), "o": (520, 900, 2700), "u": (360, 760, 2600),
          "e": (560, 1650, 2700), "i": (340, 1850, 2900)}
FORMANTS = ((1.0, 5.5), (0.55, 8.0), (0.12, 11.0))


def line(notes, voice="vocal", glide=0.035, vib=0.18):
    """Monophonic line with legato glides and delayed vibrato.

    notes: (start s, midi, duration s, velocity, vowel). "vocal" is a saw and pulse source through
    three morphing formant bandpasses (a chopped vocal); "glass" is a near-pure sine.
    Returns (start time, mono signal).
    """
    notes = sorted(notes)
    t0 = notes[0][0] - 0.02
    n = ns(max(t + d for t, _, d, _, _ in notes) + 0.35 - t0)
    pitch, wob, amp = np.full(n, float(notes[0][1])), np.zeros(n), np.zeros(n)
    form = np.tile(np.array(VOWELS[notes[0][4]], float), (n, 1))
    prev = None
    for t, m, d, v, vowel in notes:
        s, e = ns(t - t0), ns(t + d - t0)
        if prev is not None and t - prev[1] < 0.07 and abs(m - prev[0]) <= 12:
            g = max(2, min(ns(glide), e - s))
            pitch[s:s + g] = prev[0] + (m - prev[0]) * (1 - np.cos(np.linspace(0, np.pi, g))) / 2
            pitch[s + g:] = m
        else:
            pitch[s:] = m
        u = secs(e - s)
        if d > 0.3:
            wob[s:e] += vib * np.sin(TAU * 5.4 * u) * np.clip((u - 0.17) / 0.2, 0, 1)
        k = min(n - s, e - s + ns(0.14))
        a = env(k, 0.005, 0.12, hold=(e - s) / SR, decay=0.22, sustain=0.72) * v
        amp[s:s + k] = np.maximum(amp[s:s + k], a)
        target, onset = np.array(VOWELS[vowel], float), np.array(VOWELS["u"], float)
        form[s:] = target
        q = min(ns(0.05), n - s)
        w = np.linspace(0, 1, q)[:, None]
        form[s:s + q] = onset * (1 - w) + target * w
        prev = (m, t + d)
    f = hz(pitch + wob)
    if voice == "vocal":
        src = 0.75 * saw(f) + 0.35 * pulse(f, 0.28)
        y = sum(g * tv(src, "band", form[:, i], q=q_, block=64) for i, (g, q_) in enumerate(FORMANTS))
        y = y + butter(src, "low", 1500) * 0.22
        y = np.tanh(1.4 * y) / 1.4
    else:
        ph = _ph(f)
        y = np.sin(ph) + 0.12 * np.sin(2 * ph)
    return t0, y * amp


# Effects -------------------------------------------------------------------

def riser(length, f_lo=250.0, f_hi=9000.0, curve=2.2):
    n = ns(length)
    u = secs(n) / length
    cut = f_lo * (f_hi / f_lo) ** (u ** 1.3)
    out = np.stack([tv(noise(n), "band", cut, q=1.4, block=128) for _ in range(2)], 1)
    out *= (u ** curve)[:, None]
    out[-ns(0.015):] *= np.linspace(1, 0, ns(0.015))[:, None]
    return out


def tone_riser(notes, length, climb=12):
    n = ns(length)
    u = secs(n) / length
    out = np.zeros((n, 2))
    for m in notes:
        pitch = m + climb * u ** 1.6
        for cents, p in ((-12, -0.6), (12, 0.6)):
            out += pan(saw(hz(pitch + cents / 100)), p)
    out = tv(out, "low", 300 * 25 ** u, q=1.0, block=128) * (u ** 2)[:, None] / len(notes)
    out[-ns(0.015):] *= np.linspace(1, 0, ns(0.015))[:, None]
    return out


def boom(vel=1.0, length=2.2):
    n = ns(length)
    t = secs(n)
    y = np.tanh(1.4 * np.sin(_ph(36 + 50 * np.exp(-t / 0.09)))) * np.exp(-t / 0.7)
    y += butter(noise(n), "low", 180) * np.exp(-t / 0.04) * 0.6
    return y * env(n, 0.001, 0.4) * vel


def reverse(x, fade=0.01):
    y = np.array(x[::-1], float)
    k = ns(fade)
    ramp = np.linspace(1, 0, k)
    y[-k:] *= ramp[:, None] if y.ndim == 2 else ramp
    return y


def shimmer(notes, length, density=14.0):
    n = ns(length)
    out = np.zeros((n, 2))
    for _ in range(int(density * length)):
        start = rng().uniform(0, max(0.01, length - 0.6))
        m = notes[int(rng().integers(len(notes)))]
        k = ns(0.9)
        t = secs(k)
        g = np.sin(TAU * hz(m) * t + 0.4 * np.sin(TAU * 2 * hz(m) * t)) * np.exp(-t / 0.35)
        g *= env(k, 0.04, 0.2) * rng().uniform(0.4, 1.0)
        i = ns(start)
        j = min(n, i + k)
        out[i:j] += pan(g[:j - i], rng().uniform(-0.85, 0.85))
    return out
