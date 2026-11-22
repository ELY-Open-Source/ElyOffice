"""Marimba, a soft felt kick and brushes."""
import numpy as np

from dsp import SR, butter, env, eq, hz, noise, ns, rng, secs, tv

TAU = 2 * np.pi


def marimba(m, vel=0.6, hard=0.4):
    """Rosewood bar over a tube: tuned modes, yarn mallet."""
    f = hz(m)
    tau = float(np.clip(0.85 * (220 / f) ** 0.55, 0.22, 1.7))
    n = ns(tau * 5 + 0.05)
    t = secs(n)
    y = np.zeros(n)
    for ratio, amp, life in ((1.0, 1.0, 1.0), (3.98, 0.32 * (0.4 + hard), 0.22), (9.86, 0.12 * hard, 0.07),
                             (2.71, 0.03, 0.12)):
        if ratio * f < 15000:
            y += amp * np.sin(TAU * ratio * f * t + rng().uniform(0, TAU)) * np.exp(-t / (tau * life))
    bloom = 1 - 0.25 * np.exp(-t / 0.012)
    y *= bloom
    contact = butter(noise(n), "low", 900 + 3500 * hard) * np.exp(-t / (0.002 + 0.002 * (1 - hard)))
    y += 0.12 * contact
    y = eq(y, "peak", 2 * f, 2.0, 1.5)
    return y * env(n, 0.0012 + 0.0015 * (1 - hard), 0.05) * vel


def kick(vel=0.7):
    """Felt beater on a loose head: round, no click."""
    n = ns(0.5)
    t = secs(n)
    f = 46 + 48 * np.exp(-t / 0.032)
    y = np.sin(TAU * np.cumsum(f) / SR) * (0.65 * np.exp(-t / 0.24) + 0.35 * np.exp(-t / 0.07))
    y = np.tanh(1.6 * y) / np.tanh(1.6)
    felt = butter(noise(n), "band", (90, 700)) * np.exp(-t / 0.008) * 0.25
    return butter(y + felt, "low", 2400) * env(n, 0.003, 0.08) * vel


def swish(length=0.5, vel=0.4, rise=0.6):
    """Brush swirled across the head; stereo drift."""
    n = ns(length)
    u = secs(n) / length
    shape = np.where(u < rise, np.sin(np.pi / 2 * u / rise) ** 2, np.cos(np.pi / 2 * (u - rise) / (1 - rise)) ** 2)
    out = np.zeros((n, 2))
    for ch in range(2):
        y = tv(noise(n), "band", 2400 + 2200 * np.sin(np.pi * u + ch), q=0.7, block=128)
        y = butter(y, "high", 1300) * (1 + 0.3 * np.sin(TAU * 11 * secs(n) + 2 * ch))
        out[:, ch] = y * shape
    return out * vel * 0.5


def slap(vel=0.6):
    """Brush slap on the snare: bristles, wires, a little drum."""
    n = ns(0.45)
    t = secs(n)
    bristle = butter(noise(n), "band", (1500, 9000)) * np.exp(-t / 0.016)
    wires = butter(noise(n), "band", (2500, 7000)) * np.exp(-t / 0.11) * 0.3
    body = np.sin(TAU * 196 * t) * np.exp(-t / 0.045) * 0.28
    y = (bristle + wires + body) * env(n, 0.0025, 0.08)
    return y / np.abs(y).max() * vel


def tap(vel=0.3):
    """Brush tip tap: the sixteenth texture."""
    n = ns(0.1)
    t = secs(n)
    y = butter(noise(n), "band", (2500, 8500)) * np.exp(-t / 0.009)
    return y * env(n, 0.0012, 0.03) * vel


def roll(length, vel=0.5):
    """Brush swell rising into a downbeat; stereo."""
    n = ns(length)
    u = secs(n) / length
    out = np.zeros((n, 2))
    for ch in range(2):
        y = tv(noise(n), "band", 1500 + 3500 * u ** 1.5, q=0.9, block=128)
        grain = 1 + 0.5 * np.sin(TAU * (9 + 7 * u) * secs(n) + ch)
        out[:, ch] = butter(butter(y, "high", 900), "low", 6500) * grain
    fade = u ** 2.2 * np.cos(np.clip((u - 0.96) / 0.04, 0, 1) * np.pi / 2)
    return out * fade[:, None] * vel
