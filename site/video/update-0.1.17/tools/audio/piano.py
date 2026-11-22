"""Felt piano: stiff-string partials, felt hammer, beating unisons, mechanics."""
import numpy as np

from dsp import SR, butter, eq, hz, noise, ns, pan, rng, secs

TAU = 2 * np.pi
LAYERS = (0.18, 0.3, 0.42, 0.55, 0.68, 0.82, 0.95)
_CACHE = {}


def _ring(m):
    """Seconds a note rings with the pedal down."""
    return float(np.clip(9.0 - (m - 36) * 0.11, 2.6, 9.0))


def _render(m, vel):
    f0 = hz(m)
    n = ns(_ring(m))
    t = secs(n).astype(np.float32)
    g = np.random.default_rng(int(m * 100 + vel * 1000))
    stiff = 1.4e-4 * 2 ** ((m - 48) / 14)
    cutoff = float(np.clip(f0 * (2.2 + 7.5 * vel ** 1.6), 500, 5200))
    base = float(np.clip(7.5 * (196 / f0) ** 0.55, 1.4, 11.0))
    y = np.zeros(n, np.float32)
    for k in range(1, 64):
        fk = k * f0 * np.sqrt(1 + stiff * k * k)
        if fk > min(11000, 5.5 * cutoff):
            break
        felt = 1 / (1 + (fk / cutoff) ** 4)
        strike = abs(np.sin(np.pi * k * 0.118)) + 0.08
        amp = felt * strike / k ** 0.75
        tau = base / (1 + 0.35 * (k - 1) ** 0.8 + fk / 2400)
        beat = fk * 0.0005 * g.uniform(0.6, 1.4)
        e = 0.72 * np.exp(-t / (0.3 * tau)) + 0.28 * np.exp(-t / tau)
        e *= 0.82 + 0.18 * np.cos(TAU * beat * t + g.uniform(0, TAU))
        y += (amp * e * np.sin(TAU * fk * t + g.uniform(0, TAU))).astype(np.float32)
    y /= np.abs(y[: ns(0.05)]).max() + 1e-9
    thump_n = ns(0.06)
    thump = butter(g.standard_normal(thump_n), "band", (60, 320)) * np.exp(-secs(thump_n) / 0.012)
    felt_hit = butter(g.standard_normal(thump_n), "band", (900, 3200)) * np.exp(-secs(thump_n) / 0.004)
    y[:thump_n] += (0.16 * thump + 0.05 * vel * felt_hit).astype(np.float32)
    y = eq(eq(y, "peak", 240, 0.9, 1.5), "peak", 3200, 1.0, -2.5)
    up = ns(0.0025 + 0.004 * (1 - vel))
    y[:up] *= np.sin(np.linspace(0, np.pi / 2, up)) ** 2
    return y


def _layer(m, vel):
    i = int(np.argmin([abs(v - vel) for v in LAYERS]))
    key = (m, i)
    if key not in _CACHE:
        _CACHE[key] = _render(m, LAYERS[i])
    return _CACHE[key], LAYERS[i]


def note(m, vel, dur):
    """One key, held dur seconds, then a soft felt damper."""
    y, layer = _layer(int(m), vel)
    rel = float(np.clip(0.42 - (m - 40) * 0.004, 0.16, 0.42))
    n = min(len(y), ns(dur + rel))
    out = y[:n].astype(float) * (vel / layer) ** 0.6 * vel ** 1.5
    t = secs(n)
    r = np.clip((t - dur) / rel, 0, 1)
    out *= np.cos(r * np.pi / 2) ** 2
    if dur + rel < len(y) / SR:
        k = ns(0.05)
        i = max(0, min(n - k, ns(dur)))
        damp = butter(noise(k), "band", (120, 900)) * np.exp(-secs(k) / 0.015) * 0.012 * vel
        out[i:i + k] += damp[:n - i]
    return out


def chord(notes, vel, dur, spread=0.012, p=0.0):
    """Notes rolled bottom up, spread across the stereo field."""
    notes = sorted(notes)
    k = len(notes)
    tail = ns(spread * k + 0.01)
    parts = [(i, note(m, vel * rng().uniform(0.86, 1.0) * (1.06 if i == k - 1 else 1), dur)) for i, m in
             enumerate(notes)]
    out = np.zeros((max(len(v) for _, v in parts) + tail, 2))
    for i, v in parts:
        s = ns(i * spread * rng().uniform(0.7, 1.3))
        out[s:s + len(v)] += pan(v, p + 0.5 * (i / max(1, k - 1) - 0.5))
    return out / np.sqrt(k) * 1.2
