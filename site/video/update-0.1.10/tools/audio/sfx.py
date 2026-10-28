"""One-shots for the cues: rubbery liquid and glossy UI sounds."""
import numpy as np

import instruments as ins
from dsp import (SR, butter, convolve, edges, env, eq, hz, impulse, noise, ns, pan, pulse, reseed, rng, saw,
                 secs, tv, write_wav)

TAU = 2 * np.pi
SEED = 2010
PEAK_DB = -4.0
# per-sound trim so equal cue gains feel equally loud
TRIM = {"bloop": -2, "drip": -3, "splash": 0, "stretch": -5, "snap": -3, "pop": -3, "merge": -3, "whoosh": -4,
        "swoosh-up": -4, "click": -5, "type": -10, "ding": -4, "chime": -4, "tick": -8, "thud": -1, "glass": -5,
        "boing": -4, "shimmer": -6}


def _ph(f):
    return TAU * np.cumsum(f) / SR


def _room(x, amount, seconds=0.9):
    st = x if x.ndim == 2 else pan(x, 0) / np.sqrt(2)
    tail = np.zeros((len(st) + ns(seconds), 2))
    tail[:len(st)] = st
    wet = convolve(tail, impulse(seconds, (0.7, 0.55, 0.3), 5, predelay=0.008, damp=6500))
    tail += butter(wet, "high", 250) * amount
    return tail


def _plink(f0, f1, length, decay):
    n = ns(length)
    t = secs(n)
    f = f0 * (f1 / f0) ** (1 - np.exp(-t / 0.015))
    return np.sin(_ph(f)) * np.exp(-t / decay) * env(n, 0.0008, 0.01)


def _at(x, start, total):
    out = np.zeros(ns(total))
    i = ns(start)
    k = min(len(x), len(out) - i)
    out[i:i + k] = x[:k]
    return out


def bloop():
    n = ns(0.32)
    t = secs(n)
    f = 170 + 520 * (1 - np.exp(-t / 0.035))
    body = np.sin(_ph(f)) * np.exp(-t / 0.09)
    goo = tv(saw(f), "band", f * 1.7, q=4.0, block=32) * np.exp(-t / 0.05) * 0.25
    bub = _at(_plink(900, 1500, 0.12, 0.03), 0.055, 0.32) * 0.18
    return (body + goo + bub) * env(n, 0.002, 0.04)


def drip():
    n = ns(0.24)
    t = secs(n)
    f = 700 * 2 ** (1.6 * (1 - np.exp(-t / 0.018)))
    ph = _ph(f)
    y = np.sin(ph) * np.exp(-t / 0.05) + 0.25 * np.sin(2 * ph) * np.exp(-t / 0.02)
    y += _at(_plink(1700, 2500, 0.1, 0.02), 0.07, 0.24) * 0.15
    return y * env(n, 0.0008, 0.04)


def splash():
    n = ns(1.0)
    t = secs(n)
    thump = np.sin(_ph(55 + 40 * np.exp(-t / 0.04))) * np.exp(-t / 0.12) * 0.9
    out = pan(thump, 0)
    for ch in range(2):
        spray = tv(noise(n), "low", 600 + 4200 * np.exp(-t / 0.12), q=0.9, block=64)
        spray = eq(spray, "peak", 3000, 0.9, -4.0) * np.exp(-t / 0.16) * np.minimum(1, t / 0.003) * 0.7
        out[:, ch] += spray
    for _ in range(14):
        start = rng().uniform(0.05, 0.6)
        f0 = rng().uniform(800, 1900)
        drop = _at(_plink(f0, f0 * rng().uniform(1.3, 1.8), 0.12, rng().uniform(0.02, 0.04)), start, 1.0)
        out += pan(drop * rng().uniform(0.1, 0.25) * np.exp(-start / 0.5), rng().uniform(-0.8, 0.8))
    return _room(out * env(n, 0.001, 0.15)[:, None], 0.25)


def stretch():
    n = ns(0.55)
    t = secs(n)
    u = t / 0.55
    f = 140 * 2 ** (1.3 * u) * (1 + 0.03 * np.sin(TAU * 9 * t))
    src = saw(f) + 0.5 * pulse(f, 0.3)
    y = tv(src, "band", 350 + 450 * u, q=5.0, block=32) + 0.6 * tv(src, "band", 700 + 500 * u, q=7.0, block=32)
    y *= 1 + 0.25 * np.sin(TAU * 24 * t)
    y = butter(y, "low", 3500)
    return y * env(n, 0.05, 0.08, decay=0.4, sustain=0.6) * (0.6 + 0.4 * u)


def snap():
    n = ns(0.22)
    t = secs(n)
    y = np.sin(_ph(260 + 1600 * np.exp(-t / 0.02))) * np.exp(-t / 0.05)
    y += butter(noise(n), "band", (2000, 6000)) * np.exp(-t / 0.0015) * 0.4
    y += _at(_plink(480, 620, 0.15, 0.04), 0.03, 0.22) * 0.3
    return y * env(n, 0.0005, 0.04)


def pop():
    n = ns(0.13)
    t = secs(n)
    y = np.sin(_ph(520 + 900 * (1 - np.exp(-t / 0.006)))) * np.exp(-t / 0.028)
    y += butter(noise(n), "high", 1500) * np.exp(-t / 0.0008) * 0.3
    return y * env(n, 0.0005, 0.03)


def merge():
    n = ns(0.62)
    t = secs(n)
    u = t / 0.62
    f = 210 * 2 ** (-0.9 * u) * (1 + 0.08 * np.sin(TAU * 7 * t) * np.exp(-t / 0.25))
    src = saw(f)
    y = tv(src, "band", 600 - 220 * u, q=5.0, block=32) + 0.6 * tv(src, "band", 1100 - 350 * u, q=7.0, block=32)
    y = y * 0.8 + np.sin(_ph(f)) * 0.5
    y *= 0.8 + 0.2 * np.sin(TAU * 18 * t)
    y += _at(_plink(500, 800, 0.15, 0.04), 0.04, 0.62) * 0.35 + _at(_plink(380, 640, 0.15, 0.05), 0.13, 0.62) * 0.3
    y = butter(y, "low", 4000) * env(n, 0.015, 0.12, decay=0.25, sustain=0.5)
    return np.stack([y, np.roll(y, ns(0.004))], 1)


def whoosh():
    n = ns(0.65)
    t = secs(n)
    u = t / 0.65
    body = 0.7 * noise(n)[:, None] + 0.3 * noise(n, 2)
    centre = 300 * 8 ** np.sin(np.pi * u)
    y = np.stack([tv(body[:, c], "band", centre, q=1.2, block=64) for c in range(2)], 1)
    y = butter(y, "low", 5000) * (np.sin(np.pi * u ** 0.85) ** 2)[:, None]
    y *= np.stack([np.cos(u * np.pi / 2), np.sin(u * np.pi / 2)], 1) * 1.1 + 0.3
    return y


def swoosh_up():
    n = ns(0.5)
    t = secs(n)
    u = t / 0.5
    y = np.stack([tv(noise(n), "band", 300 * (5000 / 300) ** u, q=1.3, block=64) for _ in range(2)], 1)
    y += pan(np.sin(_ph(250 * (1600 / 250) ** (u ** 1.3))) * 0.35, 0)
    y *= (u ** 1.5 * np.cos(np.clip((u - 0.9) / 0.1, 0, 1) * np.pi / 2) ** 2)[:, None]
    return butter(y, "low", 9000)


def click():
    n = ns(0.06)
    t = secs(n)
    y = np.sin(TAU * 1150 * t) * np.exp(-t / 0.005) * 0.5 + np.sin(TAU * 380 * t) * np.exp(-t / 0.012) * 0.7
    y += butter(noise(n), "band", (2000, 6000)) * np.exp(-t / 0.0012) * 0.35
    return y * env(n, 0.0005, 0.01)


def type_():
    n = ns(0.06)
    t = secs(n)
    y = butter(noise(n), "band", (1200, 4500)) * np.exp(-t / 0.0018) * 0.6
    y += np.sin(TAU * 190 * t) * np.exp(-t / 0.009) * 0.5
    y += _at(butter(noise(ns(0.03)), "band", (800, 3000)) * np.exp(-secs(ns(0.03)) / 0.002), 0.011, 0.06) * 0.25
    return butter(y, "low", 6000) * env(n, 0.0004, 0.01)


def _bell(m, ring=0.6):
    f = hz(m)
    n = ns(ring * 3)
    t = secs(n)
    y = np.sin(TAU * f * t + 0.8 * np.exp(-t / 0.12) * np.sin(TAU * 2 * f * t)) * np.exp(-t / ring)
    y += 0.3 * np.sin(TAU * 2 * f * t) * np.exp(-t / 0.3) + 0.08 * np.sin(TAU * 3 * f * t) * np.exp(-t / 0.1)
    return y * env(n, 0.001, 0.2)


def ding():
    a, b = _bell(85, 0.5), _bell(90, 0.6)
    y = np.zeros(len(b) + ns(0.12))
    y[:len(a)] += a * 0.8
    y[ns(0.12):] += b
    return _room(pan(y, 0) * 0.7, 0.3)


def chime():
    total = 1.4
    out = np.zeros((ns(total), 2))
    for i, (m, p) in enumerate(((80, -0.4), (85, 0.0), (90, 0.4))):
        out += pan(_at(ins.glock(m, 1.0, 0.45), i * 0.065, total), p) * (0.7 + 0.1 * i)
    return _room(out, 0.3)


def tick():
    n = ns(0.045)
    t = secs(n)
    y = np.sin(TAU * 2200 * t) * np.exp(-t / 0.006) + np.sin(TAU * 3300 * t) * np.exp(-t / 0.004) * 0.3
    y += butter(noise(n), "high", 3000) * np.exp(-t / 0.0006) * 0.2
    return y * env(n, 0.0004, 0.01)


def thud():
    n = ns(0.42)
    t = secs(n)
    y = np.tanh(1.5 * np.sin(_ph(55 + 70 * np.exp(-t / 0.03)))) * np.exp(-t / 0.12)
    y += butter(noise(n), "low", 300) * np.exp(-t / 0.02) * 0.5
    y += tv(saw(110 + 0 * t), "band", 250 + 350 * np.exp(-t / 0.05), q=3.0, block=32) * np.exp(-t / 0.06) * 0.12
    return y * env(n, 0.001, 0.08)


def glass():
    total = 1.0
    out = np.zeros(ns(total))
    for delay, scale, detune in ((0.0, 1.0, 1.0), (0.018, 0.55, 1.012)):
        n = ns(total - delay)
        t = secs(n)
        y = sum(a * np.sin(TAU * f * detune * t + rng().uniform(0, TAU)) * np.exp(-t / d)
                for f, a, d in ((2350, 1.0, 0.55), (3710, 0.45, 0.35), (5230, 0.22, 0.2), (7080, 0.1, 0.12)))
        y += butter(noise(n), "high", 2500) * np.exp(-t / 0.001) * 0.2
        out += _at(y * env(n, 0.0005, 0.1) * scale, delay, total)
    return _room(pan(butter(out, "low", 9000), 0.1) * 0.7, 0.35)


def boing():
    n = ns(0.7)
    t = secs(n)
    wob = np.exp(-t / 0.22) * np.sin(TAU * 16 * t)
    f = 180 * 2 ** (0.9 * (1 - np.exp(-t / 0.05)) + 0.35 * wob)
    y = tv(saw(f) * 0.6 + np.sin(_ph(f)) * 0.8, "low", f * 4, q=3.0, block=32)
    return y * env(n, 0.002, 0.08, decay=0.3, sustain=0.3)


def shimmer():
    total = 1.0
    out = np.zeros((ns(total), 2))
    for i in range(18):
        start = 0.03 + i * 0.04 + rng().uniform(0, 0.02)
        m = (78, 80, 83, 85, 90, 92, 95)[int(rng().integers(7))]
        g = _at(ins.glock(m, rng().uniform(0.3, 0.7), 0.25), start, total) * np.exp(-start / 0.6)
        out += pan(g, rng().uniform(-0.8, 0.8))
    n = ns(total)
    air = butter(noise(n, 2), "band", (7000, 12000)) * (np.sin(np.pi * secs(n) / total) ** 2)[:, None] * 0.05
    return _room(out + air, 0.4, 1.2)


SOUNDS = {"bloop": bloop, "drip": drip, "splash": splash, "stretch": stretch, "snap": snap, "pop": pop,
          "merge": merge, "whoosh": whoosh, "swoosh-up": swoosh_up, "click": click, "type": type_, "ding": ding,
          "chime": chime, "tick": tick, "thud": thud, "glass": glass, "boing": boing, "shimmer": shimmer}


def render(name):
    x = SOUNDS[name]()
    x = edges(butter(x, "high", 30), 0.0005, 0.01)
    return x * 10 ** ((PEAK_DB + TRIM[name]) / 20) / np.abs(x).max()


def build_sfx(out_dir):
    reseed(SEED)
    for name in SOUNDS:
        write_wav(out_dir / f"{name}.wav", render(name))
    return list(SOUNDS)
