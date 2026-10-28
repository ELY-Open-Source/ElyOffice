# Usage: .work/video015/venv/Scripts/python.exe site/video/update-0.1.5/audio/compose.py
"""Music bed (96 BPM, 24 bars) and UI sounds."""
import wave
from pathlib import Path

import numpy as np
import pedalboard as pb
import pyloudnorm as pyln
from scipy import signal
from scipy.ndimage import minimum_filter1d, uniform_filter1d

SR = 48000
BEAT = 30000
BAR = 4 * BEAT
N = 24 * BAR
TAIL = 8 * SR
OUT = Path(__file__).resolve().parent.parent / "public" / "audio"
TARGET_LUFS = -16.0
CEILING_DB = -1.4
LIFTS = (3, 7, 11, 17, 22)
BREAKS = (6, 10, 16, 21)
GAINS = {"piano": 0.86, "pad": 0.245, "strings": 0.13, "arp": 0.14, "bell": 0.12,
         "bass": 0.16, "kick": 0.52, "snare": 0.64, "hats": 0.17, "fx": 0.37}
rng = np.random.default_rng(105)

# chord: bass root, pad voicing, arp tones, piano left hand
CHORDS = {
    "Bb": (34, [50, 57, 60, 62, 65], [62, 65, 69, 74], [46, 53, 62]),
    "Csus": (36, [55, 60, 65, 67, 72], [65, 67, 72, 74], [48, 55, 65]),
    "C": (36, [55, 60, 62, 64, 69], [64, 67, 72, 74], [48, 55, 64]),
    "F": (41, [53, 60, 64, 67, 69], [65, 69, 72, 76], [41, 48, 57]),
    "Dm": (38, [50, 57, 60, 65, 69], [62, 65, 69, 72], [50, 57, 65]),
    "Gm": (31, [53, 57, 58, 62, 65], [62, 65, 70, 74], [43, 50, 58]),
    "Am": (33, [52, 55, 60, 64, 67], [64, 67, 69, 72], [45, 52, 60]),
    "Fend": (29, [53, 60, 65, 67, 69, 72], [], [41, 48, 57, 65]),
}
CHANGES = [(0, "Bb"), (4, "Csus"), (8, "F"), (12, "Dm"), (16, "Bb"), (20, "C"),
           (24, "F"), (28, "Dm"), (32, "Bb"), (36, "C"), (40, "F"), (44, "Dm"),
           (48, "Bb"), (52, "C"), (56, "Gm"), (60, "Csus"), (62, "C"), (64, "Bb"),
           (68, "C"), (72, "Am"), (76, "Dm"), (80, "Gm"), (82, "Csus"), (84, "Fend")]

MEL_A = [(0, 72, 1.5, .62), (1.5, 77, .5, .5), (2, 76, 1, .55), (3, 72, 1, .46),
         (4, 74, 1.5, .6), (5.5, 69, .5, .46), (6, 72, 2, .55)]
MEL_A1 = MEL_A + [(8, 74, 1.5, .6), (9.5, 77, .5, .5), (10, 76, 1, .55), (11, 72, 1, .46),
                  (12, 67, 1.5, .55), (13.5, 72, .5, .46), (14, 74, 1, .5), (15, 76, 1, .48)]
MEL_A2 = MEL_A + [(8, 74, 1.5, .62), (9.5, 77, .5, .52), (10, 81, 1, .6), (11, 79, 1, .52),
                  (12, 76, 1.5, .58), (13.5, 74, .5, .48), (14, 72, 2, .55)]
MEL_B = [(0, 77, 1, .66), (1, 76, .5, .5), (1.5, 72, 1, .55), (2.5, 69, .5, .48),
         (3, 72, 1, .52), (4, 74, 1, .62), (5, 72, .5, .5), (5.5, 69, 1, .52),
         (6.5, 65, .5, .45), (7, 69, 1, .5), (8, 74, 1, .62), (9, 77, .5, .52),
         (9.5, 81, 1.5, .62), (11, 79, 1, .52), (12, 76, 1.5, .6), (13.5, 74, .5, .5),
         (14, 72, 1, .52), (15, 74, 1, .5), (16, 77, 1.5, .62), (17.5, 74, .5, .5),
         (18, 70, 1, .52), (19, 72, 1, .5), (20, 74, 2, .58), (22, 76, 1, .6), (23, 79, 1, .66)]
MEL_D = [(0, 81, 1.5, .72), (1.5, 79, .5, .56), (2, 77, 1, .62), (3, 74, 1, .56),
         (4, 76, 1.5, .68), (5.5, 72, .5, .52), (6, 79, 2, .64), (8, 76, 1.5, .68),
         (9.5, 72, .5, .52), (10, 74, 1, .58), (11, 76, 1, .56), (12, 77, 1.5, .68),
         (13.5, 76, .5, .54), (14, 74, 1, .58), (15, 72, 1, .54), (16, 74, 1.5, .64),
         (17.5, 77, .5, .56), (18, 76, 2, .62)]
INTRO = [(0, 65, .3), (1, 69, .33), (2, 72, .36), (3, 74, .4),
         (4, 67, .42), (5, 72, .46), (6, 74, .5), (7, 77, .54)]
BELLS = [(26.5, 81), (30.5, 79), (34.5, 81), (66.5, 84), (70.5, 84), (74.5, 81), (78.5, 81)]


def hz(m):
    return 440.0 * 2 ** ((m - 69) / 12)


def sample(beat):
    return int(round(beat * BEAT))


def secs(n):
    return np.arange(n) / SR


def filt(x, kind, f, order=2):
    return signal.sosfilt(signal.butter(order, f, kind, fs=SR, output="sos"), x, axis=0)


def peaking(x, f0, gain_db, q):
    a, w = 10 ** (gain_db / 40), 2 * np.pi * f0 / SR
    alpha = np.sin(w) / (2 * q)
    b = [1 + alpha * a, -2 * np.cos(w), 1 - alpha * a]
    d = [1 + alpha / a, -2 * np.cos(w), 1 - alpha / a]
    return signal.lfilter(np.array(b) / d[0], np.array(d) / d[0], x, axis=0)


def sweep(x, cutoff, kind, order=2, block=512):
    """Filter whose corner follows cutoff(seconds); state carries across blocks."""
    y, zi = np.empty_like(x), None
    for s in range(0, len(x), block):
        e = min(s + block, len(x))
        sos = signal.butter(order, cutoff((s + e) / 2 / SR), kind, fs=SR, output="sos")
        if zi is None:
            zi = np.zeros((sos.shape[0], 2) + x.shape[1:])
        y[s:e], zi = signal.sosfilt(sos, x[s:e], axis=0, zi=zi)
    return y


def curve(points, log=False):
    ts, vs = np.array(points).T
    vs = np.log(vs) if log else vs
    return lambda t: np.exp(np.interp(t, ts, vs)) if log else np.interp(t, ts, vs)


def edges(x, a=0.003, r=0.02):
    x = x.copy()
    ai, ri = max(2, int(a * SR)), max(2, int(r * SR))
    up, down = np.linspace(0, 1, ai), np.linspace(1, 0, ri)
    if x.ndim == 2:
        up, down = up[:, None], down[:, None]
    x[:ai] *= up
    x[-ri:] *= down
    return x


def place(x, pan):
    th = (pan + 1) * np.pi / 4
    return np.stack([x * np.cos(th), x * np.sin(th)], 1) * np.sqrt(2)


def noise(n, ch=1):
    return rng.standard_normal((n, ch) if ch > 1 else n)


class Mix:
    def __init__(self):
        self.stems, self.sends = {}, {k: self.blank() for k in ("hall", "room", "echo")}

    @staticmethod
    def blank():
        return np.zeros((N + TAIL, 2))

    def add(self, stem, sig, beat, gain=1.0, pan=0.0, **sends):
        sig = place(sig, pan) if sig.ndim == 1 else sig
        i = sample(beat)
        bus = self.stems.setdefault(stem, self.blank())
        n = min(len(sig), len(bus) - i)
        bus[i:i + n] += sig[:n] * gain
        for name, amount in sends.items():
            self.sends[name][i:i + n] += sig[:n] * gain * amount


def piano(m, dur, vel):
    f0 = hz(m)
    n = int((dur + 0.5) * SR)
    t = secs(n)
    tau = np.clip(3.2 * (262 / f0) ** 0.7, 0.6, 7.0)
    bright = 800 + 2400 * vel
    y = np.zeros(n)
    for k in range(1, 16):
        fk = k * f0 * np.sqrt(1 + 0.0004 * k * k)
        if fk > 7000:
            break
        amp = k ** -1.2 / (1 + (fk / bright) ** 3)
        tk = tau / (1 + 0.45 * (k - 1))
        env = 0.6 * np.exp(-t / (0.3 * tk)) + 0.4 * np.exp(-t / tk)
        for cents in (-0.9, 0.2, 1.1):
            y += amp * env * np.sin(2 * np.pi * fk * 2 ** (cents / 1200) * t + rng.uniform(0, 6.28))
    off = int(dur * SR)
    y[off:] *= np.exp(-secs(n - off) / 0.14)
    k = int(0.012 * SR)
    y[:k] += filt(noise(k), "low", 900) * np.exp(-secs(k) / 0.003) * 0.12
    return edges(y, 0.006, 0.02) * vel ** 1.6 / 3


def saw(f, n, phase):
    dt = f / SR
    p = (phase + dt * np.arange(n)) % 1.0
    y = 2 * p - 1
    lo = p < dt
    x = p[lo] / dt
    y[lo] -= 2 * x - x * x - 1
    hi = p > 1 - dt
    x = (p[hi] - 1) / dt
    y[hi] -= x * x + 2 * x + 1
    return y


def pad(notes, dur, attack, release, octave=0, bright=1.0):
    n = int((dur + release) * SR)
    out = np.zeros((n, 2))
    for m in notes:
        f = hz(m + octave)
        for cents, side in ((-11, 0), (-4, 1), (4, 0), (11, 1)):
            v = saw(f * 2 ** (cents / 1200), n, rng.uniform())
            out[:, side] += 0.75 * v * bright
            out[:, 1 - side] += 0.25 * v * bright
        out += np.sin(2 * np.pi * f * secs(n))[:, None] * 0.6
    t = secs(n)
    env = np.minimum(1, t / attack) ** 2
    rel = np.clip((t - dur) / release, 0, 1)
    env *= np.cos(rel * np.pi / 2) ** 2
    return out * env[:, None] / len(notes)


def pluck(m, vel):
    f0, n = hz(m), int(0.9 * SR)
    t = secs(n)
    y = np.zeros(n)
    for k in range(1, 25):
        if k * f0 > 7500:
            break
        y += k ** -1.3 * np.exp(-t / (0.5 / (1 + 0.7 * (k - 1)))) * np.sin(2 * np.pi * k * f0 * t + rng.uniform(0, 6.28))
    return edges(y, 0.0015, 0.05) * vel


def bell(m, vel, dur=3.0):
    f, n = hz(m), int(dur * SR)
    t = secs(n)
    index = 1.6 * np.exp(-t / 0.35)
    y = np.sin(2 * np.pi * f * t + index * np.sin(2 * np.pi * 3.5 * f * t)) * np.exp(-t / 1.1)
    y += 0.3 * np.sin(2 * np.pi * 2 * f * t) * np.exp(-t / 0.5)
    return edges(y, 0.002, 0.1) * vel


def bass(m, dur, vel, decay=None):
    f, n = hz(m), int((dur + 0.08) * SR)
    t = secs(n)
    y = np.sin(2 * np.pi * f * t) + 0.22 * np.sin(4 * np.pi * f * t) + 0.07 * np.sin(6 * np.pi * f * t)
    y = np.tanh(1.4 * y) / np.tanh(1.4)
    env = np.minimum(1, t / 0.008) * np.clip((dur + 0.06 - t) / 0.06, 0, 1)
    if decay:
        env *= np.exp(-t / decay)
    return y * env * vel


def kick(vel):
    n = int(0.5 * SR)
    t = secs(n)
    f = 47 + 75 * np.exp(-t / 0.035)
    y = np.tanh(1.5 * np.sin(2 * np.pi * np.cumsum(f) / SR) * np.exp(-t / 0.26))
    k = int(0.004 * SR)
    y[:k] += filt(noise(k), "low", 2200) * 0.2
    return edges(y, 0.001, 0.04) * vel


def snare(vel):
    n = int(0.35 * SR)
    t = secs(n)
    env = 0.55 * np.exp(-t / 0.13)
    for d in (0.0, 0.009, 0.019):
        env += np.where(t >= d, np.exp(-(t - d) / 0.006), 0) * 0.6
    y = filt(noise(n), "band", [1200, 7500]) * env * np.minimum(1, t / 0.002)
    y += np.sin(2 * np.pi * 185 * t) * np.exp(-t / 0.045) * 0.35
    return edges(peaking(y, 2600, -5, 1.0), 0.001, 0.03) * vel


def hat(vel, open_=False):
    n = int((0.3 if open_ else 0.08) * SR)
    t = secs(n)
    y = filt(noise(n), "band", [7000, 12500], 3) * np.exp(-t / (0.2 if open_ else 0.03))
    return edges(y, 0.001, 0.02) * vel


def shaker(vel):
    n = int(0.09 * SR)
    t = secs(n)
    y = filt(noise(n), "band", [4800, 10000]) * np.minimum(1, t / 0.007) * np.exp(-t / 0.035)
    return edges(y, 0.001, 0.02) * vel


def riser(strength, notes):
    n = int(1.25 * SR)
    t = secs(n)
    u = t / t[-1]
    swish = sweep(noise(n, 2), lambda s: [r := 300 * (16 ** ((s / 1.25) ** 1.4)), min(r * 2.4, 9000)], "band", 1)
    y = filt(swish, "low", 8000) * (u ** 2.4)[:, None] * 0.9
    swell = sum(np.sin(2 * np.pi * hz(m + 12) * t) for m in notes) / len(notes)
    y += place(swell * np.exp(5 * (u - 1)) * u * 0.6, 0)
    y[-int(0.02 * SR):] *= np.linspace(1, 0, int(0.02 * SR))[:, None]
    return y * strength


def impact(strength, tail=2.4):
    n = int(tail * SR)
    t = secs(n)
    f = 38 + 32 * np.exp(-t / 0.12)
    boom = np.tanh(1.3 * np.sin(2 * np.pi * np.cumsum(f) / SR) * np.exp(-t / 0.65))
    thump = filt(noise(n), "low", 220) * np.exp(-t / 0.05) * 0.8
    air = filt(noise(n, 2), "band", [250, 2200]) * np.exp(-t / 0.45)[:, None] * 0.07
    return edges(place(boom + thump, 0) + air, 0.001, 0.2) * strength


def impulse(seconds, t60, seed):
    """Stereo reverb impulse: decorrelated noise, per-band decay."""
    g = np.random.default_rng(seed)
    n = int(seconds * SR)
    t = secs(n)
    ir = np.zeros((n, 2))
    for ch in range(2):
        w = g.standard_normal(n)
        lo, hi = filt(w, "low", 500), filt(w, "high", 4000)
        bands = (lo, w - lo - hi, hi)
        ir[:, ch] = sum(b * 10 ** (-3 * t / T) for b, T in zip(bands, t60))
    ir = filt(ir, "low", 7000) * np.minimum(1, t / 0.02)[:, None]
    ir = np.concatenate([np.zeros((int(0.022 * SR), 2)), edges(ir, 0.001, seconds * 0.15)])
    return ir / np.sqrt((ir ** 2).sum(0))


def reverb(x, ir):
    return np.stack([signal.fftconvolve(x[:, c], ir[:, c])[:len(x)] for c in range(2)], 1)


def pingpong(x, beats=0.75, fb=0.4, taps=7):
    d, y, out = sample(beats), x.mean(1), np.zeros_like(x)
    for k in range(1, taps + 1):
        y = filt(filt(y, "low", 3200, 1), "high", 250, 1)
        out[k * d:, k % 2] += y[:len(x) - k * d] * fb ** (k - 1)
    return out


def chord_at(beat):
    return CHORDS[[c for b, c in CHANGES if b <= beat][-1]]


def drops(bar, beat):
    return bar in BREAKS and beat >= 2


def score(mix):
    for i, (b, name) in enumerate(CHANGES):
        end = CHANGES[i + 1][0] if i + 1 < len(CHANGES) else 96
        notes = CHORDS[name][1]
        first, final = b == 0, name == "Fend"
        sig = pad(notes, (end - b) * 0.625, 2.6 if first else 0.25, 3.0 if final else 0.9)
        mix.add("pad", sig, b, hall=0.5)
        if 40 <= b < 84:
            mix.add("strings", pad(notes[-3:], (end - b) * 0.625, 0.4, 0.9, 12, 0.5), b, hall=0.6)

    for x, m, v in INTRO:
        mix.add("piano", piano(m, 2.4, v), x, pan=(m - 70) / 30, hall=0.35)
    for x, name in ((0, "Bb"), (4, "Csus")):
        for m in CHORDS[name][3]:
            mix.add("piano", piano(m, 3.8, 0.3), x, pan=-0.2, hall=0.3)

    for start, line, low in ((8, MEL_A1, 0), (24, MEL_A2, 0), (40, MEL_B, 0), (64, MEL_D, 1)):
        for x, m, d, v in line:
            mix.add("piano", piano(m, d * 0.625 + 0.15, v), start + x, pan=0.08, hall=0.28)
            if low:
                mix.add("piano", piano(m - 12, d * 0.625 + 0.15, v * 0.55), start + x, pan=-0.1, hall=0.28)
    for x, m, d, v in ((84, 77, 4.5, .66), (88, 84, 1.5, .32), (89.5, 81, .5, .27), (90, 77, 3.0, .26)):
        mix.add("piano", piano(m, d * 0.625, v), x, pan=0.1, hall=0.45)
    for m in CHORDS["Fend"][3]:
        mix.add("piano", piano(m, 4.0, 0.38), 84, pan=-0.15, hall=0.45)
    for x, m in BELLS + [(84, 89)]:
        mix.add("bell", bell(m, 0.5 if x < 84 else 0.35), x, pan=0.35, hall=0.7, echo=0.35)

    kicks = []
    for bar in range(3, 22):
        base = (bar - 1) * 4
        section = 0 if bar < 7 else 1 if bar < 11 else 2 if bar < 17 else 3
        lh_vel = (0.3, 0.33, 0.37, 0.4)[section]
        for x in ((0,) if section < 2 else (0, 2)):
            for m in chord_at(base + x)[3]:
                mix.add("piano", piano(m, 1.8, lh_vel), base + x, pan=-0.2, hall=0.3)

        pattern = ([0, 2], [0, 2, 2.5], [0, 0.75, 2, 2.5], [0, 1, 2, 3])[section]
        for x in pattern:
            if not drops(bar, x):
                kicks.append(base + x)
                mix.add("kick", kick(0.9 if x in (0, 2) else 0.7), base + x, room=0.05)
        for x in (1, 3):
            if not drops(bar, x):
                mix.add("snare", snare(0.55 + 0.1 * (section > 1)), base + x, pan=0.05, room=0.35, hall=0.12)
        if section > 1:
            for x in ((3.75,) if section == 2 else (1.75, 3.75)):
                if not drops(bar, x):
                    mix.add("snare", snare(0.14), base + x, pan=0.1, room=0.3)
        if bar in (16, 21):
            for j in range(8):
                mix.add("snare", snare(0.12 + 0.05 * j), base + 2 + j * 0.25, pan=0.1, room=0.3, hall=0.1)
        for j in range(8):
            x = j * 0.5
            if drops(bar, x) and bar in (6, 10):
                continue
            jitter = rng.normal(0, 0.004)
            if section == 3 and j % 2:
                mix.add("hats", hat(0.32, True), base + x + jitter, pan=0.3, room=0.15)
            elif section == 2 and j == 7:
                mix.add("hats", hat(0.3, True), base + x + jitter, pan=0.3, room=0.15)
            else:
                mix.add("hats", hat(0.5 if j % 2 else 0.28), base + x + jitter, pan=0.3, room=0.1)
        if section > 0:
            for j in range(16):
                vel = (0.5, 0.22, 0.34, 0.22)[j % 4] * (1 + 0.3 * (section == 3))
                mix.add("hats", shaker(vel), base + j * 0.25 + rng.normal(0, 0.004), pan=-0.35, room=0.1)

        step = 0.5 if section < 2 else 0.25
        for j in range(int(4 / step)):
            x = j * step
            tones = chord_at(base + x)[2]
            idx = ([0, 1, 2, 3, 2, 1, 2, 3] if section < 2 else [0, 1, 2, 3, 1, 2, 3, 2])[j % 8]
            m = tones[idx] + (12 if section == 3 and j % 8 in (3, 6) else 0)
            vel = (0.7 if j % 2 == 0 else 0.5) * (0.8 if step < 0.5 else 1.0)
            mix.add("arp", pluck(m, vel), base + x, pan=0.4 * np.sin(j * 0.9), hall=0.2, echo=0.32)

        rhythm = ([(0, 1.8, 0), (2, 1.3, 0), (3.5, 0.4, 0)],
                  [(0, 1.3, 0), (1.5, 0.4, 0), (2, 1.3, 0), (3.5, 0.4, 12)],
                  [(0, .9, 0), (1, .4, 0), (1.5, .4, 12), (2, .9, 0), (3, .4, 0), (3.5, .4, 12)],
                  [(0, .9, 0), (1, .4, 0), (1.5, .4, 12), (2, .9, 0), (3, .4, 0), (3.5, .4, 12)])[section]
        if bar in BREAKS:
            rhythm = [r for r in rhythm if r[0] < 2] + [(2, 1.9, 0)]
        for x, d, octave in rhythm:
            mix.add("bass", bass(chord_at(base + x)[0] + octave, d * 0.625, 0.8 if octave else 1.0), base + x)

    kicks.append(84)
    mix.add("kick", kick(1.0), 84, room=0.05)
    mix.add("bass", bass(29, 5.0, 1.0, decay=1.8), 84)
    strengths = {3: (0.6, 0.6), 7: (0.55, 0.45), 11: (0.75, 0.7), 17: (1.0, 1.0), 22: (0.9, 1.0)}
    for bar in LIFTS:
        beat = (bar - 1) * 4
        rise, hit = strengths[bar]
        mix.add("fx", riser(rise, chord_at(beat)[1]), beat - 2, hall=0.3)
        mix.add("fx", impact(hit, 4.0 if bar == 22 else 2.4), beat, hall=0.45)
    return np.array(kicks)


def duck(kicks, depth):
    g = np.ones(N + TAIL)
    shape = np.minimum(1, secs(int(0.5 * SR)) / 0.004) * np.exp(-secs(int(0.5 * SR)) / 0.13)
    for k in kicks:
        i = sample(k)
        n = min(len(shape), len(g) - i)
        g[i:i + n] = np.minimum(g[i:i + n], 1 - depth * shape[:n])
    return g[:, None]


def run_board(board, x):
    return board(x.T.astype(np.float32), SR).T.astype(np.float64)


def true_peak(x):
    return np.abs(signal.resample_poly(x, 4, 1, axis=0)).max()


def limit(x, ceiling_db):
    """Look-ahead limiter on 4x-oversampled peaks, release 120 ms."""
    c = 10 ** (ceiling_db / 20)
    peak = np.abs(signal.resample_poly(x, 4, 1, axis=0)).max(1)[:len(x) * 4].reshape(-1, 4).max(1)
    need = np.minimum(1, c / np.maximum(peak, 1e-9))
    look = 96
    red = 1 - minimum_filter1d(need, 2 * look + 1)
    r, a = np.empty_like(red), np.exp(-1 / (0.12 * SR))
    acc = 0.0
    for i, v in enumerate(red):
        acc = v if v > acc else acc * a
        r[i] = acc
    return x * uniform_filter1d(1 - r, look)[:, None]


def master(mix, kicks):
    s = mix.stems
    gains = GAINS
    pad_lp = curve([(0, 220), (4.9, 1700), (5, 1500), (14.9, 1700), (15, 1900), (24.9, 2200),
                    (25, 2300), (39.9, 2700), (40, 3000), (52.4, 3100), (52.5, 2600), (59, 450), (70, 450)], True)
    pad_bus = sweep(s["pad"] + s["strings"] * gains["strings"] / gains["pad"], pad_lp, "low", 4)
    pad_bus = run_board(pb.Pedalboard([pb.Chorus(rate_hz=0.22, depth=0.3, centre_delay_ms=9, mix=0.45)]), pad_bus)
    piano = peaking(filt(s["piano"], "high", 90), 2800, -2.5, 0.9)
    dry = (piano * gains["piano"] + pad_bus * gains["pad"] * duck(kicks, 0.4)
           + s["arp"] * gains["arp"] * duck(kicks, 0.3) + s["bell"] * gains["bell"]
           + s["bass"] * gains["bass"] * duck(kicks, 0.55) + s["kick"] * gains["kick"]
           + s["snare"] * gains["snare"] + s["hats"] * gains["hats"] + s["fx"] * gains["fx"])
    sends = mix.sends
    echo = pingpong(sends["echo"] * 0.2)
    hall = reverb((sends["hall"] + echo * 0.5) * 0.4, impulse(3.6, (3.4, 2.6, 1.3), 7))
    room = reverb(sends["room"] * 0.25, impulse(0.9, (0.6, 0.5, 0.3), 11))
    wet = filt(hall, "high", 160) * duck(kicks, 0.25) + filt(room, "high", 200) + echo * 0.6
    ride = curve([(0, -10), (4.97, -4.5), (5.0, -3.2), (14.97, -3.2), (15, -2.2), (24.97, -2.2),
                  (25, -1.1), (39.97, -1.1), (40, 0.8), (52.47, 0.8), (52.5, 0.4), (70, 0.4)])
    x = filt(dry + wet, "high", 28) * 10 ** (ride(secs(N + TAIL)) / 20)[:, None]
    x = peaking(x, 2600, -2.0, 0.8)
    x = x + 0.25 * filt(x, "high", 8000)
    x = run_board(pb.Pedalboard([pb.Compressor(threshold_db=-20, ratio=1.8, attack_ms=25, release_ms=250)]), x)
    x = x[:N]
    t = secs(N)
    x *= np.clip(np.cos(np.clip((t - 57.0) / 2.3, 0, 1) * np.pi / 2), 0, 1)[:, None] ** 2
    x = edges(x, 0.002, 0.01)
    meter = pyln.Meter(SR)
    for _ in range(3):
        x *= 10 ** ((TARGET_LUFS - meter.integrated_loudness(x)) / 20)
        x = np.tanh(x * 1.1) / 1.1
        x = limit(x, CEILING_DB)
    stems = {k: 20 * np.log10(np.sqrt((v[:N] ** 2).mean()) * gains.get(k, 1) + 1e-12) for k, v in s.items()}
    return x, meter.integrated_loudness(x), stems


def write(path, x):
    x = np.atleast_2d(x.T).T
    q = np.round(np.clip(x, -1, 1 - 2 ** -23) * 2 ** 23).astype("<i4")
    path.parent.mkdir(parents=True, exist_ok=True)
    with wave.open(str(path), "wb") as w:
        w.setnchannels(x.shape[1])
        w.setsampwidth(3)
        w.setframerate(SR)
        w.writeframes(q.view(np.uint8).reshape(-1, 4)[:, :3].tobytes())
