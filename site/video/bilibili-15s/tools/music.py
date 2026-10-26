# Usage: python tools/music.py (numpy). 120 BPM, 30 beats = 15 s.
import wave
from pathlib import Path

import numpy as np

SR = 48000
BEAT = 0.5
N = round(15.0 * SR)
OUT = Path(__file__).parent.parent / "assets" / "audio" / "music.wav"
TARGET_LUFS = -16.0
CEILING = 10 ** (-1.0 / 20)

rng = np.random.default_rng(1022)
dry = np.zeros((N, 2))
send = np.zeros((N, 2))

# (bass root, pad voicing, arpeggio tones): D A Bm G
CHORDS = [
    (38, [50, 57, 62, 66, 69], [62, 66, 69, 74]),
    (45, [49, 57, 61, 64, 69], [61, 64, 69, 73]),
    (47, [47, 59, 62, 66, 71], [62, 66, 71, 74]),
    (43, [43, 59, 62, 67, 71], [62, 67, 71, 74]),
]
ARP = [0, 1, 2, 3, 2, 1, 2, 3]
# Morph landings in beats (one per app, then suite and star).
LANDINGS = [4, 6, 8, 10, 12, 14, 16, 18, 24]
HOOK = [(0, 74, 1.5), (1.5, 76, 0.5), (2, 78, 1.0), (3, 81, 1.0)]
TOP = [(18, 78, 1), (19, 81, 1), (20, 83, 1.5), (21.5, 81, 0.5), (22, 78, 2)]


def at(beat):
    return round(beat * BEAT * SR)


def hz(m):
    return 440.0 * 2 ** ((m - 69) / 12)


def times(sec):
    return np.arange(int(sec * SR)) / SR


def attack(sig, sec):
    a = max(1, int(sec * SR))
    sig[:a] *= np.linspace(0.0, 1.0, a)
    return sig


def tail(sig, sec=0.02):
    r = int(sec * SR)
    ramp = np.linspace(1.0, 0.0, r)
    sig[-r:] *= ramp if sig.ndim == 1 else ramp[:, None]
    return sig


def put(sig, beat, gain=1.0, pan=0.0, rev=0.2):
    i = at(beat)
    if sig.ndim == 1:
        th = (pan + 1) * np.pi / 4
        sig = np.stack([sig * np.cos(th), sig * np.sin(th)], 1) * np.sqrt(2)
    n = min(len(sig), N - i)
    dry[i:i + n] += sig[:n] * gain
    send[i:i + n] += sig[:n] * gain * rev


def band_noise(n, lo, hi):
    spec = np.fft.rfft(rng.standard_normal(n))
    f = np.fft.rfftfreq(n, 1 / SR)
    spec *= (f > lo) * (f < hi)
    out = np.fft.irfft(spec, n)
    return out / np.abs(out).max()


def pluck(m, dur=1.0, tau=0.45, partials=8):
    t = times(dur)
    s = np.zeros_like(t)
    for h in range(1, partials + 1):
        if hz(m) * h > 15000:
            break
        s += np.sin(2 * np.pi * hz(m) * h * t) * np.exp(-t * (1 + 0.8 * (h - 1)) / tau) / h ** 1.4
    return tail(attack(s * 0.5, 0.002))


def mallet(m, dur=1.6, tau=0.7):
    t = times(dur)
    f = hz(m)
    s = (np.sin(2 * np.pi * f * t) * np.exp(-t / tau)
         + 0.25 * np.sin(2 * np.pi * 2 * f * t) * np.exp(-t / (tau / 3))
         + 0.12 * np.sin(2 * np.pi * 4.0 * f * t) * np.exp(-t / (tau / 8)))
    return tail(attack(s * 0.5, 0.003))


def tick(level):
    t = times(0.08)
    s = (np.sin(2 * np.pi * 1750 * t) * np.exp(-t / 0.009)
         + 0.5 * np.sin(2 * np.pi * 2950 * t) * np.exp(-t / 0.005)
         + 0.3 * band_noise(len(t), 2000, 8000) * np.exp(-t / 0.0015))
    return tail(attack(s * level, 0.0003), 0.01)


def kick(level=1.0, decay=0.22):
    t = times(0.7)
    f = 46 + 70 * np.exp(-t / 0.032)
    s = np.sin(2 * np.pi * np.cumsum(f) / SR) * np.exp(-t / decay)
    return tail(attack(s * level, 0.0015))


def snap():
    t = times(0.3)
    e = sum(np.exp(-np.clip(t - d, 0, None) / 0.004) * (t >= d) for d in (0.0, 0.008, 0.016))
    e = e + 0.45 * np.exp(-t / 0.055)
    s = band_noise(len(t), 900, 5000) * e
    return tail(attack(s * 0.5, 0.0004))


def shaker():
    t = times(0.09)
    s = band_noise(len(t), 5000, 12000) * np.exp(-t / 0.022)
    return tail(attack(s * 0.3, 0.003))


def bass(m, dur):
    t = times(dur)
    w = 2 * np.pi * hz(m) * t
    s = (np.sin(w) + 0.28 * np.sin(2 * w) + 0.08 * np.sin(3 * w)) * np.exp(-t / 0.9)
    return tail(attack(np.tanh(1.4 * s) / np.tanh(1.4), 0.004), 0.04)


def pad(voices, sec, rise, release=0.6, decay=None):
    t = times(sec + release)
    out = np.zeros((len(t), 2))
    for m in voices:
        for side, cents in ((0, -6), (1, 6)):
            f = hz(m) * 2 ** (cents / 1200)
            ph = rng.uniform(0, 2 * np.pi, 4)
            for h, a in enumerate((1.0, 0.4, 0.18, 0.07), 1):
                out[:, side] += a * np.sin(2 * np.pi * f * h * t + ph[h - 1])
    env = np.ones(len(t))
    r = int(rise * SR)
    env[:r] = np.sin(np.linspace(0, np.pi / 2, r)) ** 2
    if decay:
        env *= np.exp(-t / decay)
    else:
        k = int(sec * SR)
        env[k:] *= np.cos(np.linspace(0, np.pi / 2, len(t) - k)) ** 2
    return tail(out * env[:, None] / len(voices) * 0.35, 0.05)


def reverb(x):
    n_ir = int(2.4 * SR)
    t = np.arange(n_ir) / SR
    f = np.fft.rfftfreq(n_ir, 1 / SR)
    nfft = 1 << int(np.ceil(np.log2(N + n_ir)))
    out = np.zeros_like(x)
    for c in range(2):
        ir = np.fft.irfft(np.fft.rfft(rng.standard_normal(n_ir)) / (1 + (f / 5000) ** 2), n_ir)
        ir *= np.exp(-t / 0.32)
        ir = np.concatenate([np.zeros(int(0.015 * SR)), ir])[:n_ir]
        ir /= np.sqrt(np.sum(ir ** 2))
        out[:, c] = np.fft.irfft(np.fft.rfft(x[:, c], nfft) * np.fft.rfft(ir, nfft), nfft)[:N]
    return out


def biquad(f, b, a):
    z = np.exp(-2j * np.pi * f / SR)
    return (b[0] + b[1] * z + b[2] * z ** 2) / (a[0] + a[1] * z + a[2] * z ** 2)


def lufs(x):
    f = np.fft.rfftfreq(len(x), 1 / SR)
    k = (biquad(f, (1.53512485958697, -2.69169618940638, 1.19839281085285),
                (1.0, -1.69065929318241, 0.73248077421585))
         * biquad(f, (1.0, -2.0, 1.0), (1.0, -1.99004745483398, 0.99007225036621)))
    y = np.fft.irfft(np.fft.rfft(x, axis=0) * k[:, None], len(x), axis=0)
    c = np.concatenate([[0.0], np.cumsum(np.sum(y ** 2, 1))])
    starts = np.arange(0, len(x) - 19200 + 1, 4800)
    p = (c[starts + 19200] - c[starts]) / 19200
    p = p[-0.691 + 10 * np.log10(p + 1e-20) > -70]
    p = p[-0.691 + 10 * np.log10(p) > -0.691 + 10 * np.log10(p.mean()) - 10]
    return -0.691 + 10 * np.log10(p.mean())


def run_min(x, w):
    n = len(x)
    xp = np.concatenate([x, np.ones(-(-(n + w) // w) * w - n)]).reshape(-1, w)
    pre = np.minimum.accumulate(xp, 1).ravel()
    suf = np.minimum.accumulate(xp[:, ::-1], 1)[:, ::-1].ravel()
    return np.minimum(suf[:n], pre[w - 1:w - 1 + n])


def limit(x, half=480):
    need = np.minimum(1.0, CEILING / np.maximum(np.abs(x).max(1), 1e-12))
    hold = run_min(np.concatenate([np.ones(half), need, np.ones(half)]), 2 * half + 1)
    h = half // 2
    c = np.concatenate([[0.0], np.cumsum(np.concatenate([np.ones(h), hold[:N], np.ones(h)]))])
    gain = (c[2 * h + 1:] - c[:-2 * h - 1]) / (2 * h + 1)
    return np.clip(x * gain[:, None], -CEILING, CEILING)


def master():
    mix = dry + reverb(send) * 0.32
    fade = np.ones(N)
    k = int(0.5 * SR)
    fade[-k:] = np.cos(np.linspace(0, np.pi / 2, k)) ** 2
    mix *= fade[:, None]
    g = 1.0
    for _ in range(4):
        g *= 10 ** ((TARGET_LUFS - lufs(limit(mix * g))) / 20)
    return limit(mix * g)


def write(x):
    OUT.parent.mkdir(parents=True, exist_ok=True)
    pcm = np.round(x * 8388607).astype("<i4").view(np.uint8).reshape(-1, 4)[:, :3]
    with wave.open(str(OUT), "wb") as w:
        w.setnchannels(2)
        w.setsampwidth(3)
        w.setframerate(SR)
        w.writeframes(pcm.tobytes())


def swish(sec, lo, hi, level):
    n = int(sec * SR)
    t = np.arange(n) / SR
    out = np.zeros((n, 2))
    for c in range(2):
        noise = rng.standard_normal(n)
        spec = np.fft.rfft(noise)
        f = np.fft.rfftfreq(n, 1 / SR)
        centre = np.sqrt(lo * hi)
        spec *= np.exp(-((np.log(f + 1) - np.log(centre)) ** 2) / 0.9)
        out[:, c] = np.fft.irfft(spec, n)
    out /= np.abs(out).max()
    env = np.sin(np.pi * np.clip(t / sec, 0, 1)) ** 3
    return tail(out * env[:, None] * level)


def riser(sec):
    n = int(sec * SR)
    t = np.arange(n) / SR
    s = band_noise(n, 1500, 9000) * (t / sec) ** 2.2
    tone = np.sin(2 * np.pi * np.cumsum(220 + 660 * (t / sec) ** 2) / SR) * (t / sec) ** 3
    return tail(attack(s * 0.35 + tone * 0.12, 0.01))


def crash():
    t = times(1.6)
    s = band_noise(len(t), 4000, 14000) * np.exp(-t / 0.45)
    return tail(attack(s * 0.5, 0.001))


def drums(b0, b1, four=True, snaps=True):
    for beat in range(b0, b1):
        if four or beat % 2 == 0:
            put(kick(0.95), beat, 0.95, 0.0, 0.03)
        if snaps and beat % 2 == 1:
            put(snap(), beat, 0.5, 0.05, 0.25)
        put(shaker(), beat + 0.5, 0.26, 0.35, 0.1)
        put(tick(0.7), beat, 0.08, -0.1, 0.1)


def harmony(b0, b1, step=0.5, up=0, gain=0.33):
    for bar in range(b0 // 4, (b1 + 3) // 4):
        root, voices, tones = CHORDS[bar % 4]
        s = bar * 4
        put(pad(voices, 4 * BEAT, 0.2), s, 0.42, rev=0.35)
        for off, dur, interval in ((0, 1.4, 0), (1.5, 0.45, 0), (2, 1.4, 0), (3.5, 0.45, 7)):
            if b0 <= s + off < b1:
                put(bass(root + interval, dur * BEAT), s + off, 0.55, 0.0, 0.02)
        for k in range(int(4 / step)):
            if b0 <= s + k * step < b1:
                m = tones[ARP[k % 8]] + up
                put(pluck(m, 0.8, 0.3), s + k * step, gain * (1.0 if k % 2 == 0 else 0.8),
                    -0.35 if k % 2 else 0.35, 0.28)


def arrange():
    # 0-2 s: hook, kick on the downbeat so frame 0 already moves.
    put(kick(1.0, 0.3), 0, 1.0, 0.0, 0.03)
    drums(0, 4, four=False, snaps=False)
    harmony(0, 4)
    for beat, m, length in HOOK:
        put(mallet(m, length * BEAT + 0.6, 0.5), beat, 0.42, 0.2, 0.4)
    # 2-9 s: the seven apps, full groove.
    drums(4, 18)
    harmony(4, 18)
    # 9-12 s: the suite, lifted; last beat drops out into the riser.
    drums(18, 23)
    harmony(18, 23, step=0.25, up=12, gain=0.26)
    for beat, m, length in TOP:
        put(mallet(m, length * BEAT + 0.6, 0.6), beat, 0.45, 0.3, 0.4)
    for k in range(8):
        put(snap(), 22 + k * 0.25, 0.22 + 0.05 * k, 0.0, 0.2)
    put(riser(1.9), 22.1, 0.9, 0.0, 0.3)
    # 12-15 s: the star.
    put(kick(1.0, 0.5), 24, 1.1, 0.0, 0.05)
    put(crash(), 24, 0.55, 0.0, 0.5)
    put(bass(38, 2.8), 24, 0.6, 0.0, 0.05)
    put(pad([50, 57, 62, 66, 69, 74], 2.6, 0.01, 0.4), 24, 0.85, rev=0.45)
    for m, pan in ((62, -0.3), (66, 0.3), (69, -0.15), (74, 0.15), (78, 0.0)):
        put(mallet(m, 3.0, 1.0), 24, 0.3, pan, 0.45)
    for k, m in enumerate((86, 90, 93, 98)):
        put(pluck(m, 1.6, 0.6), 24.5 + k * 0.5, 0.18, 0.4 if k % 2 else -0.4, 0.5)
    put(kick(0.8, 0.4), 26.0, 0.8, 0.0, 0.05)
    put(pluck(74, 2.6, 1.1), 26.0, 0.3, 0.0, 0.5)
    put(pluck(81, 2.6, 1.1), 26.0, 0.24, 0.0, 0.5)
    # A short swish into each morph (the motion's sound), varied per landing.
    for k, land in enumerate(LANDINGS[:-1]):
        lo, hi = (900, 5000) if k % 2 else (1400, 7000)
        put(swish(0.42, lo, hi, 1.0), land - 0.72, 0.16, 0.5 if k % 2 else -0.5, 0.2)


def main():
    arrange()
    out = master()
    write(out)
    print(f"{OUT}: {N / SR:.3f} s, peak {20 * np.log10(np.abs(out).max()):.2f} dBFS, "
          f"{lufs(out):.2f} LUFS")


if __name__ == "__main__":
    main()
